use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

use crate::{
    models::{AppError, AppErrorCode, GpuMode, TranscriptSegment, TranscriptionLanguage},
    services::{
        audio::{ProcessControl, ProcessControlState},
        transcription::ports::{
            Transcriber, TranscriberOutput, TranscriberProgress, TranscriberRequest,
        },
    },
};

pub const WHISPER_CPP_VERSION: &str = "1.8.6";
pub type WhisperRequest = TranscriberRequest;

pub fn resolve_whisper_sidecar(configured_path: &Path) -> Result<PathBuf, AppError> {
    let valid_name = configured_path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            [
                "whisper-cli.exe",
                "whisper-cli",
                "whisper-cli-x86_64-pc-windows-msvc.exe",
                "whisper-cli-x86_64-apple-darwin",
                "whisper-cli-aarch64-apple-darwin",
                "whisper-cli-x86_64-unknown-linux-gnu",
                "whisper-cli-aarch64-unknown-linux-gnu",
            ]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
        });
    if !configured_path.is_absolute() || !configured_path.is_file() || !valid_name {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_UNAVAILABLE",
            "The transcription engine is unavailable",
        ));
    }
    Ok(configured_path.to_path_buf())
}

pub fn build_whisper_args(request: &WhisperRequest) -> Vec<OsString> {
    let mut arguments = vec![
        "-m".into(),
        request.model_path.as_os_str().to_owned(),
        "-f".into(),
        request.audio_path.as_os_str().to_owned(),
        "-l".into(),
        language_code(request.language).into(),
        "-ojf".into(),
        "-of".into(),
        request.output_base.as_os_str().to_owned(),
        "-pp".into(),
        "-np".into(),
    ];
    if request.gpu_mode == GpuMode::Disabled {
        arguments.push("-ng".into());
    }
    arguments
}

pub fn parse_progress(line: &str) -> Option<f64> {
    let marker = "progress = ";
    let start = line.find(marker)? + marker.len();
    let percent = line.get(start..)?.trim_start().split('%').next()?.trim();
    let value = percent.parse::<f64>().ok()?;
    (value.is_finite() && (0.0..=100.0).contains(&value)).then(|| value / 100.0)
}

pub fn parse_whisper_json(bytes: &[u8], offset_ms: u64) -> Result<TranscriberOutput, AppError> {
    let document: WhisperDocument = serde_json::from_slice(bytes).map_err(|_| {
        sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_OUTPUT_INVALID",
            "The transcription engine returned invalid output",
        )
    })?;
    let detected_language = document.result.language.trim().to_owned();
    if detected_language.is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_LANGUAGE_MISSING",
            "The transcription language is unavailable",
        ));
    }
    let mut segments = Vec::new();
    for segment in document.transcription {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        let start_ms = offset_ms.checked_add(segment.offsets.from).ok_or_else(|| {
            sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_TIMESTAMP_OVERFLOW",
                "The transcription timestamps are invalid",
            )
        })?;
        let end_ms = offset_ms
            .checked_add(segment.offsets.to.max(segment.offsets.from))
            .ok_or_else(|| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_TIMESTAMP_OVERFLOW",
                    "The transcription timestamps are invalid",
                )
            })?;
        segments.push(TranscriptSegment {
            index: segments.len() as u32,
            start_ms,
            end_ms,
            text: text.into(),
        });
    }
    if segments.is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E015,
            "NO_SPEECH_DETECTED",
            "No speech was detected in the media",
        ));
    }
    Ok(TranscriberOutput {
        detected_language,
        segments,
    })
}

