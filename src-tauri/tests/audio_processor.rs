use std::{ffi::OsString, fs, path::PathBuf};

use bilicatch_lib::{
    infrastructure::audio::{build_ffmpeg_args, FfmpegLocator, FfmpegMediaProcessor},
    models::{AppErrorCode, AudioOutputProfile},
    services::audio::{
        AudioProcessRequest, AudioSourceTier, MediaProcessorPort, ProcessControl,
        ProcessControlState, ProcessOutcome,
    },
};

fn request(profile: AudioOutputProfile, tier: AudioSourceTier) -> AudioProcessRequest {
    AudioProcessRequest {
        source_path: PathBuf::from("D:/temp/source.m4s"),
        cover_path: PathBuf::from("D:/temp/cover.jpg"),
        output_path: PathBuf::from("D:/output/.processing.mp3"),
        output_profile: profile,
        source_tier: tier,
        title: "Title; --help".into(),
        uploader: "UP Creator".into(),
    }
}

fn values(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn mp3_arguments_are_discrete_and_use_only_approved_bitrates() {
    let args = values(
        &build_ffmpeg_args(&request(
            AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            AudioSourceTier::Lossy,
        ))
        .unwrap(),
    );

    assert_eq!(
        &args[..4],
        ["-nostdin", "-hide_banner", "-loglevel", "error"]
    );
    assert!(args.windows(2).any(|pair| pair == ["-c:a", "libmp3lame"]));
    assert!(args.windows(2).any(|pair| pair == ["-b:a", "320k"]));
    assert!(args.contains(&"title=Title; --help".into()));
    assert!(args.contains(&"artist=UP Creator".into()));
    assert_eq!(args.last().unwrap(), "D:/output/.processing.mp3");
}

#[test]
fn m4a_copies_audio_and_flac_requires_a_real_lossless_source() {
    let m4a = values(
        &build_ffmpeg_args(&request(
            AudioOutputProfile::M4aOriginal,
            AudioSourceTier::Lossy,
        ))
        .unwrap(),
    );
    assert!(m4a.windows(2).any(|pair| pair == ["-c:a", "copy"]));

    let error = build_ffmpeg_args(&request(
        AudioOutputProfile::FlacLossless,
        AudioSourceTier::Lossy,
    ))
    .err()
    .unwrap();
    assert_eq!(error.code, AppErrorCode::E008);

    let flac = values(
        &build_ffmpeg_args(&request(
            AudioOutputProfile::FlacLossless,
            AudioSourceTier::HiResLossless,
        ))
        .unwrap(),
    );
    assert!(flac.windows(2).any(|pair| pair == ["-c:a", "flac"]));
}

#[test]
fn locator_accepts_only_an_explicit_existing_absolute_ffmpeg_file() {
    let temp = tempfile::tempdir().unwrap();
    let executable = temp.path().join("ffmpeg.exe");
    fs::write(&executable, b"fixture").unwrap();

    assert_eq!(FfmpegLocator::resolve(&executable).unwrap(), executable);
    for rejected in [
        PathBuf::from("ffmpeg"),
        temp.path().join("missing-ffmpeg.exe"),
        temp.path().to_path_buf(),
    ] {
        let error = FfmpegLocator::resolve(&rejected).unwrap_err();
        assert_eq!(error.code, AppErrorCode::E008);
        assert_eq!(error.details.as_deref(), Some("FFMPEG_UNAVAILABLE"));
    }
}

#[test]
fn process_control_is_shared_and_cancel_wins_over_pause() {
    let control = ProcessControl::new();
    let observer = control.clone();
    control.request_pause();
    assert_eq!(observer.state(), ProcessControlState::PauseRequested);
    observer.request_cancel();
    assert_eq!(control.state(), ProcessControlState::CancelRequested);
}

fn process_fixture(
    root: &std::path::Path,
    executable: &std::path::Path,
) -> (FfmpegMediaProcessor, AudioProcessRequest) {
    let source = root.join("source.m4s");
    let cover = root.join("cover.jpg");
    fs::write(&source, b"audio").unwrap();
    fs::write(&cover, b"cover").unwrap();
    let processor = FfmpegMediaProcessor::new(executable).unwrap();
    let request = AudioProcessRequest {
        source_path: source,
        cover_path: cover,
        output_path: root.join("processed.mp3"),
        output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
        source_tier: AudioSourceTier::Lossy,
        title: "Fixture".into(),
        uploader: "UP".into(),
    };
    (processor, request)
}

#[test]
fn pre_cancelled_processing_acknowledges_without_spawning() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("ffmpeg.exe");
        fs::write(&executable, b"not executable").unwrap();
        let (processor, request) = process_fixture(temp.path(), &executable);
        let control = ProcessControl::new();
        control.request_cancel();

        let outcome = processor.process(request, control).await.unwrap();

        assert_eq!(outcome, ProcessOutcome::Cancelled);
    });
}

#[test]
fn invalid_inputs_and_spawn_failures_are_safe_e008_errors() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("ffmpeg.exe");
        fs::write(&executable, b"not executable").unwrap();
        let (processor, request) = process_fixture(temp.path(), &executable);
        fs::remove_file(&request.cover_path).unwrap();
        let error = processor
            .process(request.clone(), ProcessControl::new())
            .await
            .unwrap_err();
        assert_eq!(error.code, AppErrorCode::E008);
        assert_eq!(error.details.as_deref(), Some("FFMPEG_INPUT_INVALID"));

        fs::write(&request.cover_path, b"cover").unwrap();
        fs::remove_file(&executable).unwrap();
        let error = processor
            .process(request, ProcessControl::new())
            .await
            .unwrap_err();
        assert_eq!(error.code, AppErrorCode::E008);
        assert_eq!(error.details.as_deref(), Some("FFMPEG_UNAVAILABLE"));
        assert!(!error
            .message
            .contains(temp.path().to_string_lossy().as_ref()));
    });
}
