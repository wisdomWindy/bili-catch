use async_trait::async_trait;

use crate::infrastructure::bilibili::validate_media_url;
use crate::models::{AppError, AppErrorCode, AudioOutputProfile, VideoSource};
use crate::services::download::{validate_media_source_kind, MediaKind, MediaSourceCandidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceTier {
    /// 普通 AAC 音轨（30216/30232/30280），MP3 转码只能用它。
    Lossy,
    /// 杜比全景声（30250，ec-3）：码率高于普通 AAC，但 MP3 装不下多声道。
    DolbyAtmos,
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
    pub source: VideoSource,
    pub ep_id: Option<u64>,
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

fn validate_candidate(candidate: &AudioSourceCandidate) -> Result<AudioSourceCandidate, AppError> {
    if candidate.media.kind != MediaKind::Audio {
        return Err(source_unavailable());
    }
    validate_media_source_kind(&candidate.media)?;
    validate_media_url(&candidate.media.primary_url)?;
    let mut validated = candidate.clone();
    validated
        .media
        .backup_urls
        .retain(|backup_url| validate_media_url(backup_url).is_ok());
    Ok(validated)
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
        // 原样保存：AAC 和杜比都可以直接 copy 进 M4A，取码率最高的那一条，
        // 这样杜比视频不会再被降级成 192K AAC。
        AudioOutputProfile::M4aOriginal => candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.tier,
                    AudioSourceTier::Lossy | AudioSourceTier::DolbyAtmos
                )
            })
            .max_by_key(|candidate| candidate.media.bandwidth)
            .ok_or_else(source_unavailable)?,
        // WAV：能拿到无损就用无损（FLAC 解码后是逐样本一致的 PCM），
        // 否则退到码率最高的立体声 AAC——从 AAC 解出来的 WAV 不会再掉音质，
        // 但也不会比 AAC 本身更好，体积却大一个数量级。
        AudioOutputProfile::WavPcm => {
            let lossless = candidates
                .iter()
                .filter(|candidate| {
                    matches!(
                        candidate.tier,
                        AudioSourceTier::Lossless | AudioSourceTier::HiResLossless
                    )
                })
                .max_by_key(|candidate| candidate.media.bandwidth);
            match lossless {
                Some(candidate) => candidate,
                None => candidates
                    .iter()
                    .filter(|candidate| candidate.tier == AudioSourceTier::Lossy)
                    .max_by_key(|candidate| candidate.media.bandwidth)
                    .ok_or_else(source_unavailable)?,
            }
        }
        // MP3 必须重新编码：只能挑立体声 AAC，绝不能拿杜比 5.1 去压成 MP3，
        // 那是实打实的降级，不是“提升码率”。
        AudioOutputProfile::Mp3 { .. } => candidates
            .iter()
            .filter(|candidate| candidate.tier == AudioSourceTier::Lossy)
            .max_by_key(|candidate| candidate.media.bandwidth)
            .ok_or_else(|| {
                if candidates
                    .iter()
                    .any(|candidate| candidate.tier == AudioSourceTier::DolbyAtmos)
                {
                    AppError::new(
                        AppErrorCode::E006,
                        "This video only ships a Dolby track; choose the original M4A or FLAC output",
                    )
                } else {
                    source_unavailable()
                }
            })?,
    };

    validate_candidate(selected)
}

#[cfg(test)]
mod tests {
    use super::{select_audio_source, AudioSourceCandidate, AudioSourceTier};
    use crate::models::{AppErrorCode, AudioOutputProfile};
    use crate::services::download::{MediaKind, MediaSourceCandidate};

    fn candidate(tier: AudioSourceTier, id: u32, bandwidth: u64) -> AudioSourceCandidate {
        AudioSourceCandidate {
            tier,
            media: MediaSourceCandidate {
                id,
                kind: MediaKind::Audio,
                bandwidth,
                primary_url: format!("https://a.bilivideo.com/{id}.m4s"),
                backup_urls: Vec::new(),
                mime_type: "audio/mp4".into(),
                codecs: "mp4a.40.2".into(),
                content_length: None,
                etag: None,
            },
        }
    }

    fn candidates() -> Vec<AudioSourceCandidate> {
        vec![
            candidate(AudioSourceTier::Lossy, 30232, 132_000),
            candidate(AudioSourceTier::Lossy, 30280, 192_000),
            candidate(AudioSourceTier::DolbyAtmos, 30250, 448_000),
        ]
    }

    #[test]
    fn original_m4a_keeps_the_highest_real_stream_including_dolby() {
        let selected = select_audio_source(
            &candidates(),
            AudioOutputProfile::M4aOriginal,
            true,
        )
        .expect("original profile should resolve");

        assert_eq!(selected.media.id, 30250);
        assert_eq!(selected.tier, AudioSourceTier::DolbyAtmos);
    }

    #[test]
    fn mp3_never_downmixes_the_dolby_track() {
        let selected = select_audio_source(
            &candidates(),
            AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            true,
        )
        .expect("mp3 profile should fall back to stereo AAC");

        assert_eq!(selected.media.id, 30280);
        assert_eq!(selected.tier, AudioSourceTier::Lossy);
    }

    #[test]
    fn mp3_reports_a_dolby_only_video_instead_of_silently_downmixing() {
        let dolby_only = vec![candidate(AudioSourceTier::DolbyAtmos, 30250, 448_000)];

        let error = match select_audio_source(
            &dolby_only,
            AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            true,
        ) {
            Ok(_) => panic!("a Dolby-only video cannot be encoded as MP3"),
            Err(error) => error,
        };

        assert_eq!(error.code, AppErrorCode::E006);
    }

    #[test]
    fn wav_prefers_the_lossless_source_and_falls_back_to_stereo_aac() {
        let mut with_flac = candidates();
        with_flac.push(candidate(AudioSourceTier::HiResLossless, 30251, 1_411_000));
        assert_eq!(
            select_audio_source(&with_flac, AudioOutputProfile::WavPcm, false)
                .unwrap()
                .media
                .id,
            30251
        );

        // 没有无损源时退到最高的立体声 AAC，而不是杜比 5.1
        assert_eq!(
            select_audio_source(&candidates(), AudioOutputProfile::WavPcm, false)
                .unwrap()
                .media
                .id,
            30280
        );
    }

    #[test]
    fn lossless_output_still_requires_the_lossless_tier() {
        let error = match select_audio_source(
            &candidates(),
            AudioOutputProfile::FlacLossless,
            true,
        ) {
            Ok(_) => panic!("lossy sources cannot be selected for lossless output"),
            Err(error) => error,
        };
        assert_eq!(error.code, AppErrorCode::E006);

        let mut with_flac = candidates();
        with_flac.push(candidate(AudioSourceTier::HiResLossless, 30251, 1_411_000));
        let selected =
            select_audio_source(&with_flac, AudioOutputProfile::FlacLossless, true).unwrap();
        assert_eq!(selected.media.id, 30251);
    }
}
