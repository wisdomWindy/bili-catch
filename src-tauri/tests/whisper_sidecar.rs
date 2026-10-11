use std::{ffi::OsString, path::Path};

use bilicatch_lib::{
    infrastructure::transcription::whisper_sidecar::{
        build_whisper_args, parse_progress, parse_whisper_json, resolve_whisper_sidecar,
        WhisperRequest,
    },
    models::{GpuMode, TranscriptionLanguage},
};

#[test]
fn sidecar_locator_accepts_only_the_fixed_platform_names() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "whisper-cli.exe",
        "whisper-cli",
        "whisper-cli-x86_64-pc-windows-msvc.exe",
        "whisper-cli-x86_64-apple-darwin",
        "whisper-cli-aarch64-apple-darwin",
        "whisper-cli-x86_64-unknown-linux-gnu",
        "whisper-cli-aarch64-unknown-linux-gnu",
    ] {
        let sidecar = root.path().join(name);
        std::fs::write(&sidecar, b"binary").unwrap();
        assert_eq!(resolve_whisper_sidecar(&sidecar).unwrap(), sidecar);
    }

    let wrong = root.path().join("whisper.exe");
    std::fs::write(&wrong, b"binary").unwrap();
    assert_eq!(
        resolve_whisper_sidecar(&wrong)
            .unwrap_err()
            .details
            .as_deref(),
        Some("SIDECAR_UNAVAILABLE")
    );
}

#[test]
fn typed_builder_emits_v1_8_6_full_json_args_without_shell_joining() {
    let args = build_whisper_args(&WhisperRequest {
        model_path: Path::new("D:/Models/ggml-base.bin").into(),
        audio_path: Path::new("D:/Audio/part 01.wav").into(),
        output_base: Path::new("D:/Temp/result-01").into(),
        language: TranscriptionLanguage::ZhTw,
        gpu_mode: GpuMode::Disabled,
        offset_ms: 0,
    });

    let expected = [
        "-m",
        "D:/Models/ggml-base.bin",
        "-f",
        "D:/Audio/part 01.wav",
        "-l",
        "zh",
        "-ojf",
        "-of",
        "D:/Temp/result-01",
        "-pp",
        "-np",
        "-ng",
    ]
    .map(OsString::from);
    assert_eq!(args, expected);
}

#[test]
fn full_json_parser_normalizes_offsets_and_detected_language() {
    let payload = r#"{
      "result": { "language": "zh" },
      "transcription": [
        { "offsets": { "from": 0, "to": 1250 }, "text": " 你好。 " },
        { "offsets": { "from": 1250, "to": 2410 }, "text": "世界" }
      ]
    }"#;

    let result = parse_whisper_json(payload.as_bytes(), 5_000).unwrap();

    assert_eq!(result.detected_language, "zh");
    assert_eq!(result.segments.len(), 2);
    assert_eq!(result.segments[0].index, 0);
    assert_eq!(result.segments[0].start_ms, 5_000);
    assert_eq!(result.segments[1].end_ms, 7_410);
    assert_eq!(result.segments[0].text, "你好。");
}

#[test]
fn parser_rejects_empty_speech_and_progress_is_best_effort() {
    let error = parse_whisper_json(
        br#"{"result":{"language":"en"},"transcription":[{"offsets":{"from":0,"to":5},"text":"  "}]}"#,
        0,
    )
    .unwrap_err();
    assert_eq!(error.code, bilicatch_lib::models::AppErrorCode::E015);
    assert_eq!(
        parse_progress("whisper_print_progress_callback: progress = 37%"),
        Some(0.37)
    );
    assert_eq!(parse_progress("unrelated diagnostic"), None);
}
