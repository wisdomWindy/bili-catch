use serde::{Deserialize, Serialize};

use super::{
    AppError, AsrModelId, AudioFormat, ContainerKind, TranscriptFormat, TranscriptionLanguage,
    VideoCodec, VideoSource,
};

fn default_source() -> VideoSource {
    VideoSource::Ugc
}

fn default_container_kind() -> ContainerKind {
    ContainerKind::Video
}

fn default_group_index() -> u32 {
    1
}

fn default_part_count() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionPreset {
    pub model_id: AsrModelId,
    pub language: TranscriptionLanguage,
    pub format: TranscriptFormat,
    pub timestamps: bool,
    pub punctuation: bool,
}

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
    #[serde(default = "default_source")]
    pub source: VideoSource,
    #[serde(default = "default_container_kind")]
    pub container_kind: ContainerKind,
    #[serde(default)]
    pub ep_id: Option<u64>,
    #[serde(default = "default_group_index")]
    pub group_index: u32,
    pub mode: DownloadMode,
    pub output_dir: String,
    pub quality_id: Option<String>,
    pub codec: Option<VideoCodec>,
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate_id: Option<String>,
    #[serde(default)]
    pub auto_transcribe: bool,
    #[serde(default)]
    pub transcription_preset: Option<TranscriptionPreset>,
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
    #[serde(default = "default_part_count")]
    pub part_count: u32,
    #[serde(default = "default_source")]
    pub source: VideoSource,
    #[serde(default = "default_container_kind")]
    pub container_kind: ContainerKind,
    #[serde(default)]
    pub ep_id: Option<u64>,
    #[serde(default = "default_group_index")]
    pub group_index: u32,
    pub mode: DownloadMode,
    pub quality_id: Option<String>,
    pub codec: Option<VideoCodec>,
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate_id: Option<String>,
    #[serde(default)]
    pub auto_transcribe: bool,
    #[serde(default)]
    pub transcription_preset: Option<TranscriptionPreset>,
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

#[cfg(test)]
mod tests {
    use super::{DownloadTask, DownloadTaskDraft, VideoSource};

    #[test]
    fn legacy_task_json_defaults_the_unit_fields() {
        let value = serde_json::json!({
            "id": "batch-1:0",
            "revision": 1,
            "createdAt": "2026-09-10T00:00:00Z",
            "updatedAt": "2026-09-10T00:00:00Z",
            "fileName": "P1.mp4",
            "outputDir": "Downloads",
            "outputPath": null,
            "bvid": "BV1xx411c7BF",
            "cid": 1001,
            "page": 1,
            "partTitle": "P1",
            "mode": "video-only",
            "qualityId": "80",
            "codec": "avc",
            "audioFormat": null,
            "audioBitrateId": null,
            "autoTranscribe": false,
            "transcriptionPreset": null,
            "status": "queued",
            "controlRequest": "none",
            "progressPercent": 0,
            "bytesDownloaded": "0",
            "totalBytes": null,
            "speedBytesPerSecond": "0",
            "etaSeconds": null,
            "automaticRetryCount": 0,
            "nextRetryAt": null,
            "error": null
        });

        let task: DownloadTask =
            serde_json::from_value(value).expect("legacy task must deserialize");
        assert_eq!(task.source, VideoSource::Ugc);
        assert_eq!(task.ep_id, None);
        assert_eq!(task.group_index, 1);
        assert_eq!(task.part_count, 1);
    }

    #[test]
    fn legacy_draft_json_defaults_the_unit_fields() {
        let value = serde_json::json!({
            "canonicalUrl": "https://www.bilibili.com/video/BV1xx411c7BF",
            "bvid": "BV1xx411c7BF",
            "cid": 1,
            "page": 1,
            "partTitle": "P1",
            "videoTitle": "Example",
            "partCount": 1,
            "mode": "video-only",
            "outputDir": "Downloads",
            "qualityId": "80",
            "codec": "avc",
            "audioFormat": null,
            "audioBitrateId": null,
            "autoTranscribe": false,
            "transcriptionPreset": null
        });

        let draft: DownloadTaskDraft =
            serde_json::from_value(value).expect("legacy draft must deserialize");
        assert_eq!(draft.source, VideoSource::Ugc);
        assert_eq!(draft.ep_id, None);
        assert_eq!(draft.group_index, 1);
    }
}
