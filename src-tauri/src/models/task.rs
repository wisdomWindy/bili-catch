use serde::{Deserialize, Serialize};

use super::{AppError, AudioFormat, VideoCodec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadMode {
    VideoAudio,
    VideoOnly,
    AudioOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Queued,
    Downloading,
    Paused,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskControlRequest {
    None,
    PauseRequested,
    ResumeRequested,
    CancelRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenTarget {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskDraft {
    pub canonical_url: String,
    pub bvid: String,
    pub cid: u64,
    pub page: u32,
    pub part_title: String,
    pub video_title: String,
    pub part_count: u32,
    pub mode: DownloadMode,
    pub output_dir: String,
    pub quality_id: Option<String>,
    pub codec: Option<VideoCodec>,
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub file_name: String,
    pub output_dir: String,
    pub output_path: Option<String>,
    pub bvid: String,
    pub cid: u64,
    pub page: u32,
    pub part_title: String,
    pub mode: DownloadMode,
    pub quality_id: Option<String>,
    pub codec: Option<VideoCodec>,
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate_id: Option<String>,
    pub status: TaskStatus,
    pub control_request: TaskControlRequest,
    pub progress_percent: u8,
    pub bytes_downloaded: String,
    pub total_bytes: Option<String>,
    pub speed_bytes_per_second: String,
    pub eta_seconds: Option<u64>,
    pub automatic_retry_count: u8,
    pub next_retry_at: Option<String>,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDownloadTasksRequest {
    pub request_id: String,
    pub drafts: Vec<DownloadTaskDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlDownloadTaskRequest {
    pub task_id: String,
    pub action: TaskAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDownloadTaskRequest {
    pub task_id: String,
    pub target: OpenTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListSnapshot {
    pub sequence: u64,
    pub capacity: u16,
    pub tasks: Vec<DownloadTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTasksResult {
    pub sequence: u64,
    pub capacity: u16,
    pub tasks: Vec<DownloadTask>,
    pub reused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskFailure {
    pub task_id: String,
    pub error: AppError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskResult {
    pub sequence: u64,
    pub affected_task_ids: Vec<String>,
    pub failures: Vec<BatchTaskFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearTasksResult {
    pub sequence: u64,
    pub removed_task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgressEvent {
    pub sequence: u64,
    pub task: DownloadTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRemovedEvent {
    pub sequence: u64,
    pub task_id: String,
}
