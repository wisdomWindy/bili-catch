use async_trait::async_trait;

use crate::{
    models::{AppError, AppErrorCode, DownloadMode, VideoCodec},
    services::download::DownloadProgress,
    services::download::{validate_media_source_kind, MediaSourceCandidate},
};

pub fn aggregate_video_progress(
    video: &DownloadProgress,
    audio: Option<&DownloadProgress>,
) -> DownloadProgress {
    let Some(audio) = audio else {
        return video.clone();
    };
    let total_bytes = video
        .total_bytes
        .zip(audio.total_bytes)
        .map(|(video, audio)| video + audio);
    let eta_seconds = total_bytes.and_then(|total| {
        let remaining = total.saturating_sub(video.downloaded_bytes + audio.downloaded_bytes);
        let speed = video.speed_bytes_per_second + audio.speed_bytes_per_second;
        (speed > 0).then_some(remaining / speed)
    });
    DownloadProgress {
        downloaded_bytes: video.downloaded_bytes + audio.downloaded_bytes,
        total_bytes,
        speed_bytes_per_second: video.speed_bytes_per_second + audio.speed_bytes_per_second,
        eta_seconds,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSourceCandidate {
    pub quality_id: String,
    pub codec: VideoCodec,
    pub source: MediaSourceCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSourceRequest {
    pub bvid: String,
    pub cid: u64,
    pub mode: DownloadMode,
    pub quality_id: String,
    pub codec: VideoCodec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSourceBundle {
    pub video: MediaSourceCandidate,
    pub audio: Option<MediaSourceCandidate>,
}

#[async_trait]
pub trait VideoSourcePort: Send + Sync {
    async fn resolve(&self, request: &VideoSourceRequest) -> Result<VideoSourceBundle, AppError>;
}

pub fn select_video_source(
    candidates: &[VideoSourceCandidate],
    quality_id: &str,
    codec: VideoCodec,
    authenticated: bool,
) -> Result<MediaSourceCandidate, AppError> {
    if !authenticated
        && quality_id
            .parse::<u32>()
            .ok()
            .is_some_and(|quality| quality > 32)
    {
        return Err(AppError::new(
            AppErrorCode::E003,
            "Authentication is required for the selected video quality",
        ));
    }
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.quality_id == quality_id && candidate.codec == codec)
        .ok_or_else(|| {
            AppError::new(
                AppErrorCode::E004,
                "The requested video source is unavailable",
            )
        })?;
    validate_media_source_kind(&candidate.source)?;
    Ok(candidate.source.clone())
}
