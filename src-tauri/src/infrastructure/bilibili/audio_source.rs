use crate::services::audio::{AudioMetadata, AudioSourceCandidate, AudioSourceTier};
use crate::services::download::{MediaKind, MediaSourceCandidate};

use super::raw::{PlayData, RawAudioStream, ViewData};

fn adapt_stream(stream: &RawAudioStream, tier: AudioSourceTier) -> AudioSourceCandidate {
    AudioSourceCandidate {
        tier,
        media: MediaSourceCandidate {
            id: stream.id,
            kind: MediaKind::Audio,
            bandwidth: stream.bandwidth,
            primary_url: stream.base_url.clone(),
            backup_urls: stream.backup_url.clone(),
            mime_type: stream.mime_type.clone(),
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

pub(crate) fn adapt_audio_metadata(view: &ViewData) -> AudioMetadata {
    AudioMetadata {
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
        cover_url: view.pic.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{adapt_audio_metadata, adapt_audio_source_candidates};
    use crate::infrastructure::bilibili::raw::{PlayData, ViewData};
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
              "pic":"https://i0.hdslb.com/cover.jpg","duration":90,
              "owner":{"name":""},"pages":[]
            }"#,
        )
        .expect("fixture should deserialize");

        let metadata = adapt_audio_metadata(&view);

        assert_eq!(metadata.title, "BV1xx411c7BF");
        assert_eq!(metadata.uploader, "Bilibili");
        assert_eq!(metadata.cover_url, "https://i0.hdslb.com/cover.jpg");
    }
}