#[derive(Debug, Deserialize)]
struct WhisperDocument {
    result: WhisperResult,
    transcription: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperResult {
    language: String,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    offsets: WhisperOffsets,
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperOffsets {
    from: u64,
    to: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SidecarRunResult {
    Completed { success: bool, diagnostics: String },
    Cancelled,
}

#[async_trait]
trait SidecarRunner: Send + Sync {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        diagnostics_path: &Path,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<SidecarRunResult, AppError>;
}

pub struct WhisperSidecarTranscriber {
    executable: PathBuf,
    runner: Arc<dyn SidecarRunner>,
}

impl WhisperSidecarTranscriber {
    pub fn new(configured_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            executable: resolve_whisper_sidecar(configured_path)?,
            runner: Arc::new(TokioSidecarRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(executable: PathBuf, runner: Arc<dyn SidecarRunner>) -> Self {
        Self { executable, runner }
    }

    async fn execute_once(
        &self,
        request: &WhisperRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<SidecarRunResult, AppError> {
        let diagnostics_path = append_suffix(&request.output_base, ".stderr.log");
        let _ = fs::remove_file(&diagnostics_path);
        let result = self
            .runner
            .run(
                &self.executable,
                build_whisper_args(request),
                &diagnostics_path,
                control,
                progress,
            )
            .await;
        let _ = fs::remove_file(diagnostics_path);
        result
    }
}

#[async_trait]
impl Transcriber for WhisperSidecarTranscriber {
    async fn transcribe(
        &self,
        mut request: TranscriberRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<TranscriberOutput, AppError> {
        validate_request(&request)?;
        let output_path = append_suffix(&request.output_base, ".json");
        let _ = fs::remove_file(&output_path);
        let mut result = self
            .execute_once(&request, control.clone(), progress.clone())
            .await?;
        if let SidecarRunResult::Completed {
            success: false,
            diagnostics,
        } = &result
        {
            if is_gpu_failure(diagnostics) && request.gpu_mode == GpuMode::Auto {
                request.gpu_mode = GpuMode::Disabled;
                let _ = fs::remove_file(&output_path);
                result = self.execute_once(&request, control, progress).await?;
            }
        }
        match result {
            SidecarRunResult::Cancelled => Err(sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_CANCELLED",
                "Transcription was cancelled",
            )),
            SidecarRunResult::Completed {
                success: false,
                diagnostics,
            } if is_gpu_failure(&diagnostics) => Err(sidecar_error(
                AppErrorCode::E014,
                "GPU_UNAVAILABLE",
                "GPU transcription is unavailable",
            )),
            SidecarRunResult::Completed {
                success: false,
                diagnostics,
            } => Err(sidecar_failure(&diagnostics)),
            SidecarRunResult::Completed {
                success: true,
                diagnostics,
            } if !output_path.is_file() => Err(sidecar_failure(&diagnostics)),
            SidecarRunResult::Completed {
                success: true,
                diagnostics: _,
            } => {
                let bytes = fs::read(&output_path).map_err(|_| {
                    sidecar_error(
                        AppErrorCode::E014,
                        "SIDECAR_OUTPUT_MISSING",
                        "The transcription engine did not produce output",
                    )
                })?;
                let parsed = parse_whisper_json(&bytes, request.offset_ms);
                let _ = fs::remove_file(output_path);
                parsed
            }
        }
    }
}

struct TokioSidecarRunner;

#[async_trait]
impl SidecarRunner for TokioSidecarRunner {
    async fn run(
        &self,
        executable: &Path,
        arguments: Vec<OsString>,
        diagnostics_path: &Path,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<SidecarRunResult, AppError> {
        let diagnostics = fs::File::create(diagnostics_path).map_err(|_| {
            sidecar_error(
                AppErrorCode::E014,
                "SIDECAR_DIAGNOSTICS_UNAVAILABLE",
                "The transcription engine could not be started",
            )
        })?;
        let mut command = Command::new(executable);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000 | 0x0000_0200);
        let mut child = command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(diagnostics))
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_UNAVAILABLE",
                    "The transcription engine could not be started",
                )
            })?;

        loop {
            if control.state() == ProcessControlState::CancelRequested {
                terminate_process_tree(&mut child).await?;
                return Ok(SidecarRunResult::Cancelled);
            }
            if let Some(status) = child.try_wait().map_err(|_| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_EXIT_FAILED",
                    "The transcription engine did not exit cleanly",
                )
            })? {
                let text = read_diagnostics(diagnostics_path);
                return Ok(SidecarRunResult::Completed {
                    success: status.success(),
                    diagnostics: text,
                });
            }
            if let Some(value) = latest_progress(diagnostics_path) {
                progress(value);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

fn language_code(language: TranscriptionLanguage) -> &'static str {
    match language {
        TranscriptionLanguage::Auto => "auto",
        TranscriptionLanguage::Zh | TranscriptionLanguage::ZhTw => "zh",
        TranscriptionLanguage::En => "en",
        TranscriptionLanguage::Ja => "ja",
        TranscriptionLanguage::Ko => "ko",
    }
}

fn validate_request(request: &WhisperRequest) -> Result<(), AppError> {
    if !request.model_path.is_file() {
        return Err(sidecar_error(
            AppErrorCode::E011,
            "MODEL_UNAVAILABLE",
            "The selected transcription model is unavailable",
        ));
    }
    if !request.audio_path.is_file() {
        return Err(sidecar_error(
            AppErrorCode::E013,
            "AUDIO_PART_UNAVAILABLE",
            "The preprocessed audio is unavailable",
        ));
    }
    if request.output_base.as_os_str().is_empty() {
        return Err(sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_OUTPUT_INVALID",
            "The transcription output path is invalid",
        ));
    }
    Ok(())
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(suffix);
    value.into()
}

fn is_gpu_failure(diagnostics: &str) -> bool {
    let value = diagnostics.to_ascii_lowercase();
    value.contains("gpu")
        && (value.contains("failed")
            || value.contains("unavailable")
            || value.contains("not found")
            || value.contains("unsupported"))
}

fn read_diagnostics(path: &Path) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]).into_owned()
}

