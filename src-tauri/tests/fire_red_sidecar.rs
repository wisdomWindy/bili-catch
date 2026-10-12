use std::{ffi::OsString, path::Path, sync::Arc};

use bilicatch_lib::{
    infrastructure::transcription::fire_red_sidecar::{
        build_fire_red_args, parse_fire_red_json, resolve_fire_red_sidecar, FireRedRequest,
        FireRedSidecarTranscriber,
    },
    models::{GpuMode, TranscriptionLanguage},
    services::{
        audio::ProcessControl,
        transcription::ports::{Transcriber, TranscriberRequest},
    },
};

#[test]
fn locator_accepts_only_the_fixed_sherpa_onnx_sidecar_names() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "sherpa-onnx-offline.exe",
        "sherpa-onnx-offline",
        "sherpa-onnx-offline-x86_64-pc-windows-msvc.exe",
        "sherpa-onnx-offline-x86_64-apple-darwin",
        "sherpa-onnx-offline-aarch64-apple-darwin",
    ] {
        let sidecar = root.path().join(name);
        std::fs::write(&sidecar, b"binary").unwrap();
        assert_eq!(resolve_fire_red_sidecar(&sidecar).unwrap(), sidecar);
    }

    let wrong = root.path().join("sherpa-onnx.exe");
    std::fs::write(&wrong, b"binary").unwrap();
    assert_eq!(
        resolve_fire_red_sidecar(&wrong)
            .unwrap_err()
            .details
            .as_deref(),
        Some("SIDECAR_UNAVAILABLE")
    );
}

#[test]
fn typed_builder_emits_fire_red_ctc_cpu_args_without_shell_joining() {
    let args = build_fire_red_args(
        &FireRedRequest {
            model_path: Path::new("D:/Models/fire red/model.int8.onnx").into(),
            audio_path: Path::new("D:/Audio/part 01.wav").into(),
            output_base: Path::new("D:/Temp/result-01").into(),
            language: TranscriptionLanguage::Zh,
            gpu_mode: GpuMode::Auto,
            offset_ms: 0,
        },
        Path::new("D:/App/tokens.txt"),
    );

    assert_eq!(
        args,
        [
            "--tokens=D:/App/tokens.txt",
            "--fire-red-asr-ctc=D:/Models/fire red/model.int8.onnx",
            "--model-type=fire_red_asr",
            "--num-threads=4",
            "--debug=false",
            "D:/Audio/part 01.wav",
        ]
        .map(OsString::from)
    );
}

#[test]
fn json_parser_builds_timestamped_segments_and_preserves_mixed_language_text() {
    let payload = br#"{
      "lang":"",
      "text":"\u4f60\u597d world\u3002\u4e0b\u4e00\u53e5",
      "timestamps":[0.20,0.45,0.80,1.10,1.45,1.80,2.10],
      "tokens":["\u4f60","\u597d"," world","\u3002","\u4e0b","\u4e00","\u53e5"]
    }"#;

    let output = parse_fire_red_json(payload, 5_000, TranscriptionLanguage::Zh).unwrap();

    assert_eq!(output.detected_language, "zh");
    assert_eq!(output.segments.len(), 2);
    assert_eq!(output.segments[0].start_ms, 5_200);
    assert_eq!(output.segments[0].end_ms, 6_600);
    assert_eq!(output.segments[0].text, "\u{4f60}\u{597d} world\u{3002}");
    assert_eq!(output.segments[1].text, "\u{4e0b}\u{4e00}\u{53e5}");
    assert_eq!(output.segments[1].end_ms, 7_600);
}

#[test]
fn parser_rejects_empty_speech() {
    let error = parse_fire_red_json(
        br#"{"lang":"","text":" ","timestamps":[],"tokens":[]}"#,
        0,
        TranscriptionLanguage::Auto,
    )
    .unwrap_err();

    assert_eq!(error.code, bilicatch_lib::models::AppErrorCode::E015);
}

#[tokio::test]
#[ignore = "requires the official FireRedASR2 model fixture"]
async fn official_model_smoke_transcribes_with_the_bundled_windows_sidecar() {
    let model_root = std::env::var_os("BILICATCH_FIRE_RED_SMOKE_ROOT")
        .map(std::path::PathBuf::from)
        .expect("BILICATCH_FIRE_RED_SMOKE_ROOT");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let transcriber = FireRedSidecarTranscriber::new(
        &manifest_dir
            .join("binaries")
            .join("sherpa-onnx-offline-x86_64-pc-windows-msvc.exe"),
        &manifest_dir
            .join("binaries")
            .join("fire-red-asr2-tokens.txt"),
    )
    .unwrap();

    let output = transcriber
        .transcribe(
            TranscriberRequest {
                model_path: model_root.join("model.int8.onnx"),
                audio_path: model_root.join("test_wavs").join("0.wav"),
                output_base: model_root.join("smoke-output"),
                language: TranscriptionLanguage::Zh,
                gpu_mode: GpuMode::Auto,
                offset_ms: 0,
            },
            ProcessControl::new(),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();

    assert_eq!(output.detected_language, "zh");
    assert!(!output.segments.is_empty());
    assert!(output
        .segments
        .iter()
        .all(|segment| segment.end_ms >= segment.start_ms));
}
