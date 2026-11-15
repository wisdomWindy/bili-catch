use std::{
    ffi::OsString,
    path::PathBuf,
    process::Stdio,
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

/// OpenASR 目录里的模型 id。
pub const OPENASR_MODEL: &str = "firered2-llm";
/// 8B 单次输入上限 40 秒。
pub const OPENASR_MAX_CHUNK_SECONDS: f64 = 40.0;
/// 覆盖路径用的环境变量，便于未打包时手动验证。
pub const OPENASR_BIN_ENV: &str = "BILICATCH_OPENASR_BIN";

pub fn build_openasr_args(request: &TranscriberRequest) -> Vec<OsString> {
    // firered2-llm 内置中英语种集，显式传 -l 会被拒绝（实测：unsupported_capability）。
    vec![
        "transcribe".into(),
        request.audio_path.as_os_str().to_owned(),
        "-m".into(),
        OPENASR_MODEL.into(),
        "-f".into(),
        "json".into(),
        "--offline".into(),
    ]
}

/// 依次尝试环境变量、`LOCALAPPDATA` 与程序目录。
pub fn resolve_openasr_binary() -> Result<PathBuf, AppError> {
    if let Some(value) = std::env::var_os(OPENASR_BIN_ENV) {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Ok(path);
        }
        return Err(sidecar_error(
            AppErrorCode::E014,
            "OPENASR_PATH_INVALID",
            "The configured OpenASR binary does not exist",
        ));
    }
    let mut candidates = Vec::new();
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(base).join("OpenASR").join(binary_name()));
    }
    if let Some(base) = std::env::var_os("PROGRAMDATA") {
        candidates.push(
            PathBuf::from(base)
                .join("BiliCatch")
                .join("openasr")
                .join(binary_name()),
        );
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(directory) = current.parent() {
            candidates.push(directory.join("openasr").join(binary_name()));
        }
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            sidecar_error(
                AppErrorCode::E014,
                "OPENASR_UNAVAILABLE",
                "OpenASR is not installed; set BILICATCH_OPENASR_BIN to its executable",
            )
        })
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "openasr.exe"
    } else {
        "openasr"
    }
}

pub fn parse_openasr_json(
    bytes: &[u8],
    offset_ms: u64,
    language: TranscriptionLanguage,
) -> Result<TranscriberOutput, AppError> {
    let document: OpenAsrDocument = serde_json::from_slice(bytes).map_err(|_| {
        sidecar_error(
            AppErrorCode::E014,
            "SIDECAR_OUTPUT_INVALID",
            "The OpenASR engine returned invalid output",
        )
    })?;

    let mut segments = Vec::new();
    for segment in document.segments {
        if !segment.start.is_finite() || !segment.end.is_finite() {
            continue;
        }
        let text = segment.text.trim();
        // 纯标点片段在字幕里没有意义，直接丢弃。
        if !text.chars().any(is_meaningful) {
            continue;
        }
        let start_ms = offset_ms.saturating_add((segment.start.max(0.0) * 1000.0).round() as u64);
        let end_ms = offset_ms
            .saturating_add((segment.end.max(segment.start) * 1000.0).round() as u64);
        segments.push(TranscriptSegment {
            index: segments.len() as u32,
            start_ms,
            end_ms,
            text: text.into(),
            ..TranscriptSegment::default()
        });
    }

    if segments.is_empty() {
        let text = document.text.trim();
        if text.is_empty() {
            return Err(sidecar_error(
                AppErrorCode::E015,
                "NO_SPEECH_DETECTED",
                "No speech was detected in the media",
            ));
        }
        segments.push(TranscriptSegment {
            index: 0,
            start_ms: offset_ms,
            end_ms: offset_ms,
            text: text.into(),
            ..TranscriptSegment::default()
        });
    }

    Ok(TranscriberOutput {
        detected_language: detected_language(language).into(),
        segments,
    })
}

fn is_meaningful(character: char) -> bool {
    character.is_alphanumeric() || !character.is_ascii_punctuation() && !is_cjk_punctuation(character)
}

fn is_cjk_punctuation(character: char) -> bool {
    matches!(
        character,
        '，' | '。' | '！' | '？' | '；' | '：' | '、' | '“' | '”' | '‘' | '’' | '…' | '—'
    )
}

fn detected_language(language: TranscriptionLanguage) -> &'static str {
    match language {
        TranscriptionLanguage::En => "en",
        TranscriptionLanguage::Ja => "ja",
        TranscriptionLanguage::Ko => "ko",
        _ => "zh",
    }
}

