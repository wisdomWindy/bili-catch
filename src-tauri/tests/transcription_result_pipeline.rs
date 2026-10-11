use std::path::Path;

use bilicatch_lib::{
    models::{AsrModelId, TranscriptFormat, TranscriptSegment, TranscriptionLanguage},
    services::transcription::{
        cache::source_fingerprint,
        export::{choose_export_path, render_transcript},
        processing::{apply_punctuation_preference, merge_segments},
        scheduler::effective_capacity,
    },
};

fn segments() -> Vec<TranscriptSegment> {
    vec![
        TranscriptSegment {
            index: 0,
            start_ms: 1_250,
            end_ms: 2_500,
            text: "Hello, world!".into(),
        },
        TranscriptSegment {
            index: 1,
            start_ms: 3_661_005,
            end_ms: 3_662_010,
            text: "第二段。".into(),
        },
    ]
}

#[test]
fn renderers_produce_stable_text_subtitle_json_and_markdown() {
    let value = segments();
    assert_eq!(
        render_transcript(&value, "zh", TranscriptFormat::Txt, false).unwrap(),
        "Hello, world!\n第二段。\n"
    );
    assert_eq!(
        render_transcript(&value, "zh", TranscriptFormat::Srt, false).unwrap(),
        "1\n00:00:01,250 --> 00:00:02,500\nHello, world!\n\n2\n01:01:01,005 --> 01:01:02,010\n第二段。\n"
    );
    assert_eq!(
        render_transcript(&value, "zh", TranscriptFormat::Vtt, false).unwrap(),
        "WEBVTT\n\n00:00:01.250 --> 00:00:02.500\nHello, world!\n\n01:01:01.005 --> 01:01:02.010\n第二段。\n"
    );
    let json = render_transcript(&value, "zh", TranscriptFormat::Json, false).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["language"],
        "zh"
    );
    assert!(json.ends_with('\n'));
    assert_eq!(
        render_transcript(&value, "zh", TranscriptFormat::Markdown, true).unwrap(),
        "- [00:00:01.250] Hello, world!\n- [01:01:01.005] 第二段。\n"
    );
}

#[test]
fn punctuation_rule_preserves_apostrophes_decimals_and_path_content() {
    let input = "It's 3.14, 你好！ C:/media/file.wav。";
    assert_eq!(
        apply_punctuation_preference(input, false),
        "It's 3.14 你好 C:/media/file.wav"
    );
    assert_eq!(apply_punctuation_preference(input, true), input);
}

#[test]
fn chunk_merge_reindexes_and_clamps_non_monotonic_boundaries() {
    let merged = merge_segments(vec![
        vec![TranscriptSegment {
            index: 7,
            start_ms: 100,
            end_ms: 900,
            text: "one".into(),
        }],
        vec![TranscriptSegment {
            index: 4,
            start_ms: 850,
            end_ms: 800,
            text: "two".into(),
        }],
    ]);
    assert_eq!(merged[0].index, 0);
    assert_eq!(merged[1].index, 1);
    assert_eq!(merged[1].start_ms, 900);
    assert_eq!(merged[1].end_ms, 900);
}

#[test]
fn fingerprint_and_scheduler_capacity_include_all_stable_inputs() {
    let first = source_fingerprint(
        Path::new("D:/media/a.mp3"),
        10,
        20,
        AsrModelId::Base,
        TranscriptionLanguage::Auto,
        true,
    );
    let second = source_fingerprint(
        Path::new("D:/media/a.mp3"),
        10,
        21,
        AsrModelId::Base,
        TranscriptionLanguage::Auto,
        true,
    );
    assert_ne!(first, second);
    assert_eq!(effective_capacity(3, 0), 3);
    assert_eq!(effective_capacity(3, 1), 1);
    assert_eq!(effective_capacity(0, 0), 1);
}

#[test]
fn export_name_is_sanitized_bounded_and_never_overwrites() {
    let root = tempfile::tempdir().unwrap();
    let first =
        choose_export_path(root.path(), "bad:name?.mp4", "zh", TranscriptFormat::Srt).unwrap();
    std::fs::write(&first, b"existing").unwrap();
    let second =
        choose_export_path(root.path(), "bad:name?.mp4", "zh", TranscriptFormat::Srt).unwrap();
    assert_eq!(first.file_name().unwrap(), "bad_name_.zh.srt");
    assert_eq!(second.file_name().unwrap(), "bad_name_.zh (1).srt");
    assert!(
        second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .chars()
            .count()
            <= 200
    );
}
