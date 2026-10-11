use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use tokio::{io::AsyncReadExt, process::Command};

use crate::{
    infrastructure::audio::FfmpegLocator,
    models::{AppError, AppErrorCode, MediaKind, MediaSourceInfo},
    services::audio::{ProcessControl, ProcessControlState},
};

const CHUNK_DURATION_SECONDS: f64 = 30.0 * 60.0;
const SILENCE_SEARCH_WINDOW_SECONDS: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkRange {
    pub start_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioPart {
    pub path: PathBuf,
    pub offset_ms: u64,
}

#[derive(Debug)]
pub struct ManagedAudioParts {
    workspace: PathBuf,
    parts: Vec<AudioPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegPreprocessRequest {
    pub task_id: String,
    pub run_revision: u64,
    pub source_path: PathBuf,
    pub duration_seconds: u64,
    pub temporary_directory: PathBuf,
    pub silence_points: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FfmpegRunOutput {
    Completed { stderr: String },
    Cancelled,
}

#[async_trait]
pub(crate) trait TranscriptionFfmpegRunner: Send + Sync {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<FfmpegRunOutput, AppError>;
}

pub struct FfmpegTranscriptionPreprocessor {
    executable: PathBuf,
    runner: Arc<dyn TranscriptionFfmpegRunner>,
}

impl FfmpegTranscriptionPreprocessor {
    pub fn new(configured_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            executable: FfmpegLocator::resolve(configured_path)?,
            runner: Arc::new(TokioTranscriptionFfmpegRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(executable: PathBuf, runner: Arc<dyn TranscriptionFfmpegRunner>) -> Self {
        Self { executable, runner }
    }

    pub async fn inspect(
        &self,
        source_path: &Path,
        control: ProcessControl,
    ) -> Result<MediaSourceInfo, AppError> {
        let media_kind = validate_source(source_path)?;
        let arguments = vec![
            "-nostdin".into(),
            "-hide_banner".into(),
            "-i".into(),
            source_path.as_os_str().to_owned(),
            "-map".into(),
            "0:a:0".into(),
            "-t".into(),
            "0.100".into(),
            "-f".into(),
            "null".into(),
            "-".into(),
        ];
        let output = self
            .runner
            .run(&self.executable, arguments, control)
            .await?;
        let stderr = match output {
            FfmpegRunOutput::Completed { stderr } => stderr,
            FfmpegRunOutput::Cancelled => {
                return Err(preprocess_error(
                    "SOURCE_INSPECTION_CANCELLED",
                    "Source inspection was cancelled",
                ));
            }
        };
        let duration_seconds = parse_duration_seconds(&stderr).ok_or_else(|| {
            preprocess_error(
                "SOURCE_DURATION_UNAVAILABLE",
                "The media duration could not be determined",
            )
        })?;
        let metadata = fs::metadata(source_path).map_err(|_| {
            preprocess_error("SOURCE_UNAVAILABLE", "The media source is unavailable")
        })?;
        let file_name = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                preprocess_error("SOURCE_NAME_INVALID", "The media source name is invalid")
            })?;
        let format = source_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        Ok(MediaSourceInfo {
            file_name: file_name.into(),
            path: source_path.to_string_lossy().into_owned(),
            duration_seconds,
            size_bytes: metadata.len(),
            media_kind,
            format,
        })
    }

    pub async fn preprocess(
        &self,
        request: FfmpegPreprocessRequest,
        control: ProcessControl,
    ) -> Result<ManagedAudioParts, AppError> {
        validate_source(&request.source_path)?;
        let workspace = request
            .temporary_directory
            .join(format!("{}-{}", request.task_id, request.run_revision));
        if workspace.exists() {
            fs::remove_dir_all(&workspace).map_err(|_| {
                preprocess_error(
                    "TEMPORARY_CLEANUP_FAILED",
                    "The previous transcription workspace could not be removed",
                )
            })?;
        }
        fs::create_dir_all(&workspace).map_err(|_| {
            preprocess_error(
                "TEMPORARY_DIRECTORY_UNAVAILABLE",
                "The transcription workspace could not be created",
            )
        })?;

        let result = self
            .preprocess_into_workspace(&request, &workspace, control)
            .await;
        match result {
            Ok(parts) => Ok(ManagedAudioParts::new(&workspace, parts)),
            Err(error) => {
                let _ = fs::remove_dir_all(&workspace);
                Err(error)
            }
        }
    }

    async fn preprocess_into_workspace(
        &self,
        request: &FfmpegPreprocessRequest,
        workspace: &Path,
        control: ProcessControl,
    ) -> Result<Vec<(PathBuf, u64)>, AppError> {
        let ranges = choose_chunk_ranges(
            request.duration_seconds as f64,
            request.silence_points.as_slice(),
        );
        if ranges.is_empty() {
            return Err(preprocess_error(
                "SOURCE_DURATION_INVALID",
                "The media duration is invalid",
            ));
        }

        let mut parts = Vec::with_capacity(ranges.len());
        for (index, range) in ranges.into_iter().enumerate() {
            if control.state() == ProcessControlState::CancelRequested {
                return Err(preprocess_error(
                    "PREPROCESSING_CANCELLED",
                    "Audio preprocessing was cancelled",
                ));
            }
            let output_path = workspace.join(format!("part-{index:04}.wav"));
            let arguments = build_preprocess_args(
                &request.source_path,
                &output_path,
                range.start_ms,
                range.duration_ms,
            );
            match self
                .runner
                .run(&self.executable, arguments, control.clone())
                .await?
            {
                FfmpegRunOutput::Completed { .. } => {}
                FfmpegRunOutput::Cancelled => {
                    return Err(preprocess_error(
                        "PREPROCESSING_CANCELLED",
                        "Audio preprocessing was cancelled",
                    ));
                }
            }
            if !is_non_empty_file(&output_path) {
                return Err(preprocess_error(
                    "PREPROCESSING_OUTPUT_INVALID",
                    "FFmpeg did not produce a usable audio part",
                ));
            }
            parts.push((output_path, range.start_ms));
        }
        Ok(parts)
    }
}

struct TokioTranscriptionFfmpegRunner;

#[async_trait]
impl TranscriptionFfmpegRunner for TokioTranscriptionFfmpegRunner {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<FfmpegRunOutput, AppError> {
        let mut command = Command::new(executable);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        let mut child = command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                preprocess_error(
                    "FFMPEG_UNAVAILABLE",
                    "The bundled media processor could not be started",
                )
            })?;
        let mut stderr = child.stderr.take().ok_or_else(|| {
            preprocess_error("FFMPEG_PIPE_FAILED", "FFmpeg diagnostics are unavailable")
        })?;
        let stderr_reader = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).await.map(|_| bytes)
        });

        let status = loop {
            if control.state() == ProcessControlState::CancelRequested {
                child.kill().await.map_err(|_| {
                    preprocess_error("FFMPEG_CANCEL_FAILED", "FFmpeg could not be stopped")
                })?;
                let _ = child.wait().await;
                stderr_reader.abort();
                return Ok(FfmpegRunOutput::Cancelled);
            }
            match tokio::time::timeout(Duration::from_millis(50), child.wait()).await {
                Ok(result) => {
                    break result.map_err(|_| {
                        preprocess_error("FFMPEG_EXITED", "FFmpeg did not exit cleanly")
                    })?
                }
                Err(_) => continue,
            }
        };
        let stderr = stderr_reader
            .await
            .map_err(|_| {
                preprocess_error("FFMPEG_PIPE_FAILED", "FFmpeg diagnostics are unavailable")
            })?
            .map_err(|_| {
                preprocess_error("FFMPEG_PIPE_FAILED", "FFmpeg diagnostics are unavailable")
            })?;
        let diagnostics = String::from_utf8_lossy(&stderr[..stderr.len().min(64 * 1024)]).into();
        if !status.success() {
            return Err(preprocess_error(
                "SOURCE_DECODE_FAILED",
                "FFmpeg could not decode the media audio track",
            ));
        }
        Ok(FfmpegRunOutput::Completed {
            stderr: diagnostics,
        })
    }
}

