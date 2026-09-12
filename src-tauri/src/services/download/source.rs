use serde::{Deserialize, Serialize};

use crate::models::{AppError, AppErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSourceCandidate {
    pub id: u32,
    pub kind: MediaKind,
    pub bandwidth: u64,
    pub primary_url: String,
    pub backup_urls: Vec<String>,
    pub mime_type: String,
    pub codecs: String,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
}

pub fn validate_media_source_kind(source: &MediaSourceCandidate) -> Result<(), AppError> {
    let valid = match source.kind {
        MediaKind::Audio => source.mime_type.starts_with("audio/"),
        MediaKind::Video => source.mime_type == "video/mp4",
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::new(
            AppErrorCode::E004,
            "The selected media stream has an unsupported type",
        ))
    }
}

pub fn media_source_identity(bvid: &str, cid: u64, source: &MediaSourceCandidate) -> String {
    let kind = match source.kind {
        MediaKind::Audio => "audio",
        MediaKind::Video => "video",
    };
    let value = format!(
        "{bvid}:{cid}:{kind}:{}:{}:{}:{}",
        source.id,
        source.mime_type,
        source.codecs,
        source.content_length.unwrap_or(0),
    );
    format!("{:x}", md5::compute(value.as_bytes()))
}
