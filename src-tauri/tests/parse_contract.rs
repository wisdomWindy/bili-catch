use bilicatch_lib::models::{
    AudioCapability, AudioFormat, Availability, ContainerKind, MediaOption, ParseContainer,
    ParseVideoResult, VideoCodec, VideoPart, VideoSource, VideoVariant,
};
use serde_json::json;

#[test]
fn parse_result_serializes_to_the_frontend_contract() {
    let result = ParseVideoResult {
        canonical_url: "https://www.bilibili.com/video/BV1xx411c7BF".into(),
        bvid: Some("BV1xx411c7BF".into()),
        aid: Some(170001),
        source: VideoSource::Ugc,
        container: Some(ParseContainer {
            kind: ContainerKind::Video,
            id: "BV1xx411c7BF".into(),
            title: "Example".into(),
            unit_count: 1,
        }),
        title: "Example".into(),
        owner_name: "Uploader".into(),
        cover_url: "https://i0.hdslb.com/example.jpg".into(),
        duration_seconds: 90,
        requested_page: None,
        requested_cid: None,
        parts: vec![VideoPart {
            cid: 1,
            bvid: "BV1xx411c7BF".into(),
            ep_id: None,
            page: 1,
            part_count: 1,
            group_index: 1,
            title: "P1".into(),
            duration_seconds: 90,
            availability: Availability::Playable,
            unavailable_reason: None,
        }],
        qualities: vec![MediaOption {
            id: "32".into(),
            label: "480P".into(),
            requires_login: false,
        }],
        codecs: vec![VideoCodec::Avc],
        video_variants: vec![VideoVariant {
            quality_id: "32".into(),
            codec: VideoCodec::Avc,
        }],
        audio_formats: vec![AudioFormat::Mp3, AudioFormat::M4a],
        audio_bitrates: vec![MediaOption {
            id: "128".into(),
            label: "128K".into(),
            requires_login: false,
        }],
        audio_capability: AudioCapability {
            max_lossy_kbps: Some(192),
            lossless_available: true,
            hi_res_available: true,
        },
    };

    assert_eq!(
        serde_json::to_value(result).expect("parse result should serialize"),
        json!({
            "canonicalUrl": "https://www.bilibili.com/video/BV1xx411c7BF",
            "bvid": "BV1xx411c7BF",
            "aid": 170001,
            "source": "ugc",
            "container": {
                "kind": "video",
                "id": "BV1xx411c7BF",
                "title": "Example",
                "unitCount": 1
            },
            "title": "Example",
            "ownerName": "Uploader",
            "coverUrl": "https://i0.hdslb.com/example.jpg",
            "durationSeconds": 90,
            "requestedPage": null,
            "requestedCid": null,
            "parts": [{
                "cid": 1,
                "bvid": "BV1xx411c7BF",
                "epId": null,
                "page": 1,
                "partCount": 1,
                "groupIndex": 1,
                "title": "P1",
                "durationSeconds": 90,
                "availability": "playable",
                "unavailableReason": null
            }],
            "qualities": [{ "id": "32", "label": "480P", "requiresLogin": false }],
            "codecs": ["avc"],
            "videoVariants": [{ "qualityId": "32", "codec": "avc" }],
            "audioFormats": ["mp3", "m4a"],
            "audioBitrates": [{ "id": "128", "label": "128K", "requiresLogin": false }],
            "audioCapability": {
                "maxLossyKbps": 192,
                "losslessAvailable": true,
                "hiResAvailable": true
            }
        })
    );
}
