use bilicatch_lib::{
    models::{
        AudioFormat, AudioOutputProfile, DownloadMode, DownloadTaskDraft, TaskAction, TaskStatus,
        VideoCodec,
    },
    services::tasks::{
        actions_for_status, sanitize_audio_filename, sanitize_filename, validate_create_request,
    },
};

fn draft(mode: DownloadMode) -> DownloadTaskDraft {
    DownloadTaskDraft {
        canonical_url: "https://www.bilibili.com/video/BV1xx411c7BF".into(),
        bvid: "BV1xx411c7BF".into(),
        cid: 1,
        page: 1,
        part_title: "P1".into(),
        video_title: "Example video".into(),
        part_count: 1,
        mode,
        output_dir: " Downloads ".into(),
        quality_id: Some("80".into()),
        codec: Some(VideoCodec::Avc),
        audio_format: None,
        audio_bitrate_id: None,
    }
}

#[test]
fn action_matrix_exposes_only_valid_user_intents() {
    assert_eq!(
        actions_for_status(TaskStatus::Queued, false),
        vec![TaskAction::Cancel]
    );
    assert_eq!(
        actions_for_status(TaskStatus::Downloading, false),
        vec![TaskAction::Pause, TaskAction::Cancel]
    );
    assert_eq!(
        actions_for_status(TaskStatus::Failed, false),
        vec![TaskAction::Retry, TaskAction::Delete]
    );
    assert!(actions_for_status(TaskStatus::Processing, false).is_empty());
    assert_eq!(
        actions_for_status(TaskStatus::Processing, true),
        vec![TaskAction::Cancel]
    );
}

#[test]
fn batch_validation_is_atomic_and_normalizes_output_directory() {
    let normalized =
        validate_create_request(" request_1 ", vec![draft(DownloadMode::VideoOnly)], 99)
            .expect("one task should fit");
    assert_eq!(normalized.request_id, "request_1");
    assert_eq!(normalized.drafts[0].output_dir, "Downloads");

    let error = validate_create_request(
        "request_2",
        vec![
            draft(DownloadMode::VideoOnly),
            draft(DownloadMode::VideoOnly),
        ],
        99,
    )
    .expect_err("the entire batch must be rejected");
    assert_eq!(error.details.as_deref(), Some("TASK_QUEUE_FULL"));
}

#[test]
fn validation_rejects_invalid_ids_and_mode_combinations() {
    assert!(validate_create_request("bad id", vec![draft(DownloadMode::VideoOnly)], 0).is_err());

    let mut audio = draft(DownloadMode::AudioOnly);
    audio.audio_format = None;
    assert!(validate_create_request("audio_1", vec![audio], 0).is_err());

    let mut empty_quality = draft(DownloadMode::VideoOnly);
    empty_quality.quality_id = Some("   ".into());
    assert!(validate_create_request("video_1", vec![empty_quality], 0).is_err());
}

#[test]
fn audio_profiles_reject_cross_format_values() {
    assert_eq!(
        AudioOutputProfile::try_from_selection(AudioFormat::Mp3, "320").unwrap(),
        AudioOutputProfile::Mp3 { bitrate_kbps: 320 }
    );
    assert_eq!(
        AudioOutputProfile::try_from_selection(AudioFormat::M4a, "source").unwrap(),
        AudioOutputProfile::M4aOriginal
    );
    assert_eq!(
        AudioOutputProfile::try_from_selection(AudioFormat::Flac, "lossless").unwrap(),
        AudioOutputProfile::FlacLossless
    );
    assert!(AudioOutputProfile::try_from_selection(AudioFormat::Mp3, "source").is_err());
    assert!(AudioOutputProfile::try_from_selection(AudioFormat::M4a, "192").is_err());

    let mut audio = draft(DownloadMode::AudioOnly);
    audio.quality_id = None;
    audio.codec = None;
    audio.audio_format = Some(AudioFormat::Mp3);
    audio.audio_bitrate_id = Some("source".into());
    assert!(validate_create_request("audio_profile", vec![audio.clone()], 0).is_err());
    audio.audio_bitrate_id = Some("320".into());
    assert!(validate_create_request("audio_profile", vec![audio], 0).is_ok());
}

#[test]
fn validation_rejects_invalid_part_metadata() {
    let mut item = draft(DownloadMode::VideoOnly);
    item.video_title = "   ".into();
    assert!(validate_create_request("title", vec![item], 0).is_err());

    let mut item = draft(DownloadMode::VideoOnly);
    item.page = 2;
    item.part_count = 1;
    assert!(validate_create_request("page", vec![item], 0).is_err());
}

#[test]
fn filenames_are_cross_platform_safe_and_limited() {
    assert_eq!(
        sanitize_filename("  bad:/name.  ", "BV1", 2, "mp4"),
        "badname.mp4"
    );
    assert_eq!(sanitize_filename("...", "BV1", 2, "m4a"), "BV1-P2.m4a");
    assert_eq!(sanitize_filename("CON", "BV1", 1, "mp4"), "CON_.mp4");
    assert_eq!(
        sanitize_filename(&"a".repeat(250), "BV1", 1, "mp4")
            .chars()
            .count(),
        200
    );
}

#[test]
fn audio_filenames_use_video_title_for_single_part_and_page_title_for_collections() {
    assert_eq!(
        sanitize_audio_filename("  Main\n  title  ", "Part", "BV1", 1, 1, "mp3"),
        "Main title.mp3"
    );
    assert_eq!(
        sanitize_audio_filename("Main", "  bad:/  name  ", "BV1", 2, 3, "m4a"),
        "2_bad name.m4a"
    );
    assert_eq!(
        sanitize_audio_filename("???", "...", "BV1", 2, 3, "flac"),
        "BV1-P2.flac"
    );
}
