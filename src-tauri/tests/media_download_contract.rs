use bilicatch_lib::{
    models::AppErrorCode,
    services::download::{
        media_source_identity, validate_media_source_kind, MediaKind, MediaSourceCandidate,
    },
};

fn candidate(kind: MediaKind, mime_type: &str) -> MediaSourceCandidate {
    MediaSourceCandidate {
        id: 80,
        kind,
        bandwidth: 1_000_000,
        primary_url: "https://upos-sz-mirrorcos.bilivideo.com/media.m4s?token=first".into(),
        backup_urls: vec![],
        mime_type: mime_type.into(),
        codecs: "avc1.640032".into(),
        content_length: Some(1024),
        etag: Some("fixture".into()),
    }
}

#[test]
fn media_kind_and_mime_must_match_without_container_fallback() {
    validate_media_source_kind(&candidate(MediaKind::Audio, "audio/mp4")).unwrap();
    validate_media_source_kind(&candidate(MediaKind::Audio, "audio/flac")).unwrap();
    validate_media_source_kind(&candidate(MediaKind::Video, "video/mp4")).unwrap();

    for source in [
        candidate(MediaKind::Audio, "video/mp4"),
        candidate(MediaKind::Video, "audio/mp4"),
        candidate(MediaKind::Video, "video/webm"),
    ] {
        let error =
            validate_media_source_kind(&source).expect_err("crossed or non-MP4 media must fail");
        assert_eq!(error.code, AppErrorCode::E004);
    }
}

#[test]
fn source_identity_includes_track_and_codec_but_excludes_ephemeral_urls() {
    let source = candidate(MediaKind::Video, "video/mp4");
    let identity = media_source_identity("BV1xx411c7BF", 1001, &source);

    let mut refreshed = source.clone();
    refreshed.primary_url =
        "https://upos-sz-mirrorcos.bilivideo.com/media.m4s?token=refreshed".into();
    refreshed.backup_urls = vec!["https://backup.bilivideo.com/media.m4s".into()];
    assert_eq!(
        identity,
        media_source_identity("BV1xx411c7BF", 1001, &refreshed)
    );

    let mut audio = source.clone();
    audio.kind = MediaKind::Audio;
    assert_ne!(
        identity,
        media_source_identity("BV1xx411c7BF", 1001, &audio)
    );

    let mut other_codec = source;
    other_codec.codecs = "hev1.1.6.L120.90".into();
    assert_ne!(
        identity,
        media_source_identity("BV1xx411c7BF", 1001, &other_codec)
    );
}
