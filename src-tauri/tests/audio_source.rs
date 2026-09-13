use bilicatch_lib::infrastructure::bilibili::validate_media_url;
use bilicatch_lib::models::{AppErrorCode, AudioOutputProfile};
use bilicatch_lib::services::audio::{select_audio_source, AudioSourceCandidate, AudioSourceTier};
use bilicatch_lib::services::download::{MediaKind, MediaSourceCandidate};

fn candidate(id: u32, kbps: u64, tier: AudioSourceTier, host: &str) -> AudioSourceCandidate {
    candidate_with_bandwidth(id, kbps * 1_000, tier, host)
}

fn candidate_with_bandwidth(
    id: u32,
    bandwidth: u64,
    tier: AudioSourceTier,
    host: &str,
) -> AudioSourceCandidate {
    AudioSourceCandidate {
        tier,
        media: MediaSourceCandidate {
            id,
            kind: MediaKind::Audio,
            bandwidth,
            primary_url: format!("https://{host}/audio/{id}.m4s?token=fixture"),
            backup_urls: vec![format!("https://backup.{host}/audio/{id}.m4s")],
            mime_type: "audio/mp4".into(),
            codecs: "mp4a.40.2".into(),
            content_length: None,
            etag: None,
        },
    }
}

#[test]
fn anonymous_selection_accepts_returned_standard_lossy_bandwidth() {
    let candidates = vec![
        candidate_with_bandwidth(
            30232,
            85_411,
            AudioSourceTier::Lossy,
            "upos-sz-mirrorcos.bilivideo.com",
        ),
        candidate_with_bandwidth(
            30216,
            65_971,
            AudioSourceTier::Lossy,
            "upos-sz-mirrorcos.bilivideo.com",
        ),
    ];

    let selected = select_audio_source(
        &candidates,
        AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
        false,
    )
    .expect("returned standard lossy sources should remain available anonymously");

    assert_eq!(selected.media.id, 30232);
}

#[test]
fn authenticated_selection_uses_the_best_standard_source() {
    let candidates = vec![
        candidate(
            30280,
            192,
            AudioSourceTier::Lossy,
            "upos-sz-mirrorcos.bilivideo.com",
        ),
        candidate(
            30216,
            64,
            AudioSourceTier::Lossy,
            "upos-sz-mirrorcos.bilivideo.com",
        ),
    ];

    let selected = select_audio_source(&candidates, AudioOutputProfile::M4aOriginal, true)
        .expect("authenticated users should receive the best standard source");

    assert_eq!(selected.media.id, 30280);
}

#[test]
fn lossless_selection_enforces_auth_and_availability() {
    let lossless = candidate(
        30251,
        1_411,
        AudioSourceTier::HiResLossless,
        "upos-sz-mirrorcos.bilivideo.com",
    );

    let auth_error = select_audio_source(
        std::slice::from_ref(&lossless),
        AudioOutputProfile::FlacLossless,
        false,
    )
    .err()
    .expect("anonymous lossless selection must fail");
    assert_eq!(auth_error.code, AppErrorCode::E005);

    let unavailable_error = select_audio_source(&[], AudioOutputProfile::FlacLossless, true)
        .err()
        .expect("missing lossless source must fail");
    assert_eq!(unavailable_error.code, AppErrorCode::E006);

    let selected = select_audio_source(&[lossless], AudioOutputProfile::FlacLossless, true)
        .expect("authenticated users may use an available lossless source");
    assert_eq!(selected.tier, AudioSourceTier::HiResLossless);
}

#[test]
fn media_urls_require_https_and_an_explicit_cdn_allowlist() {
    for allowed in [
        "https://upos-sz-mirrorcos.bilivideo.com/audio.m4s",
        "https://upos-sz-mirrorcos.bilivideo.cn/audio.m4s",
        "https://i0.hdslb.com/cover.jpg",
    ] {
        validate_media_url(allowed).expect("known Bilibili media host should be allowed");
    }

    for rejected in [
        "http://upos-sz-mirrorcos.bilivideo.com/audio.m4s",
        "https://bilivideo.com.evil.example/audio.m4s",
        "https://user:secret@i0.hdslb.com/cover.jpg",
        "https://127.0.0.1/audio.m4s",
    ] {
        assert!(validate_media_url(rejected).is_err(), "accepted {rejected}");
    }
}

#[test]
fn audio_selection_rejects_a_video_kind_even_with_an_audio_tier() {
    let mut crossed = candidate(
        30216,
        64,
        AudioSourceTier::Lossy,
        "upos-sz-mirrorcos.bilivideo.com",
    );
    crossed.media.kind = MediaKind::Video;
    crossed.media.mime_type = "video/mp4".into();

    let error = match select_audio_source(&[crossed], AudioOutputProfile::M4aOriginal, false) {
        Ok(_) => panic!("audio selection must retain its media-kind guard"),
        Err(error) => error,
    };

    assert_eq!(error.code, AppErrorCode::E004);
}
