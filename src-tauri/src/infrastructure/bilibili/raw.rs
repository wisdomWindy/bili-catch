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
    #[serde(default)]
    pub ugc_season: Option<RawUgcSeason>,
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
pub(crate) struct RawUgcSeason {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub sections: Vec<RawSeasonSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSeasonSection {
    #[serde(default)]
    pub episodes: Vec<RawSeasonEpisode>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSeasonEpisode {
    #[serde(default)]
    pub bvid: String,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub arc: Option<RawSeasonArc>,
    #[serde(default)]
    pub pages: Vec<RawPage>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSeasonArc {
    #[serde(default)]
    pub duration: u64,
}

/// PGC responses wrap their payload in `result` instead of `data`.
#[derive(Debug, Deserialize)]
pub(crate) struct PgcResponse<T> {
    pub code: i64,
    #[serde(default)]
    pub message: String,
    pub result: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SeasonData {
    #[serde(default)]
    pub season_id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub episodes: Vec<RawEpisode>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawEpisode {
    #[serde(default)]
    pub ep_id: u64,
    #[serde(default)]
    pub aid: u64,
    #[serde(default)]
    pub bvid: String,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub long_title: String,
    /// Milliseconds in the PGC payload.
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub badge: String,
    #[serde(default)]
    pub section_type: u32,
    pub rights: Option<RawEpisodeRights>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawEpisodeRights {
    #[serde(default)]
    pub allow_download: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PlayData {
    #[serde(default)]
    pub accept_quality: Vec<u32>,
    #[serde(default)]
    pub accept_description: Vec<String>,
    #[serde(default)]
    pub v_voucher: Option<serde_json::Value>,
    pub dash: Option<RawDash>,
}

impl PlayData {
    pub(crate) fn ensure_playable(&self) -> Result<(), crate::models::AppError> {
        let playable = self.v_voucher.is_none()
            && self.dash.as_ref().is_some_and(|dash| {
                dash.video
                    .iter()
                    .any(|stream| !stream.resolved_base_url().trim().is_empty())
                    || dash
                        .audio
                        .iter()
                        .any(|stream| !stream.resolved_base_url().trim().is_empty())
                    || dash
                        .flac
                        .as_ref()
                        .filter(|flac| flac.display)
                        .and_then(|flac| flac.audio.as_ref())
                        .is_some_and(|stream| !stream.resolved_base_url().trim().is_empty())
                    || dash
                        .dolby
                        .as_ref()
                        .and_then(|dolby| dolby.audio.as_ref())
                        .is_some_and(|audio| {
                            audio
                                .streams()
                                .iter()
                                .any(|stream| !stream.resolved_base_url().trim().is_empty())
                        })
            });
        if playable {
            Ok(())
        } else {
            Err(crate::models::AppError::new(
                crate::models::AppErrorCode::E004,
                "Bilibili returned no playable media streams",
            ))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawDash {
    #[serde(default)]
    pub video: Vec<RawVideoStream>,
    #[serde(default)]
    pub audio: Vec<RawAudioStream>,
    pub flac: Option<RawFlac>,
    /// 杜比全景声轨（30250，ec-3）。B站只在大会员 + 支持杜比的视频上返回。
    pub dolby: Option<RawDolby>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawDolby {
    #[serde(default)]
    pub audio: Option<RawDolbyAudio>,
}

/// 实测里 `dolby.audio` 是数组，但不同版本也可能给单个对象；两种都接受，
/// 否则一次未知形状就会让整条 playurl 反序列化失败、连带下载也挂掉。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawDolbyAudio {
    Many(Vec<RawAudioStream>),
    One(RawAudioStream),
}

impl RawDolbyAudio {
    pub(crate) fn streams(&self) -> &[RawAudioStream] {
        match self {
            Self::Many(streams) => streams,
            Self::One(stream) => std::slice::from_ref(stream),
        }
    }
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
    use super::{RawAudioStream, RawDash, RawVideoStream};

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

    #[test]
    fn accepts_a_dolby_audio_array_object_and_null() {
        let many: RawDash = serde_json::from_str(
            r#"{"video":[],"audio":[],
                "dolby":{"type":1,"audio":[
                  {"id":30250,"base_url":"https://snake/dolby","bandwidth":448000,
                   "mime_type":"audio/mp4","codecs":"ec-3"}]}}"#,
        )
        .unwrap();
        let dolby = many.dolby.expect("dolby block").audio.expect("dolby audio");
        assert_eq!(dolby.streams().len(), 1);
        assert_eq!(dolby.streams()[0].id, 30250);
        assert_eq!(dolby.streams()[0].resolved_base_url(), "https://snake/dolby");

        // 有些响应把 audio 直接写成对象
        let one: RawDash = serde_json::from_str(
            r#"{"video":[],"audio":[],
                "dolby":{"audio":{"id":30250,"baseUrl":"https://camel/dolby","bandwidth":448000}}}"#,
        )
        .unwrap();
        let dolby = one.dolby.expect("dolby block").audio.expect("dolby audio");
        assert_eq!(dolby.streams()[0].resolved_base_url(), "https://camel/dolby");

        // 没有杜比时必须是 null/缺省，而不是解析失败
        let none: RawDash =
            serde_json::from_str(r#"{"video":[],"audio":[],"dolby":null}"#).unwrap();
        assert!(none.dolby.is_none());
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
