use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
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
/// 模型清单里外部托管模型的前缀，`openasr:<pack id>`。
pub const OPENASR_PACK_PREFIX: &str = "openasr:";
/// OpenASR 自己的 home 与模型目录覆盖项。
pub const OPENASR_HOME_ENV: &str = "OPENASR_HOME";
pub const OPENASR_MODELS_DIR_ENV: &str = "OPENASR_MODELS_DIR";

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

/// 某个外部模型包在本机的安装情况。
///
/// `Unknown` 只表示“探测不了”（没有可用的 home 目录），调用方必须退回旧的
/// “已安装”假设：宁可让转写失败后报 E011，也不能因为探测不到就把用户已经
/// 装好的模型标成未安装、连选都选不了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackState {
    Installed(InstalledPack),
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPack {
    /// 对象存储里的 `.oasr` 包路径。
    pub path: PathBuf,
    /// 包的实际大小，用于在模型列表里显示体积。
    pub size_bytes: u64,
}

/// `models/refs/<pack id>/<quant>.json` 的记录（OpenASR 0.1.4x 实测）。
#[derive(Debug, Deserialize)]
struct PackRef {
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    size_bytes: u64,
}

/// 从清单里的 `openasr:<pack id>` 取出 pack id。
pub fn pack_id_from_manifest(file_name: &str) -> Option<&str> {
    file_name
        .strip_prefix(OPENASR_PACK_PREFIX)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// OpenASR 的 home：`OPENASR_HOME` 优先，否则 `%USERPROFILE%`/`$HOME` 下的 `.openasr`。
pub fn openasr_home() -> Option<PathBuf> {
    if let Some(value) = non_empty_env(OPENASR_HOME_ENV) {
        return Some(PathBuf::from(value));
    }
    let profile = non_empty_env("USERPROFILE").or_else(|| non_empty_env("HOME"))?;
    Some(PathBuf::from(profile).join(".openasr"))
}

/// 模型包目录：`OPENASR_MODELS_DIR` 优先，否则 `<home>/models`。
pub fn openasr_models_directory() -> Option<PathBuf> {
    if let Some(value) = non_empty_env(OPENASR_MODELS_DIR_ENV) {
        return Some(PathBuf::from(value));
    }
    Some(openasr_home()?.join("models"))
}

fn non_empty_env(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

/// 只读探测：该 pack id 是否已经由 OpenASR 自己装好。
pub fn pack_state(pack_id: &str) -> PackState {
    openasr_models_directory().map_or(PackState::Unknown, |directory| {
        pack_state_at(&directory, pack_id)
    })
}

/// 不依赖环境变量的版本，便于测试。
///
/// 布局（OpenASR 0.1.41 实测）：
/// ```text
/// <models>/refs/<pack id>/<quant>.json   -> { model_id, quant, path, size_bytes, sha256 }
/// <models>/objects/sha256/<sha>/content  -> 真正的 .oasr 包
/// ```
/// 只有 refs 记录指向的文件真实存在且大小吻合才算装好；对象文件被删掉、
/// 或者只有 refs 残留，都按“没装”处理，让用户重装而不是继续失败。
pub fn pack_state_at(models_dir: &Path, pack_id: &str) -> PackState {
    if !models_dir.is_dir() {
        return PackState::Missing;
    }
    if unexpected_layout(models_dir) {
        return PackState::Unknown;
    }
    let Ok(records) = fs::read_dir(models_dir.join("refs").join(pack_id)) else {
        return PackState::Missing;
    };
    for record in records.flatten() {
        let record_path = record.path();
        if record_path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&record_path) else {
            continue;
        };
        let Ok(reference) = serde_json::from_slice::<PackRef>(&bytes) else {
            continue;
        };
        let model_id = reference.model_id.trim();
        if !model_id.is_empty() && model_id != pack_id {
            continue;
        }
        let pack_path = PathBuf::from(reference.path.trim());
        let Ok(metadata) = fs::metadata(&pack_path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        if reference.size_bytes > 0 && metadata.len() != reference.size_bytes {
            continue;
        }
        return PackState::Installed(InstalledPack {
            path: pack_path,
            size_bytes: metadata.len(),
        });
    }
    PackState::Missing
}

/// `models` 目录存在、但没有认识的 `refs/` 子目录，说明是别的 OpenASR 版本
/// 或别的目录约定：这时给不出“装了没装”的结论，绝不能猜成“没装”，
/// 否则用户已经能用的模型会在界面上被锁死。
fn unexpected_layout(models_dir: &Path) -> bool {
    models_dir.is_dir()
        && !models_dir.join("refs").is_dir()
        && fs::read_dir(models_dir).is_ok_and(|mut entries| entries.any(|entry| entry.is_ok()))
}

/// `openasr pull` 的下载进度行：`Downloaded 25179414/63310208 bytes`。
pub fn parse_download_line(line: &str) -> Option<(u64, u64)> {
    let rest = line.trim().strip_prefix("Downloaded ")?;
    let (downloaded, total) = rest.split_once('/')?;
    let total = total.split_whitespace().next()?;
    Some((downloaded.trim().parse().ok()?, total.parse().ok()?))
}

/// 安装阶段：下载 / 校验 / 已装好。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullStage {
    Downloading,
    Verifying,
    Installed,
}

pub fn parse_pull_stage(line: &str) -> Option<PullStage> {
    let line = line.trim();
    if line.starts_with("Verifying") {
        Some(PullStage::Verifying)
    } else if line.starts_with("Installed ") {
        Some(PullStage::Installed)
    } else {
        None
    }
}

/// 安装包时固定走 hf-mirror：国内直连 huggingface 会一直重试、看起来像卡死，
/// `auto` 在这种网络下也不会自己切到镜像（实测 3 分钟没有任何字节进度）。
pub fn build_pull_args(pack_id: &str) -> Vec<OsString> {
    vec![
        "pull".into(),
        pack_id.into(),
        "--source".into(),
        "hf-mirror".into(),
    ]
}

/// 用 OpenASR 自己的 CLI 安装模型包（`openasr pull <pack>`），并把进度回调出去。
///
/// BiliCatch 不自己下载 `.oasr`：包的落盘位置、校验、refs 记录都归 OpenASR 管，
/// 复制一份反而会让 `openasr list` 与转录路径对不上。
pub async fn pull_pack(
    binary: &Path,
    pack_id: &str,
    is_cancelled: std::sync::Arc<dyn Fn() -> bool + Send + Sync>,
    on_progress: std::sync::Arc<dyn Fn(u64, u64) + Send + Sync>,
    on_stage: std::sync::Arc<dyn Fn(PullStage) + Send + Sync>,
) -> Result<(), AppError> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut command = Command::new(binary);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .args(build_pull_args(pack_id))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            sidecar_error(
                AppErrorCode::E012,
                "OPENASR_PULL_UNAVAILABLE",
                "The OpenASR CLI could not be started",
            )
        })?;
    let stdout = child.stdout.take().ok_or_else(pull_pipe_error)?;
    let stderr = child.stderr.take().ok_or_else(pull_pipe_error)?;

    let last_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let mut readers = Vec::new();
    for stream in [Box::new(stdout) as Box<dyn tokio::io::AsyncRead + Unpin + Send>, Box::new(stderr)] {
        let progress = on_progress.clone();
        let stage = on_stage.clone();
        let tail = last_lines.clone();
        readers.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some((downloaded, total)) = parse_download_line(&line) {
                    progress(downloaded, total);
                }
                if let Some(event) = parse_pull_stage(&line) {
                    stage(event);
                }
                if let Ok(mut tail) = tail.lock() {
                    tail.push(line);
                    if tail.len() > 40 {
                        tail.remove(0);
                    }
                }
            }
        }));
    }

    let status = loop {
        if is_cancelled() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            for reader in readers {
                reader.abort();
            }
            return Err(sidecar_error(
                AppErrorCode::E012,
                "MODEL_DOWNLOAD_CANCELLED",
                "The OpenASR model download was cancelled",
            ));
        }
        match child.try_wait().map_err(|_| pull_pipe_error())? {
            Some(status) => break status,
            None => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    };
    for reader in readers {
        let _ = reader.await;
    }
    if status.success() {
        return Ok(());
    }
    let diagnostics = last_lines
        .lock()
        .map(|lines| lines.join("\n"))
        .unwrap_or_default();
    Err(AppError {
        code: AppErrorCode::E012,
        message: "The OpenASR model pack could not be downloaded".into(),
        details: Some(format!(
            "OPENASR_PULL_FAILED: {}",
            diagnostics.chars().take(400).collect::<String>()
        )),
    })
}

