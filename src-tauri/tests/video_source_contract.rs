use bilicatch_lib::{
    models::{AppErrorCode, VideoCodec},
    services::{
        download::{MediaKind, MediaSourceCandidate},
        video::{select_video_source, VideoSourceCandidate},
    },
};

fn candidate(quality_id: &str, codec: VideoCodec, mime_type: &str) -> VideoSourceCandidate {
    VideoSourceCandidate {
        quality_id: quality_id.into(),
        source: MediaSourceCandidate {
            id: quality_id.parse().unwrap(),
            kind: MediaKind::Video,
            bandwidth: 1_000_000,
            primary_url: "https://upos-sz-mirrorcos.bilivideo.com/video.m4s?token=fixture".into(),
            backup_urls: vec![],
            mime_type: mime_type.into(),
            codecs: match codec {
                VideoCodec::Avc => "avc1.640032",
                VideoCodec::Hevc => "hev1.1.6.L120.90",
                VideoCodec::Av1 => "av01.0.08M.08",
            }
            .into(),
            content_length: Some(1024),
            etag: Some("fixture".into()),
        },
        codec,
    }
}

#[test]
fn selector_requires_exact_quality_codec_and_mp4_source() {
    let candidates = vec![candidate("80", VideoCodec::Avc, "video/mp4")];

    let selected = select_video_source(&candidates, "80", VideoCodec::Avc, true)
        .expect("authenticated exact pair should resolve");
    assert_eq!(selected.id, 80);

    let wrong_codec = select_video_source(&candidates, "80", VideoCodec::Hevc, true)
        .expect_err("the selector must not invent a codec cross product");
    assert_eq!(wrong_codec.code, AppErrorCode::E004);

    let locked = select_video_source(&candidates, "80", VideoCodec::Avc, false)
        .expect_err("anonymous high quality must require login");
    assert_eq!(locked.code, AppErrorCode::E003);
}

#[test]
fn selector_rejects_non_mp4_video_source_before_download() {
    let error = select_video_source(
        &[candidate("32", VideoCodec::Avc, "video/webm")],
        "32",
        VideoCodec::Avc,
        false,
    )
    .expect_err("video-only must not silently transcode a non-MP4 source");

    assert_eq!(error.code, AppErrorCode::E004);
}
