use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use tokio::process::Command;

use crate::{
    models::{AppError, AppErrorCode, AudioOutputProfile},
    services::audio::{
        AudioProcessRequest, AudioSourceTier, MediaProcessorPort, ProcessControl,
        ProcessControlState, ProcessOutcome,
    },
};

pub struct FfmpegLocator;

impl FfmpegLocator {
    pub fn resolve(configured_path: &Path) -> Result<PathBuf, AppError> {
        let valid_name = configured_path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("ffmpeg") || value.eq_ignore_ascii_case("ffmpeg.exe")
            });
        if !configured_path.is_absolute() || !valid_name || !configured_path.is_file() {
            return Err(ffmpeg_error(
                "FFMPEG_UNAVAILABLE",
                "The bundled media processor is unavailable",
            ));
        }
        Ok(configured_path.to_path_buf())
    }
}

pub struct FfmpegMediaProcessor {
    executable: PathBuf,
    runner: Arc<dyn FfmpegRunner>,
}

pub struct DeferredFfmpegMediaProcessor {
    configured_path: PathBuf,
}

impl DeferredFfmpegMediaProcessor {
    pub fn new(configured_path: PathBuf) -> Self {
        Self { configured_path }
    }
}

impl FfmpegMediaProcessor {
    pub fn new(configured_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            executable: FfmpegLocator::resolve(configured_path)?,
            runner: Arc::new(TokioFfmpegRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(executable: PathBuf, runner: Arc<dyn FfmpegRunner>) -> Self {
        Self { executable, runner }
    }
}

pub fn build_ffmpeg_args(request: &AudioProcessRequest) -> Result<Vec<OsString>, AppError> {
    if request.title.trim().is_empty() || request.uploader.trim().is_empty() {
        return Err(ffmpeg_error(
            "FFMPEG_METADATA_INVALID",
            "The audio metadata is invalid",
        ));
    }
    if request.output_profile == AudioOutputProfile::FlacLossless
        && !matches!(
            request.source_tier,
            AudioSourceTier::Lossless | AudioSourceTier::HiResLossless
        )
    {
        return Err(ffmpeg_error(
            "FFMPEG_SOURCE_INVALID",
            "The source is not compatible with lossless output",
        ));
    }

    let mut args = vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        request.source_path.as_os_str().to_owned(),
        "-i".into(),
        request.cover_path.as_os_str().to_owned(),
        "-map_metadata".into(),
        "-1".into(),
        "-map".into(),
        "0:a:0".into(),
        "-map".into(),
        "1:v:0".into(),
        "-metadata".into(),
        format!("title={}", request.title).into(),
        "-metadata".into(),
        format!("artist={}", request.uploader).into(),
        "-c:v".into(),
        "copy".into(),
        "-disposition:v:0".into(),
        "attached_pic".into(),
    ];
    match request.output_profile {
        AudioOutputProfile::Mp3 { bitrate_kbps } => {
            if !matches!(bitrate_kbps, 128 | 192 | 320) {
                return Err(ffmpeg_error(
                    "FFMPEG_PROFILE_INVALID",
                    "The audio output profile is invalid",
                ));
            }
            args.extend([
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                format!("{bitrate_kbps}k").into(),
                "-id3v2_version".into(),
                "3".into(),
            ]);
        }
        AudioOutputProfile::M4aOriginal => {
            args.extend([
                "-c:a".into(),
                "copy".into(),
                "-movflags".into(),
                "+faststart".into(),
            ]);
        }
        AudioOutputProfile::FlacLossless => {
            args.extend(["-c:a".into(), "flac".into()]);
        }
    }
    args.push(request.output_path.as_os_str().to_owned());
    Ok(args)
}

pub(crate) fn ffmpeg_error(details: &str, message: &str) -> AppError {
    let mut error = AppError::new(AppErrorCode::E008, message);
    error.details = Some(details.into());
    error
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn remove_processed(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerOutcome {
    Completed,
    Cancelled,
}

#[async_trait]
pub(crate) trait FfmpegRunner: Send + Sync {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<RunnerOutcome, AppError>;
}

pub(crate) struct TokioFfmpegRunner;

#[async_trait]
impl FfmpegRunner for TokioFfmpegRunner {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<RunnerOutcome, AppError> {
        let mut child = Command::new(executable)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                ffmpeg_error(
                    "FFMPEG_UNAVAILABLE",
                    "The bundled media processor could not be started",
                )
            })?;

        let status = loop {
            if control.state() == ProcessControlState::CancelRequested {
                child.kill().await.map_err(|_| {
                    ffmpeg_error(
                        "FFMPEG_CANCEL_FAILED",
                        "The media processor could not be stopped",
                    )
                })?;
                let _ = child.wait().await;
                return Ok(RunnerOutcome::Cancelled);
            }
            match tokio::time::timeout(Duration::from_millis(50), child.wait()).await {
                Ok(result) => {
                    break result.map_err(|_| {
                        ffmpeg_error("FFMPEG_EXITED", "The media processor did not exit cleanly")
                    })?
                }
                Err(_) => continue,
            }
        };
        if status.success() {
            Ok(RunnerOutcome::Completed)
        } else {
            Err(ffmpeg_error(
                "FFMPEG_EXITED",
                "The media processor could not create the requested output",
            ))
        }
    }
}

#[async_trait]
impl MediaProcessorPort for FfmpegMediaProcessor {
    async fn process(
        &self,
        request: AudioProcessRequest,
        control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError> {
        if control.state() == ProcessControlState::CancelRequested {
            remove_processed(&request.output_path);
            return Ok(ProcessOutcome::Cancelled);
        }
        if !is_non_empty_file(&request.source_path) || !is_non_empty_file(&request.cover_path) {
            return Err(ffmpeg_error(
                "FFMPEG_INPUT_INVALID",
                "The audio processing input is unavailable",
            ));
        }
        let args = build_ffmpeg_args(&request)?;
        remove_processed(&request.output_path);
        let outcome = match self.runner.run(&self.executable, args, control).await {
            Ok(outcome) => outcome,
            Err(error) => {
                remove_processed(&request.output_path);
                return Err(error);
            }
        };
        if outcome == RunnerOutcome::Cancelled {
            remove_processed(&request.output_path);
            return Ok(ProcessOutcome::Cancelled);
        }
        if !is_non_empty_file(&request.output_path) {
            remove_processed(&request.output_path);
            return Err(ffmpeg_error(
                "FFMPEG_OUTPUT_INVALID",
                "The processed audio output is invalid",
            ));
        }
        Ok(ProcessOutcome::Completed)
    }
}

#[async_trait]
impl MediaProcessorPort for DeferredFfmpegMediaProcessor {
    async fn process(
        &self,
        request: AudioProcessRequest,
        control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError> {
        FfmpegMediaProcessor::new(&self.configured_path)?
            .process(request, control)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{DeferredFfmpegMediaProcessor, FfmpegMediaProcessor, FfmpegRunner, RunnerOutcome};
    use crate::{
        models::{AppError, AudioOutputProfile},
        services::audio::{
            AudioProcessRequest, AudioSourceTier, MediaProcessorPort, ProcessControl,
            ProcessOutcome,
        },
    };

    struct FakeRunner {
        outcome: RunnerOutcome,
        output_path: std::path::PathBuf,
        output: Option<Vec<u8>>,
        arguments: Mutex<Vec<std::ffi::OsString>>,
    }

    #[async_trait::async_trait]
    impl FfmpegRunner for FakeRunner {
        async fn run(
            &self,
            _executable: &std::path::Path,
            arguments: Vec<std::ffi::OsString>,
            _control: ProcessControl,
        ) -> Result<RunnerOutcome, AppError> {
            *self.arguments.lock().unwrap() = arguments;
            if let Some(output) = &self.output {
                std::fs::write(&self.output_path, output).unwrap();
            }
            Ok(self.outcome)
        }
    }

    fn fixture(
        root: &std::path::Path,
        outcome: RunnerOutcome,
        output: Option<Vec<u8>>,
    ) -> (FfmpegMediaProcessor, AudioProcessRequest, Arc<FakeRunner>) {
        let source_path = root.join("source.m4s");
        let cover_path = root.join("cover.jpg");
        let output_path = root.join("processed.mp3");
        std::fs::write(&source_path, b"audio").unwrap();
        std::fs::write(&cover_path, b"cover").unwrap();
        let runner = Arc::new(FakeRunner {
            outcome,
            output_path: output_path.clone(),
            output,
            arguments: Mutex::new(Vec::new()),
        });
        let processor = FfmpegMediaProcessor::with_runner(root.join("ffmpeg.exe"), runner.clone());
        let request = AudioProcessRequest {
            source_path,
            cover_path,
            output_path,
            output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 192 },
            source_tier: AudioSourceTier::Lossy,
            title: "Fixture".into(),
            uploader: "UP".into(),
        };
        (processor, request, runner)
    }

    #[test]
    fn fake_runner_success_requires_a_non_empty_output_and_receives_discrete_args() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let (processor, request, runner) = fixture(
                temp.path(),
                RunnerOutcome::Completed,
                Some(b"output".to_vec()),
            );

            let outcome = processor
                .process(request, ProcessControl::new())
                .await
                .unwrap();

            assert_eq!(outcome, ProcessOutcome::Completed);
            assert!(runner
                .arguments
                .lock()
                .unwrap()
                .iter()
                .any(|value| value == "title=Fixture"));
        });
    }

