use serde::{Deserialize, Serialize};

use super::{AsrModelId, AudioFormat, GpuMode, TranscriptFormat, TranscriptionLanguage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoQualityId {
    #[serde(rename = "16")]
    P360,
    #[serde(rename = "32")]
    P480,
    #[serde(rename = "64")]
    P720,
    #[serde(rename = "80")]
    P1080,
    #[serde(rename = "112")]
    P1080Plus,
    #[serde(rename = "120")]
    P4k,
    #[serde(rename = "125")]
    Hdr,
    #[serde(rename = "127")]
    P8k,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseBehavior {
    MinimizeToTray,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsValues {
    pub download_directory: String,
    pub temporary_directory: String,
    pub max_concurrent_tasks: u8,
    pub connections_per_task: u8,
    pub default_video_quality: VideoQualityId,
    pub default_audio_format: AudioFormat,
    pub theme: ThemePreference,
    pub locale: String,
    pub notify_on_complete: bool,
    pub completion_sound: bool,
    pub close_behavior: CloseBehavior,
    pub auto_check_updates: bool,
    pub default_asr_model: AsrModelId,
    pub default_transcription_language: TranscriptionLanguage,
    pub default_transcript_format: TranscriptFormat,
    pub transcription_timestamps: bool,
    pub transcription_punctuation: bool,
    pub prefer_platform_subtitles: bool,
    pub auto_transcribe_downloads: bool,
    pub max_concurrent_transcriptions: u8,
    pub asr_model_directory: String,
    pub asr_gpu_mode: GpuMode,
    pub asr_model_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDocument {
    pub schema_version: u16,
    pub revision: u64,
    pub values: SettingsValues,
}

pub type SettingsSnapshot = SettingsDocument;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", content = "value", rename_all = "camelCase")]
pub enum SettingsPatch {
    DownloadDirectory(String),
    TemporaryDirectory(String),
    MaxConcurrentTasks(u8),
    ConnectionsPerTask(u8),
    DefaultVideoQuality(VideoQualityId),
    DefaultAudioFormat(AudioFormat),
    Theme(ThemePreference),
    Locale(String),
    NotifyOnComplete(bool),
    CompletionSound(bool),
    CloseBehavior(CloseBehavior),
    AutoCheckUpdates(bool),
    DefaultAsrModel(AsrModelId),
    DefaultTranscriptionLanguage(TranscriptionLanguage),
    DefaultTranscriptFormat(TranscriptFormat),
    TranscriptionTimestamps(bool),
    TranscriptionPunctuation(bool),
    PreferPlatformSubtitles(bool),
    AutoTranscribeDownloads(bool),
    MaxConcurrentTranscriptions(u8),
    AsrModelDirectory(String),
    AsrGpuMode(GpuMode),
    AsrModelSource(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSettingRequest {
    pub patch: SettingsPatch,
}
