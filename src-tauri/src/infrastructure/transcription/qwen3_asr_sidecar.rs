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

pub const QWEN3_MAX_CHUNK_SECONDS: f64 = 20.0;
pub const CONV_FRONTEND_FILE: &str = "conv_frontend.onnx";
pub const ENCODER_FILE: &str = "encoder.int8.onnx";
pub const DECODER_FILE: &str = "decoder.int8.onnx";
pub type Qwen3Request = TranscriberRequest;

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub fn build_qwen3_args(request: &Qwen3Request) -> Vec<OsString> {
    vec![
        prefixed_path(
            "--qwen3-asr-conv-frontend=",
            &request.model_path.join(CONV_FRONTEND_FILE),
        ),
        prefixed_path(
            "--qwen3-asr-encoder=",
            &request.model_path.join(ENCODER_FILE),
        ),
        prefixed_path(
            "--qwen3-asr-decoder=",
            &request.model_path.join(DECODER_FILE),
        ),
        prefixed_path("--qwen3-asr-tokenizer=", &request.model_path),
        "--qwen3-asr-max-new-tokens=512".into(),
        "--num-threads=4".into(),
        "--debug=false".into(),
        request.audio_path.as_os_str().to_owned(),
    ]
}

pub fn parse_qwen3_json(
    bytes: &[u8],
    offset_ms: u64,
    requested_language: TranscriptionLanguage,
) -> Result<TranscriberOutput, AppError> {
    let document: Qwen3Document = serde_json::from_slice(bytes).map_err(|_| {
        sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_OUTPUT_INVALID",
            "The Qwen3-ASR engine returned invalid output",
        )
    })?;
    let text = document.text.trim();
    if text.is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E015,
            "NO_SPEECH_DETECTED",
            "No speech was detected in the media",
        ));
    }
    let detected = if document.lang.trim().is_empty() {
        detected_language(requested_language).to_owned()
    } else {
        document.lang.trim().to_owned()
    };
    Ok(TranscriberOutput {
        detected_language: detected,
        segments: vec![TranscriptSegment {
            index: 0,
            start_ms: offset_ms,
            end_ms: offset_ms,
            text: text.into(),
            confidence: average_confidence(&document.ys_log_probs),
            ..TranscriptSegment::default()
        }],
    })
}

#[derive(Debug, Deserialize)]
struct Qwen3Document {
    #[serde(default)]
    text: String,
    #[serde(default)]
    lang: String,
    #[serde(default)]
    ys_log_probs: Vec<f64>,
}

/// 把平均对数概率粗略映射到 0~1，供结果页展示置信度。
fn average_confidence(values: &[f64]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if !mean.is_finite() {
        return None;
    }
    Some(mean.exp().clamp(0.0, 1.0) as f32)
}

fn detected_language(language: TranscriptionLanguage) -> &'static str {
    match language {
        TranscriptionLanguage::En => "en",
        TranscriptionLanguage::Ja => "ja",
        TranscriptionLanguage::Ko => "ko",
        TranscriptionLanguage::Zh | TranscriptionLanguage::ZhTw | TranscriptionLanguage::Auto => {
            "zh"
        }
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

pub struct Qwen3AsrSidecarTranscriber {
    executable: PathBuf,
    runner: Arc<dyn SidecarRunner>,
}

impl Qwen3AsrSidecarTranscriber {
    pub fn new(configured_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            executable: resolve_qwen3_sidecar(configured_path)?,
            runner: Arc::new(TokioSidecarRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(executable: PathBuf, runner: Arc<dyn SidecarRunner>) -> Self {
        Self { executable, runner }
    }
}

pub fn resolve_qwen3_sidecar(configured_path: &Path) -> Result<PathBuf, AppError> {
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
            "The Qwen3-ASR engine is unavailable",
        ));
    }
    Ok(configured_path.to_path_buf())
}

#[async_trait]
impl Transcriber for Qwen3AsrSidecarTranscriber {
    async fn transcribe(
        &self,
        request: TranscriberRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<TranscriberOutput, AppError> {
        validate_request(&request)?;
        let mut staged = stage_runtime_paths(request)?;
        let offset_ms = staged.request.offset_ms;
        let language = staged.request.language;
        let result = self
            .runner
            .run(&self.executable, build_qwen3_args(&staged.request), control)
            .await;
        staged.cleanup();
        match result? {
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
                parse_qwen3_json(&stdout, offset_ms, language)
            }
        }
    }
}

fn validate_request(request: &Qwen3Request) -> Result<(), AppError> {
    let model_ready = request.model_path.is_dir()
        && request.model_path.join(CONV_FRONTEND_FILE).is_file()
        && request.model_path.join(ENCODER_FILE).is_file()
        && request.model_path.join(DECODER_FILE).is_file()
        && request.model_path.join("vocab.json").is_file();
    if !model_ready {
        return Err(sidecar_error(
            AppErrorCode::E011,
            "MODEL_UNAVAILABLE",
            "The selected Qwen3-ASR model is unavailable",
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
            "Qwen3-ASR currently uses the bundled CPU runtime",
        ));
    }
    Ok(())
}

struct StagedPaths {
    request: TranscriberRequest,
    cleanup_audio: Option<PathBuf>,
}