fn pull_pipe_error() -> AppError {
    sidecar_error(
        AppErrorCode::E012,
        "OPENASR_PULL_PIPE_FAILED",
        "The OpenASR download stream is unavailable",
    )
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
            return Err(classify_failure(&stderr));
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

/// 把 OpenASR 的 stderr 归类成用户能自己处理的失败原因。
///
/// `transcribe --offline` 在模型包缺失时会以退出码 4 结束并输出
/// `Model 'firered2-llm' is not installed and OpenASR is offline`。
/// 这是“模型没装”，不是“引擎坏了”：必须回 E011，否则界面只会显示
/// 笼统的“本地转写引擎运行失败”，用户无从下手。
fn classify_failure(raw: &[u8]) -> AppError {
    let diagnostics = String::from_utf8_lossy(&raw[..raw.len().min(8 * 1024)]);
    let lowered = diagnostics.to_ascii_lowercase();
    if lowered.contains("not installed") || lowered.contains("openasr pull") {
        return sidecar_error(
            AppErrorCode::E011,
            "OPENASR_MODEL_MISSING",
            "The OpenASR model pack is not installed",
        );
    }
    let details = if lowered.contains("integrity verification") {
        "OPENASR_BACKEND_INTEGRITY"
    } else if lowered.contains("memory") {
        "OPENASR_MEMORY"
    } else {
        "SIDECAR_FAILED"
    };
    sidecar_error(AppErrorCode::E014, details, "The OpenASR engine failed")
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
    use super::{
        build_openasr_args, build_pull_args, classify_failure, pack_id_from_manifest, pack_state_at,
        parse_download_line, parse_openasr_json, parse_pull_stage, InstalledPack, PackState,
        PullStage, OPENASR_MODEL,
    };
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

    #[test]
    fn missing_model_pack_is_reported_as_a_missing_model_not_a_broken_engine() {
        let diagnostics = b"Error: Model 'firered2-llm' is not installed and OpenASR is offline.\n\
Run: openasr pull firered2-llm\n";

        let error = classify_failure(diagnostics);

        assert_eq!(error.code, crate::models::AppErrorCode::E011);
        assert_eq!(error.details.as_deref(), Some("OPENASR_MODEL_MISSING"));
    }

    #[test]
    fn other_engine_failures_stay_on_the_engine_error_code() {
        let integrity = classify_failure(b"backend integrity verification failed");
        assert_eq!(integrity.details.as_deref(), Some("OPENASR_BACKEND_INTEGRITY"));

        let memory = classify_failure(b"out of memory while loading");
        assert_eq!(memory.details.as_deref(), Some("OPENASR_MEMORY"));

        let generic = classify_failure(b"thread panicked");
        assert_eq!(generic.code, crate::models::AppErrorCode::E014);
        assert_eq!(generic.details.as_deref(), Some("SIDECAR_FAILED"));
    }

    /// 真实布局（openasr 0.1.41 + `openasr pull whisper-tiny` 实测）：
    /// `<models>/refs/<id>/<quant>.json` 指向 `<models>/objects/sha256/<sha>/content`。
    fn fake_openasr_home(root: &std::path::Path, pack_id: &str, pack_size: usize) -> (std::path::PathBuf, std::path::PathBuf) {
        let models = root.join("models");
        let object = models
            .join("objects")
            .join("sha256")
            .join("13346d7bd47d349ae9e35be5cc8595997be0c2d2a5bbe41b4e3ef1c60a98ca17")
            .join("content");
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        std::fs::write(&object, vec![0_u8; pack_size]).unwrap();
        let refs = models.join("refs").join(pack_id);
        std::fs::create_dir_all(&refs).unwrap();
        let record = serde_json::json!({
            "model_id": pack_id,
            "display_name": "Test pack",
            "quant": "q8_0",
            "suffix": "q8",
            "pull": format!("{pack_id}:q8"),
            "path": object.to_string_lossy(),
            "sha256": "13346d7bd47d349ae9e35be5cc8595997be0c2d2a5bbe41b4e3ef1c60a98ca17",
            "size_bytes": pack_size,
        });
        std::fs::write(refs.join("q8_0.json"), record.to_string()).unwrap();
        (models, object)
    }

    #[test]
    fn installed_pack_is_detected_from_the_refs_record() {
        let root = tempfile::tempdir().unwrap();
        let (models, object) = fake_openasr_home(root.path(), "whisper-tiny", 32);

        assert_eq!(
            pack_state_at(&models, "whisper-tiny"),
            PackState::Installed(InstalledPack {
                path: object,
                size_bytes: 32
            })
        );
        // 仓库里没有别的模型
        assert_eq!(pack_state_at(&models, "firered2-llm"), PackState::Missing);
        // 连 models 目录都没有 = 从没装过任何包
        assert_eq!(pack_state_at(&root.path().join("missing"), "firered2-llm"), PackState::Missing);
    }

    #[test]
    fn stale_ref_without_the_pack_object_counts_as_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let (models, object) = fake_openasr_home(root.path(), "whisper-tiny", 32);
        std::fs::remove_file(&object).unwrap();

        assert_eq!(pack_state_at(&models, "whisper-tiny"), PackState::Missing);
    }

    #[test]
    fn pack_size_mismatch_counts_as_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let (models, object) = fake_openasr_home(root.path(), "whisper-tiny", 32);
        std::fs::write(&object, vec![0_u8; 8]).unwrap();

        assert_eq!(pack_state_at(&models, "whisper-tiny"), PackState::Missing);
    }

    #[test]
    fn a_models_directory_with_an_unknown_layout_is_not_reported_as_missing() {
        let root = tempfile::tempdir().unwrap();
        let models = root.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        // 别的 OpenASR 版本可能把 .oasr 直接放在 models 下：这种布局不认识，
        // 只能回 Unknown，让调用方保持“已安装”的乐观假设。
        std::fs::write(models.join("firered2-llm-q4_k.oasr"), b"pack").unwrap();

        assert_eq!(pack_state_at(&models, "firered2-llm"), PackState::Unknown);
    }

    #[test]
    fn an_empty_models_directory_means_nothing_is_installed() {
        let root = tempfile::tempdir().unwrap();
        let models = root.path().join("models");
        std::fs::create_dir_all(&models).unwrap();

        assert_eq!(pack_state_at(&models, "firered2-llm"), PackState::Missing);
    }

    #[test]
    fn manifest_pack_prefix_is_parsed_leniently() {
        assert_eq!(pack_id_from_manifest("openasr:firered2-llm"), Some("firered2-llm"));
        assert_eq!(pack_id_from_manifest("openasr: "), None);
        assert_eq!(pack_id_from_manifest("ggml-base.bin"), None);
    }

    #[test]
    fn parses_openasr_pull_download_and_stage_lines() {
        // 实测输出：`Downloaded 25179414/63310208 bytes`
        assert_eq!(
            parse_download_line("Downloaded 25179414/63310208 bytes"),
            Some((25_179_414, 63_310_208))
        );
        assert_eq!(
            parse_download_line("Downloaded 5097451168/5097451168 bytes"),
            Some((5_097_451_168, 5_097_451_168))
        );
        assert_eq!(parse_download_line("Installed whisper-tiny:q8 at C:\\x"), None);

        assert_eq!(
            parse_pull_stage("Verifying firered2-llm:q4"),
            Some(PullStage::Verifying)
        );
        assert_eq!(
            parse_pull_stage("Installed firered2-llm:q4 at C:\\models\\content"),
            Some(PullStage::Installed)
        );
        assert_eq!(parse_pull_stage("Downloaded 1/2 bytes"), None);
    }

    #[test]
    fn pull_args_pin_the_reachable_mirror() {
        let args = build_pull_args("firered2-llm")
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(args, vec!["pull", "firered2-llm", "--source", "hf-mirror"]);
    }
}
