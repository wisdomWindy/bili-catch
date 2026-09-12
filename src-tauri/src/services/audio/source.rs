use async_trait::async_trait;

use crate::infrastructure::bilibili::validate_media_url;
use crate::models::{AppError, AppErrorCode, AudioOutputProfile};
use crate::services::download::{validate_media_source_kind, MediaKind, MediaSourceCandidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceTier {
    Lossy,
    Lossless,
    HiResLossless,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioSourceCandidate {
    pub tier: AudioSourceTier,
    pub media: MediaSourceCandidate,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioMetadata {
    pub title: String,
    pub uploader: String,
    pub cover_url: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioSourceBundle {
    pub source: AudioSourceCandidate,
    pub metadata: AudioMetadata,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioSourceRequest {
    pub bvid: String,
    pub cid: u64,
    pub output_profile: AudioOutputProfile,
}

#[async_trait]
pub trait AudioSourcePort: Send + Sync {
    async fn resolve(&self, request: &AudioSourceRequest) -> Result<AudioSourceBundle, AppError>;
}

fn source_unavailable() -> AppError {
    AppError::new(
        AppErrorCode::E004,
        "The requested audio source is unavailable",
    )
}

fn validate_candidate(candidate: &AudioSourceCandidate) -> Result<(), AppError> {
    if candidate.media.kind != MediaKind::Audio {
        return Err(source_unavailable());
    }
    validate_media_source_kind(&candidate.media)?;
    validate_media_url(&candidate.media.primary_url)?;
    for backup_url in &candidate.media.backup_urls {
        validate_media_url(backup_url)?;
    }
    Ok(())
}

pub fn select_audio_source(
    candidates: &[AudioSourceCandidate],
    output_profile: AudioOutputProfile,
    authenticated: bool,
) -> Result<AudioSourceCandidate, AppError> {
    if output_profile == AudioOutputProfile::FlacLossless && !authenticated {
        return Err(AppError::new(
            AppErrorCode::E005,
            "Authentication is required for lossless audio",
        ));
    }

    let selected = match output_profile {
        AudioOutputProfile::FlacLossless => candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.tier,
                    AudioSourceTier::Lossless | AudioSourceTier::HiResLossless
                )
            })
            .max_by_key(|candidate| candidate.media.bandwidth)
            .ok_or_else(|| {
                AppError::new(
                    AppErrorCode::E006,
                    "Lossless audio is unavailable for the current account or video",
                )
            })?,
        AudioOutputProfile::Mp3 { .. } | AudioOutputProfile::M4aOriginal => {
            let maximum_kbps = if authenticated { 192 } else { 64 };
            candidates
                .iter()
                .filter(|candidate| candidate.tier == AudioSourceTier::Lossy)
                .filter(|candidate| candidate.media.bandwidth / 1_000 <= maximum_kbps)
                .max_by_key(|candidate| candidate.media.bandwidth)
                .ok_or_else(source_unavailable)?
        }
    };

    validate_candidate(selected)?;
    Ok(selected.clone())
}
