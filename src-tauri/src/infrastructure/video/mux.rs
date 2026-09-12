use std::{ffi::OsString, path::Path, sync::Arc};

use async_trait::async_trait;

use crate::{
    infrastructure::audio::{
        ffmpeg_error, FfmpegLocator, FfmpegRunner, RunnerOutcome, TokioFfmpegRunner,
    },
    models::AppError,
    services::{
        audio::{ProcessControl, ProcessControlState},
        video::{VideoMuxOutcome, VideoMuxRequest, VideoMuxerPort},
    },
};

pub struct FfmpegVideoMuxer {
    executable: std::path::PathBuf,
    runner: Arc<dyn FfmpegRunner>,
}

pub struct DeferredFfmpegVideoMuxer {
    configured_path: std::path::PathBuf,
}

impl DeferredFfmpegVideoMuxer {
    pub fn new(configured_path: std::path::PathBuf) -> Self {
        Self { configured_path }
    }
}

impl FfmpegVideoMuxer {
    pub fn new(configured_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            executable: FfmpegLocator::resolve(configured_path)?,
            runner: Arc::new(TokioFfmpegRunner),
        })
    }

    #[cfg(test)]
    fn with_runner(executable: std::path::PathBuf, runner: Arc<dyn FfmpegRunner>) -> Self {
        Self { executable, runner }
    }
}

pub fn validate_video_mux_inputs(request: &VideoMuxRequest) -> Result<(), AppError> {
    if !is_non_empty_file(&request.video_path) || !is_non_empty_file(&request.audio_path) {
        return Err(ffmpeg_error(
            "FFMPEG_INPUT_INVALID",
            "The video mux input is unavailable",
        ));
    }
    if request
        .output_path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("mp4"))
    {
        return Err(ffmpeg_error(
            "FFMPEG_OUTPUT_INVALID",
            "The video mux output must be MP4",
        ));
    }
    Ok(())
}