impl ManagedAudioParts {
    pub fn new(workspace: impl AsRef<Path>, parts: Vec<(PathBuf, u64)>) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
            parts: parts
                .into_iter()
                .map(|(path, offset_ms)| AudioPart { path, offset_ms })
                .collect(),
        }
    }

    pub fn parts(&self) -> &[AudioPart] {
        &self.parts
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

impl Drop for ManagedAudioParts {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace);
    }
}

pub fn supported_media_kind(path: &Path) -> Option<MediaKind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" => Some(MediaKind::Audio),
        "mp4" | "mkv" | "flv" | "mov" | "webm" => Some(MediaKind::Video),
        _ => None,
    }
}

pub fn choose_chunk_ranges(duration_seconds: f64, silence_points: &[f64]) -> Vec<ChunkRange> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Vec::new();
    }

    let total_ms = seconds_to_millis(duration_seconds);
    let mut ranges = Vec::new();
    let mut start_seconds = 0.0;

    while duration_seconds - start_seconds > CHUNK_DURATION_SECONDS {
        let target = start_seconds + CHUNK_DURATION_SECONDS;
        let boundary = silence_points
            .iter()
            .copied()
            .filter(|point| {
                point.is_finite()
                    && *point > start_seconds
                    && (*point - target).abs() <= SILENCE_SEARCH_WINDOW_SECONDS
            })
            .min_by(|left, right| (left - target).abs().total_cmp(&(right - target).abs()))
            .unwrap_or(target)
            .clamp(start_seconds, duration_seconds);

        let start_ms = seconds_to_millis(start_seconds);
        let boundary_ms = seconds_to_millis(boundary);
        if boundary_ms <= start_ms {
            break;
        }
        ranges.push(ChunkRange {
            start_ms,
            duration_ms: boundary_ms - start_ms,
        });
        start_seconds = boundary;
    }

    let start_ms = seconds_to_millis(start_seconds);
    if start_ms < total_ms {
        ranges.push(ChunkRange {
            start_ms,
            duration_ms: total_ms - start_ms,
        });
    }
    ranges
}

