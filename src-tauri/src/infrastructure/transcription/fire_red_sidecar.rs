use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::{io::AsyncReadExt, process::Command};

use crate::{
    models::{AppError, AppErrorCode, GpuMode, TranscriptSegment, TranscriptionLanguage},
    services::{
        audio::{ProcessControl, ProcessControlState},
        transcription::ports::{
            Transcriber, TranscriberOutput, TranscriberProgress, TranscriberRequest,
        },
    },
};

pub const SHERPA_ONNX_VERSION: &str = "1.13.8";
pub const FIRE_RED_MAX_CHUNK_SECONDS: f64 = 55.0;
pub type FireRedRequest = TranscriberRequest;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub fn resolve_fire_red_sidecar(configured_path: &Path) -> Result<PathBuf, AppError> {
    let valid_name = configured_path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            [
                "sherpa-onnx-offline.exe",
                "sherpa-onnx-offline",
                "sherpa-onnx-offline-x86_64-pc-windows-msvc.exe",
                "sherpa-onnx-offline-x86_64-apple-darwin",
                "sherpa-onnx-offline-aarch64-apple-darwin",
            ]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
        });
    if !configured_path.is_absolute() || !configured_path.is_file() || !valid_name {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_UNAVAILABLE",
            "The FireRedASR2 transcription engine is unavailable",
        ));
    }
    Ok(configured_path.to_path_buf())
}

pub fn build_fire_red_args(request: &FireRedRequest, tokens_path: &Path) -> Vec<OsString> {
    vec![
        prefixed_path("--tokens=", tokens_path),
        prefixed_path("--fire-red-asr-ctc=", &request.model_path),
        "--model-type=fire_red_asr".into(),
        "--num-threads=4".into(),
        "--debug=false".into(),
        request.audio_path.as_os_str().to_owned(),
    ]
}

pub fn parse_fire_red_json(
    bytes: &[u8],
    offset_ms: u64,
    requested_language: TranscriptionLanguage,
) -> Result<TranscriberOutput, AppError> {
    let document: FireRedDocument = serde_json::from_slice(bytes).map_err(|_| {
        sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_OUTPUT_INVALID",
            "The FireRedASR2 engine returned invalid output",
        )
    })?;
    if document.text.trim().is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E015,
            "NO_SPEECH_DETECTED",
            "No speech was detected in the media",
        ));
    }

    let segments = if document.tokens.len() == document.timestamps.len()
        && !document.tokens.is_empty()
        && document.timestamps.iter().all(|value| value.is_finite())
    {
        token_segments(&document.tokens, &document.timestamps, offset_ms)?
    } else {
        fallback_segment(&document, offset_ms)?
    };

    Ok(TranscriberOutput {
        detected_language: detected_language(requested_language).into(),
        segments,
    })
}

#[derive(Debug, Deserialize)]
struct FireRedDocument {
    #[serde(default)]
    text: String,
    #[serde(default)]
    timestamps: Vec<f64>,
    #[serde(default)]
    tokens: Vec<String>,
}

fn token_segments(
    tokens: &[String],
    timestamps: &[f64],
    offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, AppError> {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut start_seconds = timestamps[0].max(0.0);

    for (index, (token, timestamp)) in tokens.iter().zip(timestamps).enumerate() {
        let timestamp = timestamp.max(0.0);
        if text.is_empty() {
            start_seconds = timestamp;
        }
        text.push_str(token);
        let punctuation_boundary = token.chars().last().is_some_and(|value| {
            matches!(
                value,
                '.' | '?' | '!' | ';' | '\u{3002}' | '\u{ff1f}' | '\u{ff01}' | '\u{ff1b}'
            )
        });
        let duration_boundary = timestamp - start_seconds >= 8.0;
        let final_token = index + 1 == tokens.len();
        if punctuation_boundary || duration_boundary || final_token {
            let value = text.trim();
            if !value.is_empty() {
                segments.push(TranscriptSegment {
                    index: segments.len() as u32,
                    start_ms: timestamp_ms(offset_ms, start_seconds)?,
                    end_ms: timestamp_ms(offset_ms, timestamp + 0.5)?,
                    text: value.into(),
                });
            }
            text.clear();
        }
    }
    if segments.is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E015,
            "NO_SPEECH_DETECTED",
            "No speech was detected in the media",
        ));
    }
    Ok(segments)
}

