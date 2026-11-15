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
    FireRedAsr2,
    FireRedAsr2Aed,
    FireRedAsr2Llm,
    Qwen3Asr,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<u16>,
    #[serde(default)]
    pub has_video_track: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embedded_subtitles: Vec<EmbeddedSubtitleTrack>,
    #[serde(default)]
    pub hard_subtitle_detected: bool,
}

impl MediaSourceInfo {
    /// 纯音频源没有画面，硬字幕 OCR 与内嵌字幕抽取都不可用。
    pub fn supports_visual_sources(&self) -> bool {
        self.has_video_track
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedSubtitleTrack {
    pub index: u32,
    pub codec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscriptOrigin {
    EmbeddedSubtitle,
    PlatformSubtitle,
    #[default]
    Asr,
    HardSubtitleOcr,
    Fused,
}

impl TranscriptOrigin {
    pub fn is_asr(&self) -> bool {
        matches!(self, TranscriptOrigin::Asr)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentAlternate {
    pub origin: TranscriptOrigin,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "TranscriptOrigin::is_asr")]
    pub origin: TranscriptOrigin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternates: Vec<SegmentAlternate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineReport {
    #[serde(default)]
    pub sources_used: Vec<TranscriptOrigin>,
    #[serde(default)]
    pub ocr_ms: u64,
    #[serde(default)]
    pub asr_ms: u64,
    #[serde(default)]
    pub conflict_count: u32,
    #[serde(default)]
    pub low_confidence_count: u32,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub model_version: String,
    #[serde(default)]
    pub review_recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<PipelineReport>,
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

#[cfg(test)]
mod tests {
    use super::{AsrModelId, TranscriptOrigin, TranscriptSegment};

    #[test]
    fn legacy_segment_json_defaults_to_asr_origin_and_no_confidence() {
        let segment: TranscriptSegment =
            serde_json::from_str(r#"{"index":0,"startMs":0,"endMs":10,"text":"hi"}"#)
                .expect("legacy segment should deserialize");

        assert_eq!(segment.origin, TranscriptOrigin::Asr);
        assert_eq!(segment.confidence, None);
        assert!(segment.alternates.is_empty());
    }

    #[test]
    fn asr_origin_is_omitted_from_serialized_output() {
        let segment = TranscriptSegment {
            index: 0,
            start_ms: 0,
            end_ms: 10,
            text: "hi".into(),
            ..TranscriptSegment::default()
        };

        let value = serde_json::to_value(&segment).expect("segment should serialize");
        assert!(value.get("origin").is_none());
        assert!(value.get("confidence").is_none());
        assert!(value.get("alternates").is_none());
    }

    #[test]
    fn non_asr_origin_round_trips_through_kebab_case() {
        let segment = TranscriptSegment {
            index: 0,
            start_ms: 0,
            end_ms: 10,
            text: "hi".into(),
            origin: TranscriptOrigin::HardSubtitleOcr,
            confidence: Some(0.8),
            ..TranscriptSegment::default()
        };

        let value = serde_json::to_value(&segment).expect("segment should serialize");
        assert_eq!(value["origin"], "hard-subtitle-ocr");
        assert_eq!(
            serde_json::from_value::<TranscriptSegment>(value)
                .expect("segment should deserialize")
                .origin,
            TranscriptOrigin::HardSubtitleOcr
        );
    }

    #[test]
    fn fire_red_asr2_uses_the_frontend_model_id() {
        let model: AsrModelId =
            serde_json::from_str("\"fire-red-asr2\"").expect("FireRedASR2 model id");

        assert_eq!(
            serde_json::to_string(&model).expect("serialized model id"),
            "\"fire-red-asr2\""
        );
    }
}
