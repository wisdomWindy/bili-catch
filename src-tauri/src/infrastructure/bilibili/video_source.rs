use crate::{
    models::VideoCodec,
    services::{
        download::{MediaKind, MediaSourceCandidate},
        video::VideoSourceCandidate,
    },
};

use super::raw::{PlayData, RawVideoStream};

fn codec_for(value: &str) -> Option<VideoCodec> {
    if value.starts_with("av01") {
        Some(VideoCodec::Av1)
    } else if value.starts_with("hev") || value.starts_with("hvc") {
        Some(VideoCodec::Hevc)
    } else if value.starts_with("avc") {
        Some(VideoCodec::Avc)
    } else {
        None
    }
}

fn adapt_stream(stream: &RawVideoStream, codec: VideoCodec) -> VideoSourceCandidate {
    VideoSourceCandidate {
        quality_id: stream.id.to_string(),
        codec: codec.clone(),
        source: MediaSourceCandidate {
            id: stream.id,
            kind: MediaKind::Video,
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

pub(crate) fn adapt_video_source_candidates(play: &PlayData) -> Vec<VideoSourceCandidate> {
    play.dash
        .as_ref()
        .map(|dash| {
            dash.video
                .iter()
                .filter_map(|stream| {
                    codec_for(&stream.codecs).map(|codec| adapt_stream(stream, codec))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::adapt_video_source_candidates;
    use crate::infrastructure::bilibili::raw::PlayData;
    use crate::models::VideoCodec;

    #[test]
    fn adapts_video_streams_with_stable_kind_and_codec_metadata() {
        let play: PlayData = serde_json::from_str(
            r#"{
              "accept_quality":[80,32],"accept_description":["1080P","480P"],
              "dash":{"video":[
                {"id":80,"baseUrl":"https://a.bilivideo.com/80.m4s","backupUrl":[],"bandwidth":1000000,"mimeType":"video/mp4","codecs":"avc1.640032"},
                {"id":80,"baseUrl":"https://a.bilivideo.com/80-hevc.m4s","backupUrl":[],"bandwidth":1000000,"mimeType":"video/mp4","codecs":"hev1.1.6.L120.90"},
                {"id":32,"baseUrl":"https://a.bilivideo.com/32.m4s","backupUrl":[],"bandwidth":500000,"mimeType":"video/webm","codecs":"av01.0.08M.08"}
              ],"audio":[]}
            }"#,
        )
        .unwrap();

        let candidates = adapt_video_source_candidates(&play);

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].quality_id, "80");
        assert_eq!(candidates[0].codec, VideoCodec::Avc);
        assert_eq!(candidates[1].codec, VideoCodec::Hevc);
        assert_eq!(candidates[2].source.mime_type, "video/webm");
    }
}