fn fallback_segment(
    document: &FireRedDocument,
    offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, AppError> {
    let start = document.timestamps.first().copied().unwrap_or(0.0).max(0.0);
    let end = document
        .timestamps
        .last()
        .copied()
        .unwrap_or(start)
        .max(start)
        + 0.5;
    Ok(vec![TranscriptSegment {
        index: 0,
        start_ms: timestamp_ms(offset_ms, start)?,
        end_ms: timestamp_ms(offset_ms, end)?,
        text: document.text.trim().into(),
    }])
}

fn timestamp_ms(offset_ms: u64, seconds: f64) -> Result<u64, AppError> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_TIMESTAMP_INVALID",
            "The FireRedASR2 timestamps are invalid",
        ));
    }
    offset_ms
        .checked_add((seconds * 1000.0).round() as u64)
        .ok_or_else(|| {
            sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_TIMESTAMP_OVERFLOW",
                "The FireRedASR2 timestamps are invalid",
            )
        })
}

fn detected_language(language: TranscriptionLanguage) -> &'static str {
    match language {
        TranscriptionLanguage::En => "en",
        TranscriptionLanguage::Zh | TranscriptionLanguage::ZhTw | TranscriptionLanguage::Auto => {
            "zh"
        }
        TranscriptionLanguage::Ja | TranscriptionLanguage::Ko => "und",
    }
}

