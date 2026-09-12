use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ApiResponse<T> {
    pub code: i64,
    #[serde(default)]
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ViewData {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub pic: String,
    pub duration: u64,
    pub owner: RawOwner,
    #[serde(default)]
    pub pages: Vec<RawPage>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawOwner {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawPage {
    pub cid: u64,
    pub page: u32,
    #[serde(default)]
    pub part: String,
    #[serde(default)]
    pub duration: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PlayData {
    #[serde(default)]
    pub accept_quality: Vec<u32>,
    #[serde(default)]
    pub accept_description: Vec<String>,
    pub dash: Option<RawDash>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawDash {
    #[serde(default)]
    pub video: Vec<RawVideoStream>,
    #[serde(default)]
    pub audio: Vec<RawAudioStream>,
    pub flac: Option<RawFlac>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawFlac {
    #[serde(default)]
    pub display: bool,
    pub audio: Option<RawAudioStream>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawVideoStream {
    pub id: u32,
    #[serde(default, alias = "baseUrl")]
    pub base_url: String,
    #[serde(default, alias = "backupUrl")]
    pub backup_url: Vec<String>,
    #[serde(default)]
    pub bandwidth: u64,
    #[serde(default, alias = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub codecs: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawAudioStream {
    #[serde(default)]
    pub id: u32,
    #[serde(default, alias = "baseUrl")]
    pub base_url: String,
    #[serde(default, alias = "backupUrl")]
    pub backup_url: Vec<String>,
    pub bandwidth: u64,
    #[serde(default, alias = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub codecs: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NavData {
    pub wbi_img: RawWbiImage,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawWbiImage {
    pub img_url: String,
    pub sub_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WbiKey {
    pub mixin_key: String,
}
