use std::{ffi::OsStr, fs, path::PathBuf};

use bilicatch_lib::infrastructure::video::{
    build_video_mux_args, validate_video_mux_inputs, DeferredFfmpegVideoMuxer,
};
use bilicatch_lib::services::{
    audio::ProcessControl,
    video::{VideoMuxRequest, VideoMuxerPort},
};

fn request(video: PathBuf, audio: PathBuf, output: PathBuf) -> VideoMuxRequest {
    VideoMuxRequest {
        video_path: video,
        audio_path: audio,
        output_path: output,
    }
}

#[test]
fn mux_uses_discrete_stream_copy_mp4_arguments() {
    let args = build_video_mux_args(&request(
        PathBuf::from("D:/video.partial"),
        PathBuf::from("D:/audio.partial"),
        PathBuf::from("D:/output.mp4"),
    ))
    .unwrap();
    let values = args
        .iter()
        .map(|value| OsStr::to_string_lossy(value.as_os_str()))
        .collect::<Vec<_>>();

    assert!(values.windows(2).any(|pair| pair == ["-map", "0:v:0"]));
    assert!(values.windows(2).any(|pair| pair == ["-map", "1:a:0"]));
    assert!(values.windows(2).any(|pair| pair == ["-c", "copy"]));
    assert!(values
        .windows(2)
        .any(|pair| pair == ["-movflags", "+faststart"]));
    assert_eq!(
        values.last().map(|value| value.as_ref()),
        Some("D:/output.mp4")
    );
}

#[test]
fn mux_rejects_empty_inputs_and_non_mp4_output() {
    let temp = tempfile::tempdir().unwrap();
    let video = temp.path().join("video.partial");
    let audio = temp.path().join("audio.partial");
    let output = temp.path().join("output.mp4");
    fs::write(&video, b"video").unwrap();
    fs::write(&audio, []).unwrap();

    assert!(validate_video_mux_inputs(&request(video.clone(), audio, output.clone())).is_err());
    fs::write(temp.path().join("audio.partial"), b"audio").unwrap();
    assert!(validate_video_mux_inputs(&request(
        video,
        temp.path().join("audio.partial"),
        temp.path().join("output.webm")
    ))
    .is_err());
    assert!(validate_video_mux_inputs(&request(
        temp.path().join("missing"),
        temp.path().join("audio.partial"),
        output
    ))
    .is_err());
}

#[test]
fn missing_ffmpeg_sidecar_maps_to_stable_e008() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let video = temp.path().join("video.partial");
        let audio = temp.path().join("audio.partial");
        fs::write(&video, b"video").unwrap();
        fs::write(&audio, b"audio").unwrap();
        let muxer = DeferredFfmpegVideoMuxer::new(temp.path().join("missing/ffmpeg.exe"));

        let error = muxer
            .mux(
                request(video, audio, temp.path().join("output.mp4")),
                ProcessControl::new(),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, bilicatch_lib::models::AppErrorCode::E008);
        assert_eq!(error.details.as_deref(), Some("FFMPEG_UNAVAILABLE"));
    });
}