fn prefixed_path(prefix: &str, path: &Path) -> OsString {
    let mut value = OsString::from(prefix);
    value.push(path.as_os_str());
    value
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SidecarRunResult {
    Completed {
        success: bool,
        stdout: Vec<u8>,
        diagnostics: String,
    },
    Cancelled,
}

#[async_trait]
trait SidecarRunner: Send + Sync {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<SidecarRunResult, AppError>;
}

pub struct FireRedSidecarTranscriber {
    executable: PathBuf,
    tokens_path: PathBuf,
    runner: Arc<dyn SidecarRunner>,
}

impl FireRedSidecarTranscriber {
    pub fn new(configured_path: &Path, tokens_path: &Path) -> Result<Self, AppError> {
        if !tokens_path.is_absolute() || !tokens_path.is_file() {
            return Err(sidecar_error(
                AppErrorCode::E014,
                "TOKENS_UNAVAILABLE",
                "The FireRedASR2 token table is unavailable",
            ));
        }
        Ok(Self {
            executable: resolve_fire_red_sidecar(configured_path)?,
            tokens_path: tokens_path.into(),
            runner: Arc::new(TokioSidecarRunner),
        })
    }
}

#[async_trait]
impl Transcriber for FireRedSidecarTranscriber {
    async fn transcribe(
        &self,
        request: TranscriberRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<TranscriberOutput, AppError> {
        validate_request(&request)?;
        let staged = stage_runtime_paths(request, &self.tokens_path)?;
        match self
            .runner
            .run(
                &self.executable,
                build_fire_red_args(&staged.request, &staged.tokens_path),
                control,
            )
            .await?
        {
            SidecarRunResult::Cancelled => Err(sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_CANCELLED",
                "Transcription was cancelled",
            )),
            SidecarRunResult::Completed {
                success: false,
                diagnostics,
                ..
            } => Err(sidecar_failure(&diagnostics)),
            SidecarRunResult::Completed { stdout, .. } => {
                progress(1.0);
                parse_fire_red_json(&stdout, staged.request.offset_ms, staged.request.language)
            }
        }
    }
}

struct StagedFireRedPaths {
    request: TranscriberRequest,
    tokens_path: PathBuf,
    cleanup_audio: Option<PathBuf>,
}

impl Drop for StagedFireRedPaths {
    fn drop(&mut self) {
        if let Some(path) = &self.cleanup_audio {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(not(windows))]
fn stage_runtime_paths(
    request: TranscriberRequest,
    tokens_path: &Path,
) -> Result<StagedFireRedPaths, AppError> {
    Ok(StagedFireRedPaths {
        request,
        tokens_path: tokens_path.into(),
        cleanup_audio: None,
    })
}

#[cfg(windows)]
fn stage_runtime_paths(
    request: TranscriberRequest,
    tokens_path: &Path,
) -> Result<StagedFireRedPaths, AppError> {
    if [&request.model_path, &request.audio_path, tokens_path]
        .into_iter()
        .all(|path| path.to_string_lossy().is_ascii())
    {
        return Ok(StagedFireRedPaths {
            request,
            tokens_path: tokens_path.into(),
            cleanup_audio: None,
        });
    }
    let root = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .filter(|path| path.to_string_lossy().is_ascii())
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("BiliCatch")
        .join("fire-red-runtime");
    stage_windows_paths_at(request, tokens_path, &root)
}

#[cfg(windows)]
fn stage_windows_paths_at(
    mut request: TranscriberRequest,
    tokens_path: &Path,
    staging_root: &Path,
) -> Result<StagedFireRedPaths, AppError> {
    if !staging_root.to_string_lossy().is_ascii() {
        return Err(staging_error());
    }
    fs::create_dir_all(staging_root).map_err(|_| staging_error())?;
    request.model_path = stage_cached_path(&request.model_path, staging_root, "model", "onnx")?;
    let staged_tokens = stage_cached_path(tokens_path, staging_root, "tokens", "txt")?;
    let (audio_path, cleanup_audio) = if request.audio_path.to_string_lossy().is_ascii() {
        (request.audio_path.clone(), None)
    } else {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let destination = staging_root.join(format!("audio-{}-{sequence}.wav", std::process::id()));
        publish_staged_file(&request.audio_path, &destination)?;
        (destination.clone(), Some(destination))
    };
    request.audio_path = audio_path;
    Ok(StagedFireRedPaths {
        request,
        tokens_path: staged_tokens,
        cleanup_audio,
    })
}

#[cfg(windows)]
fn stage_cached_path(
    source: &Path,
    staging_root: &Path,
    label: &str,
    extension: &str,
) -> Result<PathBuf, AppError> {
    if source.to_string_lossy().is_ascii() {
        return Ok(source.into());
    }
    let size = fs::metadata(source).map_err(|_| staging_error())?.len();
    let destination = staging_root.join(format!("{label}-{size}.{extension}"));
    if fs::metadata(&destination).is_ok_and(|metadata| metadata.is_file() && metadata.len() == size)
    {
        return Ok(destination);
    }
    let _ = fs::remove_file(&destination);
    publish_staged_file(source, &destination)?;
    Ok(destination)
}

#[cfg(windows)]
fn publish_staged_file(source: &Path, destination: &Path) -> Result<(), AppError> {
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = destination.with_extension(format!("part-{}-{sequence}", std::process::id()));
    let _ = fs::remove_file(&temporary);
    if fs::hard_link(source, &temporary).is_err() && fs::copy(source, &temporary).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(staging_error());
    }
    match fs::rename(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(_) if destination.is_file() => {
            let _ = fs::remove_file(temporary);
            Ok(())
        }
        Err(_) => {
            let _ = fs::remove_file(temporary);
            Err(staging_error())
        }
    }
}

#[cfg(windows)]
fn staging_error() -> AppError {
    sidecar_error(
        AppErrorCode::E014,
        "SIDECAR_ASCII_WORKSPACE_UNAVAILABLE",
        "FireRedASR2 could not prepare a Windows-compatible runtime path",
    )
}

struct TokioSidecarRunner;

#[async_trait]
impl SidecarRunner for TokioSidecarRunner {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        control: ProcessControl,
    ) -> Result<SidecarRunResult, AppError> {
        let mut command = Command::new(executable);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000 | 0x0000_0200);
        let mut child = command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_UNAVAILABLE",
                    "The FireRedASR2 engine could not be started",
                )
            })?;
        let mut stdout = child.stdout.take().ok_or_else(pipe_error)?;
        let mut stderr = child.stderr.take().ok_or_else(pipe_error)?;
        let stdout_reader = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).await.map(|_| bytes)
        });
        let stderr_reader = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).await.map(|_| bytes)
        });

        loop {
            if control.state() == ProcessControlState::CancelRequested {
                terminate_process_tree(&mut child).await?;
                stdout_reader.abort();
                stderr_reader.abort();
                return Ok(SidecarRunResult::Cancelled);
            }
            if let Some(status) = child.try_wait().map_err(|_| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_EXIT_FAILED",
                    "The FireRedASR2 engine did not exit cleanly",
                )
            })? {
                let stdout = stdout_reader
                    .await
                    .map_err(|_| pipe_error())?
                    .map_err(|_| pipe_error())?;
                let stderr = stderr_reader
                    .await
                    .map_err(|_| pipe_error())?
                    .map_err(|_| pipe_error())?;
                return Ok(SidecarRunResult::Completed {
                    success: status.success(),
                    stdout,
                    diagnostics: String::from_utf8_lossy(&stderr[..stderr.len().min(64 * 1024)])
                        .into_owned(),
                });
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

fn validate_request(request: &FireRedRequest) -> Result<(), AppError> {
    if !request.model_path.is_file() {
        return Err(sidecar_error(
            AppErrorCode::E011,
            "MODEL_UNAVAILABLE",
            "The selected FireRedASR2 model is unavailable",
        ));
    }
    if !request.audio_path.is_file() {
        return Err(sidecar_error(
            AppErrorCode::E013,
            "AUDIO_PART_UNAVAILABLE",
            "The preprocessed audio is unavailable",
        ));
    }
    if request.gpu_mode == GpuMode::Enabled {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "GPU_UNAVAILABLE",
            "FireRedASR2 currently uses the bundled CPU runtime",
        ));
    }
    if matches!(
        request.language,
        TranscriptionLanguage::Ja | TranscriptionLanguage::Ko
    ) {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "MODEL_LANGUAGE_UNSUPPORTED",
            "FireRedASR2 supports Chinese and English transcription",
        ));
    }
    Ok(())
}

