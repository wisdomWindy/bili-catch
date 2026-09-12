use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    infrastructure::video::{
        VideoTrack, VideoWorkspace, VideoWorkspacePaths, VideoWorkspaceRequest,
    },
    models::{AppError, AppErrorCode, DownloadMode},
    services::{
        audio::{ExecutionReporterPort, ProcessControl},
        download::{ByteDownloaderPort, DownloadControl, DownloadOutcome},
        tasks::{ExecutionUpdate, TaskExecutionSpec},
    },
};

use super::progress::VideoProgressState;
use super::track::{connection_budget, registered_paths};
use super::{
    VideoMuxOutcome, VideoMuxRequest, VideoMuxerPort, VideoSourcePort, VideoSourceRequest,
};

#[derive(Clone)]
pub struct VideoExecutor {
    pub(super) source: Arc<dyn VideoSourcePort>,
    pub(super) downloader: Arc<dyn ByteDownloaderPort>,
    pub(super) workspace: Arc<dyn VideoWorkspace>,
    muxer: Arc<dyn VideoMuxerPort>,
    pub(super) reporter: Arc<dyn ExecutionReporterPort>,
    pub(super) active: Arc<Mutex<HashMap<(String, String), VideoActiveControls>>>,
}

#[derive(Clone)]
pub(super) struct VideoActiveControls {
    pub(super) download: DownloadControl,
    pub(super) process: ProcessControl,
}