    #[test]
    fn fake_runner_empty_output_and_cancel_are_cleaned_and_classified() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let (processor, request, _) = fixture(temp.path(), RunnerOutcome::Completed, None);
            let error = processor
                .process(request.clone(), ProcessControl::new())
                .await
                .unwrap_err();
            assert_eq!(error.details.as_deref(), Some("FFMPEG_OUTPUT_INVALID"));

            let (processor, request, _) = fixture(
                temp.path(),
                RunnerOutcome::Cancelled,
                Some(b"partial".to_vec()),
            );
            let outcome = processor
                .process(request.clone(), ProcessControl::new())
                .await
                .unwrap();
            assert_eq!(outcome, ProcessOutcome::Cancelled);
            assert!(!request.output_path.exists());
        });
    }

    #[test]
    fn deferred_processor_maps_a_missing_sidecar_to_stable_e008() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let (_, request, _) = fixture(temp.path(), RunnerOutcome::Completed, None);
            let processor = DeferredFfmpegMediaProcessor::new(temp.path().join("ffmpeg.exe"));

            let error = processor
                .process(request, ProcessControl::new())
                .await
                .unwrap_err();

            assert_eq!(error.code, crate::models::AppErrorCode::E008);
            assert_eq!(error.details.as_deref(), Some("FFMPEG_UNAVAILABLE"));
        });
    }
}