fn pipe_error() -> AppError {
    sidecar_error(
        AppErrorCode::E014,
        "SIDECAR_PIPE_FAILED",
        "The FireRedASR2 output stream is unavailable",
    )
}

async fn terminate_process_tree(child: &mut tokio::process::Child) -> Result<(), AppError> {
    #[cfg(windows)]
    if let Some(pid) = child.id() {
        let mut command = Command::new("taskkill.exe");
        command.creation_flags(0x0800_0000);
        let _ = command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }
    child
        .kill()
        .await
        .or_else(|error| {
            if child.try_wait().ok().flatten().is_some() {
                Ok(())
            } else {
                Err(error)
            }
        })
        .map_err(|_| {
            sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_CANCEL_FAILED",
                "The FireRedASR2 engine could not be stopped",
            )
        })?;
    let _ = child.wait().await;
    Ok(())
}

fn sidecar_failure(diagnostics: &str) -> AppError {
    let details = if diagnostics.to_ascii_lowercase().contains("model") {
        "MODEL_LOAD_FAILED"
    } else {
        "SIDECAR_FAILED"
    };
    sidecar_error(AppErrorCode::E014, details, "The FireRedASR2 engine failed")
}

fn sidecar_error(code: AppErrorCode, details: &str, message: &str) -> AppError {
    AppError {
        code,
        message: message.into(),
        details: Some(details.into()),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::path::Path;

    use super::stage_windows_paths_at;
    use crate::{
        models::{GpuMode, TranscriptionLanguage},
        services::transcription::ports::TranscriberRequest,
    };

    #[test]
    fn unicode_runtime_inputs_are_mirrored_to_ascii_paths_and_audio_is_cleaned_up() {
        let temp = tempfile::tempdir().unwrap();
        let unicode_root = temp.path().join("\u{6a21}\u{578b}");
        let staging_root = temp.path().join("ascii-stage");
        std::fs::create_dir_all(&unicode_root).unwrap();
        let model = unicode_root.join("model.onnx");
        let tokens = unicode_root.join("tokens.txt");
        let audio = unicode_root.join("audio.wav");
        std::fs::write(&model, b"model").unwrap();
        std::fs::write(&tokens, b"tokens").unwrap();
        std::fs::write(&audio, b"audio").unwrap();
        let request = TranscriberRequest {
            model_path: model,
            audio_path: audio,
            output_base: Path::new("D:/Temp/result").into(),
            language: TranscriptionLanguage::Zh,
            gpu_mode: GpuMode::Auto,
            offset_ms: 0,
        };

        let staged = stage_windows_paths_at(request, &tokens, &staging_root).unwrap();
        let staged_audio = staged.request.audio_path.clone();

        assert!(staged.request.model_path.to_string_lossy().is_ascii());
        assert!(staged.request.audio_path.to_string_lossy().is_ascii());
        assert!(staged.tokens_path.to_string_lossy().is_ascii());
        assert!(staged.request.model_path.is_file());
        assert!(staged_audio.is_file());
        drop(staged);
        assert!(!staged_audio.exists());
    }
}
