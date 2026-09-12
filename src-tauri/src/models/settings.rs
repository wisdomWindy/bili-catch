use serde::{Deserialize, Serialize};

use super::AudioFormat;

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
    pub close_behavior: CloseBehavior,
    pub auto_check_updates: bool,
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
    CloseBehavior(CloseBehavior),
    AutoCheckUpdates(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSettingRequest {
    pub patch: SettingsPatch,
}