pub fn build_preprocess_args(
    input: &Path,
    output: &Path,
    start_ms: u64,
    duration_ms: u64,
) -> Vec<OsString> {
    vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-ss".into(),
        format_seconds(start_ms).into(),
        "-t".into(),
        format_seconds(duration_ms).into(),
        "-i".into(),
        input.as_os_str().to_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
        "-ac".into(),
        "1".into(),
        "-ar".into(),
        "16000".into(),
        "-c:a".into(),
        "pcm_s16le".into(),
        output.as_os_str().to_owned(),
    ]
}

fn seconds_to_millis(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

fn format_seconds(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

fn validate_source(path: &Path) -> Result<MediaKind, AppError> {
    if !is_non_empty_file(path) {
        return Err(preprocess_error(
            "SOURCE_UNAVAILABLE",
            "The media source is unavailable",
        ));
    }
    supported_media_kind(path).ok_or_else(|| {
        preprocess_error(
            "SOURCE_FORMAT_UNSUPPORTED",
            "The media format is not supported for transcription",
        )
    })
}

fn is_non_empty_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn parse_duration_seconds(stderr: &str) -> Option<u64> {
    let marker = "Duration: ";
    let start = stderr.find(marker)? + marker.len();
    let remainder = stderr.get(start..)?;
    let value = remainder
        .split([',', '\r', '\n'])
        .next()
        .unwrap_or_default()
        .trim();
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let duration = hours * 3600.0 + minutes * 60.0 + seconds;
    (duration.is_finite() && duration > 0.0).then(|| duration.ceil() as u64)
}

fn preprocess_error(details: &str, message: &str) -> AppError {
    let mut error = AppError::new(AppErrorCode::E013, message);
    error.details = Some(details.into());
    error
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        ffi::OsString,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;

    use super::{
        FfmpegPreprocessRequest, FfmpegRunOutput, FfmpegTranscriptionPreprocessor,
        TranscriptionFfmpegRunner,
    };
    use crate::{models::AppError, services::audio::ProcessControl};

    struct FakeRunner {
        outputs: Mutex<VecDeque<Result<FfmpegRunOutput, AppError>>>,
        arguments: Mutex<Vec<Vec<OsString>>>,
    }

    #[async_trait]
    impl TranscriptionFfmpegRunner for FakeRunner {
        async fn run(
            &self,
            _executable: &Path,
            arguments: Vec<OsString>,
            _control: ProcessControl,
        ) -> Result<FfmpegRunOutput, AppError> {
            self.arguments.lock().unwrap().push(arguments.clone());
            if arguments.iter().any(|value| value == "pcm_s16le") {
                let output = PathBuf::from(arguments.last().unwrap());
                std::fs::write(output, b"wav").unwrap();
            }
            self.outputs.lock().unwrap().pop_front().unwrap()
        }
    }

    #[test]
    fn inspect_and_preprocess_use_probe_metadata_and_create_managed_chunks() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("fixture.MP4");
            std::fs::write(&source, b"media").unwrap();
            let runner = Arc::new(FakeRunner {
                outputs: Mutex::new(VecDeque::from([
                    Ok(FfmpegRunOutput::Completed {
                        stderr: "Duration: 00:30:05.000".into(),
                    }),
                    Ok(FfmpegRunOutput::Completed {
                        stderr: String::new(),
                    }),
                    Ok(FfmpegRunOutput::Completed {
                        stderr: String::new(),
                    }),
                ])),
                arguments: Mutex::new(Vec::new()),
            });
            let preprocessor = FfmpegTranscriptionPreprocessor::with_runner(
                temp.path().join("ffmpeg.exe"),
                runner,
            );

            let source_info = preprocessor
                .inspect(&source, ProcessControl::new())
                .await
                .unwrap();
            assert_eq!(source_info.duration_seconds, 1_805);

            let workspace = temp.path().join("work");
            let parts = preprocessor
                .preprocess(
                    FfmpegPreprocessRequest {
                        task_id: "task-1".into(),
                        run_revision: 2,
                        source_path: source,
                        duration_seconds: source_info.duration_seconds,
                        temporary_directory: workspace.clone(),
                        silence_points: Vec::new(),
                    },
                    ProcessControl::new(),
                )
                .await
                .unwrap();

            assert_eq!(parts.parts().len(), 2);
            assert!(parts.parts().iter().all(|part| part.path.is_file()));
            drop(parts);
            assert!(!workspace.join("task-1-2").exists());
        });
    }

    #[test]
    fn inspect_rejects_missing_or_unsupported_sources_before_spawning_ffmpeg() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let runner = Arc::new(FakeRunner {
                outputs: Mutex::new(VecDeque::new()),
                arguments: Mutex::new(Vec::new()),
            });
            let preprocessor = FfmpegTranscriptionPreprocessor::with_runner(
                temp.path().join("ffmpeg.exe"),
                runner.clone(),
            );

            let error = preprocessor
                .inspect(&temp.path().join("missing.mp3"), ProcessControl::new())
                .await
                .unwrap_err();
            assert_eq!(error.code, crate::models::AppErrorCode::E013);
            assert_eq!(error.details.as_deref(), Some("SOURCE_UNAVAILABLE"));
            assert!(runner.arguments.lock().unwrap().is_empty());
        });
    }
}