#[derive(Debug, Deserialize)]
struct OpenAsrDocument {
    #[serde(default)]
    text: String,
    #[serde(default)]
    segments: Vec<OpenAsrSegment>,
}

#[derive(Debug, Deserialize)]
struct OpenAsrSegment {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    text: String,
}

pub struct OpenAsrTranscriber {
    binary: PathBuf,
}

impl OpenAsrTranscriber {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            binary: resolve_openasr_binary()?,
        })
    }

    #[cfg(test)]
    fn with_binary(binary: PathBuf) -> Self {
        Self { binary }
    }
}

#[async_trait]
impl Transcriber for OpenAsrTranscriber {
    async fn transcribe(
        &self,
        request: TranscriberRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<TranscriberOutput, AppError> {
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
                "OpenASR runs on the bundled CPU runtime",
            ));
        }

        let mut command = Command::new(&self.binary);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        let mut child = command
            .args(build_openasr_args(&request))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_UNAVAILABLE",
                    "The OpenASR engine could not be started",
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

        let status = loop {
            if control.state() == ProcessControlState::CancelRequested {
                let _ = child.kill().await;
                let _ = child.wait().await;
                stdout_reader.abort();
                stderr_reader.abort();
                return Err(sidecar_error(
                    AppErrorCode::E014,
                    "SIDECAR_CANCELLED",
                    "Transcription was cancelled",
                ));
            }
            match child.try_wait().map_err(|_| pipe_error())? {
                Some(status) => break status,
                None => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        };
        let stdout = stdout_reader
            .await
            .map_err(|_| pipe_error())?
            .map_err(|_| pipe_error())?;
        let stderr = stderr_reader
            .await
            .map_err(|_| pipe_error())?
            .map_err(|_| pipe_error())?;
        if !status.success() {
            let diagnostics = String::from_utf8_lossy(&stderr[..stderr.len().min(8 * 1024)]);
            let details = if diagnostics.contains("integrity verification") {
                "OPENASR_BACKEND_INTEGRITY"
            } else if diagnostics.contains("memory") {
                "OPENASR_MEMORY"
            } else {
                "SIDECAR_FAILED"
            };
            return Err(sidecar_error(
                AppErrorCode::E014,
                details,
                "The OpenASR engine failed",
            ));
        }
        progress(1.0);
        parse_openasr_json(&stdout, request.offset_ms, request.language)
    }
}

fn pipe_error() -> AppError {
    sidecar_error(
        AppErrorCode::E014,
        "SIDECAR_PIPE_FAILED",
        "The OpenASR output stream is unavailable",
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
    use super::{build_openasr_args, parse_openasr_json, OPENASR_MODEL};
    use crate::models::{GpuMode, TranscriptionLanguage};
    use crate::services::transcription::ports::TranscriberRequest;

    fn request() -> TranscriberRequest {
        TranscriberRequest {
            model_path: "openasr".into(),
            audio_path: "part.wav".into(),
            output_base: "result".into(),
            language: TranscriptionLanguage::Zh,
            gpu_mode: GpuMode::Auto,
            offset_ms: 5_000,
        }
    }

    #[test]
    fn arguments_never_send_a_language_flag() {
        let arguments = build_openasr_args(&request())
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(arguments[0], "transcribe");
        assert!(arguments.contains(&"-m".to_string()));
        assert!(arguments.contains(&OPENASR_MODEL.to_string()));
        assert!(arguments.contains(&"json".to_string()));
        assert!(arguments.contains(&"--offline".to_string()));
        assert!(!arguments.iter().any(|value| value == "-l"));
    }

    #[test]
    fn json_segments_are_offset_and_punctuation_only_cues_dropped() {
        let payload = r#"{"text":"。你好。","segments":[
            {"start":2.19,"end":22.03,"text":"。"},
            {"start":22.03,"end":25.0,"text":"你好。"}
        ]}"#;

        let output = parse_openasr_json(payload.as_bytes(), 5_000, TranscriptionLanguage::Zh).unwrap();

        assert_eq!(output.segments.len(), 1);
        assert_eq!(output.segments[0].text, "你好。");
        assert_eq!(output.segments[0].start_ms, 27_030);
        assert_eq!(output.detected_language, "zh");
    }

    #[test]
    fn empty_transcript_is_reported_as_no_speech() {
        let error = parse_openasr_json(
            br#"{"text":"","segments":[]}"#,
            0,
            TranscriptionLanguage::Zh,
        )
        .unwrap_err();

        assert_eq!(error.code, crate::models::AppErrorCode::E015);
    }
}
