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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoSource {
    Ugc,
    Bangumi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerKind {
    Video,
    Season,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
    Playable,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailableReason {
    LoginRequired,
    VipRequired,
    Paid,
    RegionLocked,
    DownloadDisabled,
    Preview,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseContainer {
    pub kind: ContainerKind,
    pub id: String,
    pub title: String,
    pub unit_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPart {
    pub cid: u64,
    /// bvid of the archive this unit belongs to; differs per collection member.
    pub bvid: String,
    /// Bangumi episode id; `None` for UGC units.
    pub ep_id: Option<u64>,
    /// 1-based part index inside its container.
    pub page: u32,
    /// Number of downloadable parts inside the container.
    pub part_count: u32,
    /// 1-based container order inside the parse result.
    pub group_index: u32,
    pub title: String,
    pub duration_seconds: u64,
    pub availability: Availability,
    pub unavailable_reason: Option<UnavailableReason>,
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
    pub bvid: Option<String>,
    pub aid: Option<u64>,
    pub source: VideoSource,
    pub container: Option<ParseContainer>,
    pub title: String,
    pub owner_name: String,
    pub cover_url: String,
    pub duration_seconds: u64,
    pub requested_page: Option<u32>,
    /// Unit that should be selected first; `None` falls back to `requested_page`.
    pub requested_cid: Option<u64>,
    pub parts: Vec<VideoPart>,
    pub qualities: Vec<MediaOption>,
    pub codecs: Vec<VideoCodec>,
    pub video_variants: Vec<VideoVariant>,
    pub audio_formats: Vec<AudioFormat>,
    pub audio_bitrates: Vec<MediaOption>,
    pub audio_capability: AudioCapability,
}