fn latest_progress(path: &Path) -> Option<f64> {
    read_diagnostics(path)
        .lines()
        .rev()
        .find_map(parse_progress)
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
                "The transcription engine could not be stopped",
            )
        })?;
    let _ = child.wait().await;
    Ok(())
}

fn sidecar_failure(diagnostics: &str) -> AppError {
    let details = if is_gpu_failure(diagnostics) {
        "GPU_UNAVAILABLE"
    } else {
        "SIDECAR_FAILED"
    };
    sidecar_error(
        AppErrorCode::E014,
        details,
        "The transcription engine failed",
    )
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
    use std::{
        collections::VecDeque,
        ffi::OsString,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;

    use super::{SidecarRunResult, SidecarRunner, WhisperSidecarTranscriber};
    use crate::{
        models::{AppError, GpuMode, TranscriptionLanguage},
        services::{
            audio::ProcessControl,
            transcription::ports::{Transcriber, TranscriberProgress, TranscriberRequest},
        },
    };

    struct FakeRunner {
        outcomes: Mutex<VecDeque<SidecarRunResult>>,
        calls: Mutex<Vec<Vec<OsString>>>,
    }

    #[async_trait]
    impl SidecarRunner for FakeRunner {
        async fn run(
            &self,
            _executable: &Path,
            arguments: Vec<OsString>,
            _diagnostics_path: &Path,
            _control: ProcessControl,
            _progress: TranscriberProgress,
        ) -> Result<SidecarRunResult, AppError> {
            self.calls.lock().unwrap().push(arguments.clone());
            let outcome = self.outcomes.lock().unwrap().pop_front().unwrap();
            if matches!(outcome, SidecarRunResult::Completed { success: true, .. }) {
                let index = arguments.iter().position(|value| value == "-of").unwrap();
                let mut output = PathBuf::from(&arguments[index + 1]);
                let mut name = output.as_os_str().to_owned();
                name.push(".json");
                output = name.into();
                std::fs::write(
                    output,
                    br#"{"result":{"language":"en"},"transcription":[{"offsets":{"from":0,"to":100},"text":"hello"}]}"#,
                )
                .unwrap();
            }
            Ok(outcome)
        }
    }

    fn request(root: &Path, gpu_mode: GpuMode) -> TranscriberRequest {
        let model_path = root.join("model.bin");
        let audio_path = root.join("part.wav");
        std::fs::write(&model_path, b"model").unwrap();
        std::fs::write(&audio_path, b"wav").unwrap();
        TranscriberRequest {
            model_path,
            audio_path,
            output_base: root.join("result"),
            language: TranscriptionLanguage::Auto,
            gpu_mode,
            offset_ms: 0,
        }
    }

    #[test]
    fn auto_gpu_retries_once_on_cpu_but_enabled_gpu_does_not_fallback() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let runner = Arc::new(FakeRunner {
                outcomes: Mutex::new(VecDeque::from([
                    SidecarRunResult::Completed {
                        success: false,
                        diagnostics: "GPU backend failed".into(),
                    },
                    SidecarRunResult::Completed {
                        success: true,
                        diagnostics: String::new(),
                    },
                ])),
                calls: Mutex::new(Vec::new()),
            });
            let transcriber = WhisperSidecarTranscriber::with_runner(
                temp.path().join("whisper-cli.exe"),
                runner.clone(),
            );

            let output = transcriber
                .transcribe(
                    request(temp.path(), GpuMode::Auto),
                    ProcessControl::new(),
                    Arc::new(|_| {}),
                )
                .await
                .unwrap();

            assert_eq!(output.segments[0].text, "hello");
            assert_eq!(runner.calls.lock().unwrap().len(), 2);
            assert!(runner.calls.lock().unwrap()[1]
                .iter()
                .any(|value| value == "-ng"));
        });
    }

    #[test]
    fn enabled_gpu_failure_returns_stable_error_without_retry() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let runner = Arc::new(FakeRunner {
                outcomes: Mutex::new(VecDeque::from([SidecarRunResult::Completed {
                    success: false,
                    diagnostics: "GPU backend unavailable".into(),
                }])),
                calls: Mutex::new(Vec::new()),
            });
            let transcriber = WhisperSidecarTranscriber::with_runner(
                temp.path().join("whisper-cli.exe"),
                runner.clone(),
            );

            let error = transcriber
                .transcribe(
                    request(temp.path(), GpuMode::Enabled),
                    ProcessControl::new(),
                    Arc::new(|_| {}),
                )
                .await
                .unwrap_err();

            assert_eq!(error.details.as_deref(), Some("GPU_UNAVAILABLE"));
            assert_eq!(runner.calls.lock().unwrap().len(), 1);
        });
    }
}
