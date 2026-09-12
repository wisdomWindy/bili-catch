use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoCodec {
    Avc,
    Hevc,
    Av1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoVariant {
    pub quality_id: String,
    pub codec: VideoCodec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPart {
    pub cid: u64,
    pub page: u32,
    pub title: String,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaOption {
    pub id: String,
    pub label: String,
    pub requires_login: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCapability {
    pub max_lossy_kbps: Option<u64>,
    pub lossless_available: bool,
    pub hi_res_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseVideoResult {
    pub canonical_url: String,
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub owner_name: String,
    pub cover_url: String,
    pub duration_seconds: u64,
    pub requested_page: Option<u32>,
    pub parts: Vec<VideoPart>,
    pub qualities: Vec<MediaOption>,
    pub codecs: Vec<VideoCodec>,
    pub video_variants: Vec<VideoVariant>,
    pub audio_formats: Vec<AudioFormat>,
    pub audio_bitrates: Vec<MediaOption>,
    pub audio_capability: AudioCapability,
}