impl VideoExecutor {
    pub fn new(
        source: Arc<dyn VideoSourcePort>,
        downloader: Arc<dyn ByteDownloaderPort>,
        workspace: Arc<dyn VideoWorkspace>,
        muxer: Arc<dyn VideoMuxerPort>,
        reporter: Arc<dyn ExecutionReporterPort>,
    ) -> Self {
        Self {
            source,
            downloader,
            workspace,
            muxer,
            reporter,
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute(&self, spec: TaskExecutionSpec) -> Result<(), AppError> {
        self.execute_with_controls(spec, DownloadControl::new(), ProcessControl::new())
            .await
    }

    pub(super) async fn execute_with_controls(
        &self,
        spec: TaskExecutionSpec,
        download_control: DownloadControl,
        process_control: ProcessControl,
    ) -> Result<(), AppError> {
        match self
            .run_attempt(&spec, download_control, process_control)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => {
                let update = if error.code == AppErrorCode::E009 {
                    ExecutionUpdate::NetworkFailure { error }
                } else {
                    ExecutionUpdate::Failed { error }
                };
                self.reporter
                    .report(&spec.task_id, &spec.attempt_id, update)?;
                Ok(())
            }
        }
    }

    async fn run_attempt(
        &self,
        spec: &TaskExecutionSpec,
        download_control: DownloadControl,
        process_control: ProcessControl,
    ) -> Result<(), AppError> {
        if !matches!(
            spec.task.mode,
            DownloadMode::VideoOnly | DownloadMode::VideoAudio
        ) {
            return Err(AppError::internal(
                "The video executor received an unsupported task",
            ));
        }
        let quality_id = spec
            .task
            .quality_id
            .clone()
            .ok_or_else(|| AppError::internal("The video quality is missing"))?;
        let codec = spec
            .task
            .codec
            .clone()
            .ok_or_else(|| AppError::internal("The video codec is missing"))?;
        let bundle = self
            .source
            .resolve(&VideoSourceRequest {
                bvid: spec.task.bvid.clone(),
                cid: spec.task.cid,
                mode: spec.task.mode,
                quality_id,
                codec,
            })
            .await?;
        let paths = self.workspace.prepare(&VideoWorkspaceRequest {
            task_id: spec.task_id.clone(),
            attempt_id: spec.attempt_id.clone(),
            temporary_root: PathBuf::from(&spec.temporary_directory),
            output_directory: PathBuf::from(&spec.task.output_dir),
            final_file_name: spec.task.file_name.clone(),
        })?;
        if !self.reporter.report(
            &spec.task_id,
            &spec.attempt_id,
            ExecutionUpdate::WorkspacePrepared {
                temporary_paths: registered_paths(&paths),
            },
        )? {
            return Ok(());
        }

        let progress = Arc::new(VideoProgressState::new(
            spec.task_id.clone(),
            spec.attempt_id.clone(),
            self.reporter.clone(),
            download_control.clone(),
            spec.task.progress_percent.min(90),
        ));
        if spec.task.mode == DownloadMode::VideoAudio && spec.connection_count >= 2 {
            let audio = bundle.audio.clone().ok_or_else(|| {
                AppError::new(AppErrorCode::E004, "The audio source is unavailable")
            })?;
            let video_future = self.download_track(
                spec,
                VideoTrack::Video,
                bundle.video.clone(),
                &paths,
                connection_budget(spec.connection_count, spec.task.mode, VideoTrack::Video),
                download_control.clone(),
                progress.sink(VideoTrack::Video),
            );
            let audio_future = self.download_track(
                spec,
                VideoTrack::Audio,
                audio,
                &paths,
                connection_budget(spec.connection_count, spec.task.mode, VideoTrack::Audio),
                download_control,
                progress.sink(VideoTrack::Audio),
            );
            let (video_result, audio_result) = tokio::join!(video_future, audio_future);
            match video_result? {
                DownloadOutcome::Paused => return self.report_paused(spec),
                DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
                DownloadOutcome::Completed => {}
            }
            match audio_result? {
                DownloadOutcome::Paused => return self.report_paused(spec),
                DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
                DownloadOutcome::Completed => {}
            }
            return self
                .report_processing_and_mux(spec, &paths, process_control)
                .await;
        }
        let video_result = self
            .download_track(
                spec,
                VideoTrack::Video,
                bundle.video.clone(),
                &paths,
                connection_budget(spec.connection_count, spec.task.mode, VideoTrack::Video),
                download_control.clone(),
                progress.sink(VideoTrack::Video),
            )
            .await;
        if spec.task.mode == DownloadMode::VideoOnly {
            return self.finish_video_only(spec, &paths, video_result).await;
        }
        let video_result = video_result?;
        match video_result {
            DownloadOutcome::Paused => return self.report_paused(spec),
            DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
            DownloadOutcome::Completed => {}
        }
        let audio = bundle
            .audio
            .clone()
            .ok_or_else(|| AppError::new(AppErrorCode::E004, "The audio source is unavailable"))?;
        let audio_result = self
            .download_track(
                spec,
                VideoTrack::Audio,
                audio,
                &paths,
                connection_budget(spec.connection_count, spec.task.mode, VideoTrack::Audio),
                download_control,
                progress.sink(VideoTrack::Audio),
            )
            .await?;
        match audio_result {
            DownloadOutcome::Paused => return self.report_paused(spec),
            DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
            DownloadOutcome::Completed => {}
        }
        self.report_processing_and_mux(spec, &paths, process_control)
            .await
    }

    async fn finish_video_only(
        &self,
        spec: &TaskExecutionSpec,
        paths: &VideoWorkspacePaths,
        result: Result<DownloadOutcome, AppError>,
    ) -> Result<(), AppError> {
        match result? {
            DownloadOutcome::Paused => self.report_paused(spec),
            DownloadOutcome::Cancelled => self.report_cancelled(spec, paths),
            DownloadOutcome::Completed => {
                let output_path = self.workspace.finalize_video_only(paths)?;
                let accepted = self.reporter.report(
                    &spec.task_id,
                    &spec.attempt_id,
                    ExecutionUpdate::Completed {
                        output_path: output_path.to_string_lossy().into_owned(),
                    },
                )?;
                if accepted {
                    let _ = self.workspace.cleanup(paths);
                }
                Ok(())
            }
        }
    }

    async fn report_processing_and_mux(
        &self,
        spec: &TaskExecutionSpec,
        paths: &VideoWorkspacePaths,
        process_control: ProcessControl,
    ) -> Result<(), AppError> {
        if !self.reporter.report(
            &spec.task_id,
            &spec.attempt_id,
            ExecutionUpdate::Processing { can_cancel: true },
        )? {
            return Ok(());
        }
        let outcome = self
            .muxer
            .mux(
                VideoMuxRequest {
                    video_path: paths.video_path.clone(),
                    audio_path: paths.audio_path.clone(),
                    output_path: paths.processed_path.clone(),
                },
                process_control,
            )
            .await;
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = self.workspace.discard_processed(paths);
                return Err(error);
            }
        };
        if outcome == VideoMuxOutcome::Cancelled {
            return self.report_cancelled(spec, paths);
        }
        let output_path = self.workspace.finalize_processed(paths)?;
        let accepted = self.reporter.report(
            &spec.task_id,
            &spec.attempt_id,
            ExecutionUpdate::Completed {
                output_path: output_path.to_string_lossy().into_owned(),
            },
        )?;
        if accepted {
            let _ = self.workspace.cleanup(paths);
        }
        Ok(())
    }

    pub(super) fn active_controls(
        &self,
        task_id: &str,
        attempt_id: &str,
    ) -> Option<VideoActiveControls> {
        self.active
            .lock()
            .ok()?
            .get(&(task_id.to_owned(), attempt_id.to_owned()))
            .cloned()
    }
}
