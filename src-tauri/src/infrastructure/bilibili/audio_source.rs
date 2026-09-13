use url::Url;

use crate::models::{AppError, AppErrorCode};
use crate::services::audio::{AudioMetadata, AudioSourceCandidate, AudioSourceTier};
use crate::services::download::{MediaKind, MediaSourceCandidate};

use super::{
    raw::{PlayData, RawAudioStream, ViewData},
    validate_media_url,
};

fn adapt_stream(stream: &RawAudioStream, tier: AudioSourceTier) -> AudioSourceCandidate {
    AudioSourceCandidate {
        tier,
        media: MediaSourceCandidate {
            id: stream.id,
            kind: MediaKind::Audio,
            bandwidth: stream.bandwidth,
            primary_url: stream.resolved_base_url().to_owned(),
            backup_urls: stream.resolved_backup_urls().to_owned(),
            mime_type: stream.resolved_mime_type().to_owned(),
            codecs: stream.codecs.clone(),
            content_length: None,
            etag: None,
        },
    }
}

pub(crate) fn adapt_audio_source_candidates(play: &PlayData) -> Vec<AudioSourceCandidate> {
    let Some(dash) = &play.dash else {
        return Vec::new();
    };
    let mut candidates = dash
        .audio
        .iter()
        .map(|stream| adapt_stream(stream, AudioSourceTier::Lossy))
        .collect::<Vec<_>>();
    if let Some(stream) = dash
        .flac
        .as_ref()
        .filter(|flac| flac.display)
        .and_then(|flac| flac.audio.as_ref())
    {
        candidates.push(adapt_stream(stream, AudioSourceTier::HiResLossless));
    }
    candidates
}

fn invalid_cover_url() -> AppError {
    AppError::new(AppErrorCode::E004, "The audio cover is unavailable")
}

fn normalize_cover_url(value: &str) -> Result<String, AppError> {
    let mut url = Url::parse(value).map_err(|_| invalid_cover_url())?;
    if url.scheme() == "http" {
        url.set_scheme("https").map_err(|_| invalid_cover_url())?;
    }
    validate_media_url(url.as_str()).map(|url| url.to_string())
}

pub(crate) fn adapt_audio_metadata(view: &ViewData) -> Result<AudioMetadata, AppError> {
    Ok(AudioMetadata {
        title: if view.title.trim().is_empty() {
            view.bvid.clone()
        } else {
            view.title.trim().to_owned()
        },
        uploader: if view.owner.name.trim().is_empty() {
            "Bilibili".into()
        } else {
            view.owner.name.trim().to_owned()
        },
        cover_url: normalize_cover_url(&view.pic)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{adapt_audio_metadata, adapt_audio_source_candidates};
    use crate::infrastructure::bilibili::raw::{PlayData, ViewData};
    use crate::models::AppErrorCode;
    use crate::services::audio::AudioSourceTier;
    use crate::services::download::MediaKind;

    #[test]
    fn extracts_standard_and_lossless_dash_audio_without_exposing_raw_types() {
        let play: PlayData = serde_json::from_str(
            r#"{
              "accept_quality":[],"accept_description":[],
              "dash":{
                "video":[],
                "audio":[{
                  "id":30280,"baseUrl":"https://a.bilivideo.com/standard.m4s",
                  "backupUrl":["https://b.bilivideo.cn/standard.m4s"],
                  "bandwidth":192000,"mimeType":"audio/mp4","codecs":"mp4a.40.2"
                }],
                "flac":{"display":true,"audio":{
                  "id":30251,"base_url":"https://a.bilivideo.com/lossless.m4s",
                  "backup_url":[],"bandwidth":1411000,"mime_type":"audio/flac","codecs":"fLaC"
                }}
              }
            }"#,
        )
        .expect("fixture should deserialize");

        let candidates = adapt_audio_source_candidates(&play);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].media.id, 30280);
        assert_eq!(candidates[0].tier, AudioSourceTier::Lossy);
        assert_eq!(candidates[0].media.kind, MediaKind::Audio);
        assert_eq!(candidates[0].media.backup_urls.len(), 1);
        assert_eq!(candidates[1].media.id, 30251);
        assert_eq!(candidates[1].tier, AudioSourceTier::HiResLossless);
    }

    #[test]
    fn adapts_required_metadata_with_safe_text_fallbacks() {
        let view: ViewData = serde_json::from_str(
            r#"{
              "bvid":"BV1xx411c7BF","aid":170001,"title":"  ",
              "pic":"http://i0.hdslb.com/cover.jpg?token=fixture","duration":90,
              "owner":{"name":""},"pages":[]
            }"#,
        )
        .expect("fixture should deserialize");

        let metadata = adapt_audio_metadata(&view).expect("trusted cover should adapt");

        assert_eq!(metadata.title, "BV1xx411c7BF");
        assert_eq!(metadata.uploader, "Bilibili");
        assert_eq!(
            metadata.cover_url,
            "https://i0.hdslb.com/cover.jpg?token=fixture"
        );
    }

    #[test]
    fn preserves_valid_https_audio_cover_url() {
        let view: ViewData = serde_json::from_str(
            r#"{
              "bvid":"BV1xx411c7BF","aid":170001,"title":"Fixture",
              "pic":"https://i0.hdslb.com/cover.jpg?token=fixture","duration":90,
              "owner":{"name":"Uploader"},"pages":[]
            }"#,
        )
        .expect("fixture should deserialize");

        let metadata = adapt_audio_metadata(&view).expect("trusted cover should adapt");

        assert_eq!(
            metadata.cover_url,
            "https://i0.hdslb.com/cover.jpg?token=fixture"
        );
    }

    #[test]
    fn rejects_unsafe_audio_cover_urls() {
        for cover_url in [
            "",
            "/cover.jpg",
            "ftp://i0.hdslb.com/cover.jpg",
            "http://example.com/cover.jpg",
            "http://user:password@i0.hdslb.com/cover.jpg",
            "http://i0.hdslb.com/cover.jpg#fragment",
        ] {
            let mut view: ViewData = serde_json::from_str(
                r#"{
                  "bvid":"BV1xx411c7BF","aid":170001,"title":"Fixture",
                  "pic":"https://i0.hdslb.com/cover.jpg","duration":90,
                  "owner":{"name":"Uploader"},"pages":[]
                }"#,
            )
            .expect("fixture should deserialize");
            view.pic = cover_url.into();

            let error = match adapt_audio_metadata(&view) {
                Ok(_) => panic!("unsafe cover URL should be rejected"),
                Err(error) => error,
            };

            assert_eq!(error.code, AppErrorCode::E004);
        }
    }
}
