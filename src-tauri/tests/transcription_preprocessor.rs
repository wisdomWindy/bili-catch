use std::{ffi::OsString, path::Path};

use bilicatch_lib::{
    infrastructure::transcription::ffmpeg_preprocessor::{
        build_preprocess_args, choose_chunk_ranges, supported_media_kind, ManagedAudioParts,
    },
    models::MediaKind,
};

#[test]
fn supported_media_extensions_are_case_insensitive_and_reject_unknown_files() {
    assert_eq!(
        supported_media_kind(Path::new("voice.MP3")),
        Some(MediaKind::Audio)
    );
    assert_eq!(
        supported_media_kind(Path::new("meeting.flac")),
        Some(MediaKind::Audio)
    );
    assert_eq!(
        supported_media_kind(Path::new("recording.MKV")),
        Some(MediaKind::Video)
    );
    assert_eq!(supported_media_kind(Path::new("notes.txt")), None);
}

#[test]
fn long_media_prefers_silence_within_five_seconds_of_each_target() {
    let ranges = choose_chunk_ranges(3_605.0, &[1_798.0, 3_599.0]);

    assert_eq!(ranges.len(), 3);
    assert_eq!((ranges[0].start_ms, ranges[0].duration_ms), (0, 1_798_000));
    assert_eq!(
        (ranges[1].start_ms, ranges[1].duration_ms),
        (1_798_000, 1_801_000)
    );
    assert_eq!(
        (ranges[2].start_ms, ranges[2].duration_ms),
        (3_599_000, 6_000)
    );
}

#[test]
fn preprocess_arguments_request_16khz_mono_signed_pcm_without_shell_joining() {
    let args = build_preprocess_args(
        Path::new("D:/Media/input video.mp4"),
        Path::new("D:/Temp/part-000.wav"),
        12_500,
        30_000,
    );

    let expected = [
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        "12.500",
        "-t",
        "30.000",
        "-i",
        "D:/Media/input video.mp4",
        "-map",
        "0:a:0",
        "-vn",
        "-ac",
        "1",
        "-ar",
        "16000",
        "-c:a",
        "pcm_s16le",
        "D:/Temp/part-000.wav",
    ]
    .map(OsString::from);
    assert_eq!(args, expected);
}

#[test]
fn managed_audio_parts_remove_the_entire_workspace_on_drop() {
    let root = tempfile::tempdir().expect("temporary root");
    let workspace = root.path().join("bilicatch-transcribe/task-1-2");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let part = workspace.join("part-000.wav");
    std::fs::write(&part, b"wav").expect("part");

    let managed = ManagedAudioParts::new(&workspace, vec![(part, 0)]);
    assert_eq!(managed.parts().len(), 1);
    drop(managed);

    assert!(!workspace.exists());
}