impl StagedPaths {
    fn cleanup(&mut self) {
        if let Some(path) = self.cleanup_audio.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(not(windows))]
fn stage_runtime_paths(request: TranscriberRequest) -> Result<StagedPaths, AppError> {
    Ok(StagedPaths {
        request,
        cleanup_audio: None,
    })
}

#[cfg(windows)]
fn stage_runtime_paths(request: TranscriberRequest) -> Result<StagedPaths, AppError> {
    if [&request.model_path, &request.audio_path]
        .into_iter()
        .all(|path| path.to_string_lossy().is_ascii())
    {
        return Ok(StagedPaths {
            request,
            cleanup_audio: None,
        });
    }
    let root = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .filter(|path| path.to_string_lossy().is_ascii())
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("BiliCatch")
        .join("qwen3-runtime");
    stage_windows_paths_at(request, &root)
}

#[cfg(windows)]
fn stage_windows_paths_at(
    mut request: TranscriberRequest,
    staging_root: &Path,
) -> Result<StagedPaths, AppError> {
    if !staging_root.to_string_lossy().is_ascii() {
        return Err(staging_error());
    }
    fs::create_dir_all(staging_root).map_err(|_| staging_error())?;
    request.model_path = stage_directory(&request.model_path, staging_root)?;
    let (audio_path, cleanup_audio) = if request.audio_path.to_string_lossy().is_ascii() {
        (request.audio_path.clone(), None)
    } else {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let destination =
            staging_root.join(format!("audio-{}-{sequence}.wav", std::process::id()));
        publish_staged_file(&request.audio_path, &destination)?;
        (destination.clone(), Some(destination))
    };
    request.audio_path = audio_path;
    Ok(StagedPaths {
        request,
        cleanup_audio,
    })
}

#[cfg(windows)]
fn stage_directory(source: &Path, staging_root: &Path) -> Result<PathBuf, AppError> {
    if source.to_string_lossy().is_ascii() {
        return Ok(source.into());
    }
    let mut total_size = 0_u64;
    let mut files = Vec::new();
    for entry in fs::read_dir(source).map_err(|_| staging_error())? {
        let path = entry.map_err(|_| staging_error())?.path();
        if path.is_file() {
            total_size += fs::metadata(&path).map_err(|_| staging_error())?.len();
            files.push(path);
        }
    }
    if files.is_empty() {
        return Err(staging_error());
    }
    let destination = staging_root.join(format!("qwen3-{total_size}"));
    if fs::metadata(&destination).is_ok_and(|metadata| metadata.is_dir()) {
        return Ok(destination);
    }
    fs::create_dir_all(&destination).map_err(|_| staging_error())?;
    for path in files {
        let name = path.file_name().ok_or_else(staging_error)?;
        publish_staged_file(&path, &destination.join(name))?;
    }
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
        "Qwen3-ASR could not prepare a Windows-compatible runtime path",
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
                    "The Qwen3-ASR engine could not be started",
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
                    "The Qwen3-ASR engine did not exit cleanly",
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

fn pipe_error() -> AppError {
    sidecar_error(
        AppErrorCode::E014,
        "SIDECAR_PIPE_FAILED",
        "The Qwen3-ASR output stream is unavailable",
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
                "The Qwen3-ASR engine could not be stopped",
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
    sidecar_error(AppErrorCode::E014, details, "The Qwen3-ASR engine failed")
}

fn sidecar_error(code: AppErrorCode, details: &str, message: &str) -> AppError {
    AppError {
        code,
        message: message.into(),
        details: Some(details.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{build_qwen3_args, parse_qwen3_json};
    use crate::{
        models::{GpuMode, TranscriptionLanguage},
        services::transcription::ports::TranscriberRequest,
    };

    fn request(root: &Path) -> TranscriberRequest {
        TranscriberRequest {
            model_path: root.join("qwen3-asr"),
            audio_path: root.join("part.wav"),
            output_base: root.join("result"),
            language: TranscriptionLanguage::Ja,
            gpu_mode: GpuMode::Auto,
            offset_ms: 12_000,
        }
    }

    #[test]
    fn arguments_point_at_every_package_file() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("qwen3-asr");
        let arguments = build_qwen3_args(&request(temp.path()))
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(arguments.contains(&format!(
            "--qwen3-asr-conv-frontend={}",
            directory.join("conv_frontend.onnx").display()
        )));
        assert!(arguments.contains(&format!(
            "--qwen3-asr-tokenizer={}",
            directory.display()
        )));
        assert!(arguments
            .last()
            .is_some_and(|value| value.ends_with("part.wav")));
    }

    #[test]
    fn json_parser_emits_offset_text_and_language() {
        let payload = br#"{"lang":"ja","text":"\u3053\u3093\u306b\u3061\u306f","timestamps":[],"tokens":[]}"#;

        let output = parse_qwen3_json(payload, 12_000, TranscriptionLanguage::Ja).unwrap();

        assert_eq!(output.detected_language, "ja");
        assert_eq!(output.segments.len(), 1);
        assert_eq!(output.segments[0].start_ms, 12_000);
        assert_eq!(output.segments[0].text, "こんにちは");
    }

    #[test]
    fn empty_text_is_reported_as_no_speech() {
        let error = parse_qwen3_json(br#"{"lang":"ja","text":"  "}"#, 0, TranscriptionLanguage::Ja)
            .unwrap_err();

        assert_eq!(error.code, crate::models::AppErrorCode::E015);
    }
}
