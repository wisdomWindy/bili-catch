use serde::{Deserialize, Serialize};

use super::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AsrModelId {
    Tiny,
    Base,
    Small,
    Medium,
    LargeV3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranscriptionLanguage {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "zh")]
    Zh,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "ko")]
    Ko,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptFormat {
    Txt,
    Srt,
    Vtt,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuMode {
    Auto,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TranscribeTaskStatus {
    Pending,
    Preprocessing,
    LoadingModel,
    Transcribing,
    Processing,
    Saving,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TranscribeTaskAction {
    Cancel,
    Retry,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TranscriptionSource {
    LocalFile {
        path: String,
    },
    DownloadTask {
        task_id: String,
        output_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        bvid: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cid: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceInfo {
    pub file_name: String,
    pub path: String,
    pub duration_seconds: u64,
    pub size_bytes: u64,
    pub media_kind: MediaKind,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOptions {
    pub model_id: AsrModelId,
    pub language: TranscriptionLanguage,
    pub output_format: TranscriptFormat,
    pub timestamps: bool,
    pub punctuation: bool,
    pub output_directory: String,
    pub gpu_mode: GpuMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTranscribeTaskRequest {
    pub source: TranscriptionSource,
    pub options: TranscriptionOptions,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgress {
    pub progress: f64,
    pub processed_seconds: f64,
    pub total_seconds: Option<f64>,
    pub speed: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeTask {
    pub id: String,
    pub revision: u64,
    pub source: TranscriptionSource,
    pub source_info: MediaSourceInfo,
    pub options: TranscriptionOptions,
    pub status: TranscribeTaskStatus,
    pub progress: TranscriptionProgress,
    pub result_id: Option<String>,
    pub output_path: Option<String>,
    pub error: Option<AppError>,
    pub cache_reused: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeTaskListSnapshot {
    pub sequence: u64,
    pub tasks: Vec<TranscribeTask>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTranscribeTaskResult {
    pub task: TranscribeTask,
    pub reused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptResult {
    pub id: String,
    pub task_id: String,
    pub revision: u64,
    pub source_fingerprint: String,
    pub model_id: AsrModelId,
    pub requested_language: TranscriptionLanguage,
    pub detected_language: String,
    pub segments: Vec<TranscriptSegment>,
    pub plain_text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelDownloadState {
    Idle,
    Queued,
    Downloading,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrModel {
    pub id: AsrModelId,
    pub display_name: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub installed: bool,
    pub valid: bool,
    pub is_default: bool,
    pub in_use: bool,
    pub path: Option<String>,
    pub recommended_for: String,
    pub capabilities: Vec<String>,
    pub download_state: ModelDownloadState,
    pub downloaded_bytes: u64,
    pub error: Option<AppError>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrModelListSnapshot {
    pub sequence: u64,
    pub models: Vec<AsrModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgressEvent {
    pub sequence: u64,
    pub task: TranscribeTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionRemovedEvent {
    pub sequence: u64,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgressEvent {
    pub sequence: u64,
    pub model: AsrModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectTranscriptionSourceRequest {
    pub source: TranscriptionSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlTranscribeTaskRequest {
    pub task_id: String,
    pub action: TranscribeTaskAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub model_id: AsrModelId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskIdRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTranscriptEditsRequest {
    pub task_id: String,
    pub expected_revision: u64,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTranscriptRequest {
    pub task_id: String,
    pub result_revision: u64,
    pub format: TranscriptFormat,
    pub timestamps: bool,
    pub output_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTranscriptResult {
    pub output_path: String,
}
