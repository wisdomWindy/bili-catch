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
    #[serde(default, rename = "baseUrl")]
    pub base_url: String,
    #[serde(default, rename = "base_url")]
    pub snake_base_url: String,
    #[serde(default, rename = "backupUrl")]
    pub backup_url: Vec<String>,
    #[serde(default, rename = "backup_url")]
    pub snake_backup_url: Vec<String>,
    #[serde(default)]
    pub bandwidth: u64,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default, rename = "mime_type")]
    pub snake_mime_type: String,
    #[serde(default)]
    pub codecs: String,
}

impl RawVideoStream {
    pub(crate) fn resolved_base_url(&self) -> &str {
        if self.base_url.is_empty() {
            &self.snake_base_url
        } else {
            &self.base_url
        }
    }

    pub(crate) fn resolved_backup_urls(&self) -> &[String] {
        if self.backup_url.is_empty() {
            &self.snake_backup_url
        } else {
            &self.backup_url
        }
    }

    pub(crate) fn resolved_mime_type(&self) -> &str {
        if self.mime_type.is_empty() {
            &self.snake_mime_type
        } else {
            &self.mime_type
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawAudioStream {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "baseUrl")]
    pub base_url: String,
    #[serde(default, rename = "base_url")]
    pub snake_base_url: String,
    #[serde(default, rename = "backupUrl")]
    pub backup_url: Vec<String>,
    #[serde(default, rename = "backup_url")]
    pub snake_backup_url: Vec<String>,
    pub bandwidth: u64,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default, rename = "mime_type")]
    pub snake_mime_type: String,
    #[serde(default)]
    pub codecs: String,
}

impl RawAudioStream {
    pub(crate) fn resolved_base_url(&self) -> &str {
        if self.base_url.is_empty() {
            &self.snake_base_url
        } else {
            &self.base_url
        }
    }

    pub(crate) fn resolved_backup_urls(&self) -> &[String] {
        if self.backup_url.is_empty() {
            &self.snake_backup_url
        } else {
            &self.backup_url
        }
    }

    pub(crate) fn resolved_mime_type(&self) -> &str {
        if self.mime_type.is_empty() {
            &self.snake_mime_type
        } else {
            &self.mime_type
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RawAudioStream, RawVideoStream};

    #[test]
    fn accepts_bilibili_streams_with_both_field_namings() {
        let video: RawVideoStream = serde_json::from_str(
            r#"{
            "id":32,"baseUrl":"https://camel/video","base_url":"https://snake/video",
            "backupUrl":["https://camel/backup"],"backup_url":["https://snake/backup"],
            "bandwidth":1000,"mimeType":"video/mp4","mime_type":"video/webm","codecs":"avc1.64001f"
        }"#,
        )
        .unwrap();
        assert_eq!(video.resolved_base_url(), "https://camel/video");
        assert_eq!(video.resolved_backup_urls(), ["https://camel/backup"]);
        assert_eq!(video.resolved_mime_type(), "video/mp4");

        let audio: RawAudioStream = serde_json::from_str(
            r#"{
            "id":30216,"base_url":"https://snake/audio","backup_url":[],
            "bandwidth":64000,"mime_type":"audio/mp4","codecs":"mp4a.40.2"
        }"#,
        )
        .unwrap();
        assert_eq!(audio.resolved_base_url(), "https://snake/audio");
        assert_eq!(audio.resolved_mime_type(), "audio/mp4");
    }
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