pub fn build_video_mux_args(request: &VideoMuxRequest) -> Result<Vec<OsString>, AppError> {
    if request.output_path.as_os_str().is_empty() {
        return Err(ffmpeg_error(
            "FFMPEG_OUTPUT_INVALID",
            "The video mux output is unavailable",
        ));
    }
    Ok(vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        request.video_path.as_os_str().to_owned(),
        "-i".into(),
        request.audio_path.as_os_str().to_owned(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a:0".into(),
        "-c".into(),
        "copy".into(),
        "-movflags".into(),
        "+faststart".into(),
        request.output_path.as_os_str().to_owned(),
    ])
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn remove_output(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

#[async_trait]
impl VideoMuxerPort for FfmpegVideoMuxer {
    async fn mux(
        &self,
        request: VideoMuxRequest,
        control: ProcessControl,
    ) -> Result<VideoMuxOutcome, AppError> {
        if control.state() == ProcessControlState::CancelRequested {
            remove_output(&request.output_path);
            return Ok(VideoMuxOutcome::Cancelled);
        }
        validate_video_mux_inputs(&request)?;
        let args = build_video_mux_args(&request)?;
        remove_output(&request.output_path);
        let outcome = match self.runner.run(&self.executable, args, control).await {
            Ok(outcome) => outcome,
            Err(error) => {
                remove_output(&request.output_path);
                return Err(error);
            }
        };
        if outcome == RunnerOutcome::Cancelled {
            remove_output(&request.output_path);
            return Ok(VideoMuxOutcome::Cancelled);
        }
        if !is_non_empty_file(&request.output_path) {
            remove_output(&request.output_path);
            return Err(ffmpeg_error(
                "FFMPEG_OUTPUT_INVALID",
                "The muxed video output is invalid",
            ));
        }
        Ok(VideoMuxOutcome::Completed)
    }
}

#[async_trait]
impl VideoMuxerPort for DeferredFfmpegVideoMuxer {
    async fn mux(
        &self,
        request: VideoMuxRequest,
        control: ProcessControl,
    ) -> Result<VideoMuxOutcome, AppError> {
        FfmpegVideoMuxer::new(&self.configured_path)?
            .mux(request, control)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{FfmpegRunner, FfmpegVideoMuxer, RunnerOutcome};
    use crate::{
        infrastructure::audio::ffmpeg_error,
        models::AppError,
        services::{
            audio::ProcessControl,
            video::{VideoMuxOutcome, VideoMuxRequest, VideoMuxerPort},
        },
    };

    struct FakeRunner {
        result: Result<RunnerOutcome, AppError>,
        output_path: std::path::PathBuf,
        output: Option<Vec<u8>>,
        arguments: Mutex<Vec<std::ffi::OsString>>,
    }

    #[async_trait::async_trait]
    impl FfmpegRunner for FakeRunner {
        async fn run(
            &self,
            _executable: &std::path::Path,
            arguments: Vec<std::ffi::OsString>,
            _control: ProcessControl,
        ) -> Result<RunnerOutcome, AppError> {
            *self.arguments.lock().unwrap() = arguments;
            if let Some(output) = &self.output {
                std::fs::write(&self.output_path, output).unwrap();
            }
            self.result.clone()
        }
    }

    fn fixture(
        root: &std::path::Path,
        result: Result<RunnerOutcome, AppError>,
        output: Option<Vec<u8>>,
    ) -> (FfmpegVideoMuxer, VideoMuxRequest, Arc<FakeRunner>) {
        let video_path = root.join("video.partial");
        let audio_path = root.join("audio.partial");
        let output_path = root.join("processed.mp4");
        std::fs::write(&video_path, b"video").unwrap();
        std::fs::write(&audio_path, b"audio").unwrap();
        let runner = Arc::new(FakeRunner {
            result,
            output_path: output_path.clone(),
            output,
            arguments: Mutex::new(Vec::new()),
        });
        let muxer = FfmpegVideoMuxer::with_runner(root.join("ffmpeg.exe"), runner.clone());
        let request = VideoMuxRequest {
            video_path,
            audio_path,
            output_path,
        };
        (muxer, request, runner)
    }

    #[test]
    fn fake_success_requires_non_empty_output_and_receives_discrete_args() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let (muxer, request, runner) = fixture(
                temp.path(),
                Ok(RunnerOutcome::Completed),
                Some(b"mp4".to_vec()),
            );
            let outcome = muxer.mux(request, ProcessControl::new()).await.unwrap();
            assert_eq!(outcome, VideoMuxOutcome::Completed);
            let arguments = runner.arguments.lock().unwrap();
            assert!(arguments.windows(2).any(|pair| pair == ["-map", "0:v:0"]));
            assert!(arguments.windows(2).any(|pair| pair == ["-map", "1:a:0"]));
        });
    }

    #[test]
    fn fake_cancel_empty_and_process_failure_remove_partial_output() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let (muxer, request, _) = fixture(
                temp.path(),
                Ok(RunnerOutcome::Cancelled),
                Some(b"partial".to_vec()),
            );
            let outcome = muxer
                .mux(request.clone(), ProcessControl::new())
                .await
                .unwrap();
            assert_eq!(outcome, VideoMuxOutcome::Cancelled);
            assert!(!request.output_path.exists());

            let (muxer, request, _) = fixture(temp.path(), Ok(RunnerOutcome::Completed), None);
            let error = muxer
                .mux(request.clone(), ProcessControl::new())
                .await
                .unwrap_err();
            assert_eq!(error.details.as_deref(), Some("FFMPEG_OUTPUT_INVALID"));
            assert!(!request.output_path.exists());

            let (muxer, request, _) = fixture(
                temp.path(),
                Err(ffmpeg_error("FFMPEG_EXITED", "failed")),
                Some(b"partial".to_vec()),
            );
            let error = muxer
                .mux(request.clone(), ProcessControl::new())
                .await
                .unwrap_err();
            assert_eq!(error.details.as_deref(), Some("FFMPEG_EXITED"));
            assert!(!request.output_path.exists());
        });
    }
}
