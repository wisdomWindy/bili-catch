use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    infrastructure::audio::{AudioWorkspace, WorkspacePaths, WorkspaceRequest},
    models::{AppError, AppErrorCode, AudioOutputProfile, DownloadMode},
    services::download::{
        ByteDownloaderPort, CoverDownloaderPort, DownloadControl, DownloadOutcome,
        MediaDownloadRequest, MediaWorkspacePaths, ProgressSink,
    },
    services::tasks::{ExecutionUpdate, TaskExecutionSpec, TaskManager},
};

use super::{
    progress::AttemptProgressSink, AudioProcessRequest, AudioSourcePort, AudioSourceRequest,
    MediaProcessorPort, ProcessControl, ProcessOutcome,
};

pub trait ExecutionReporterPort: Send + Sync {
    fn report(
        &self,
        task_id: &str,
        attempt_id: &str,
        update: ExecutionUpdate,
    ) -> Result<bool, AppError>;
}

impl ExecutionReporterPort for TaskManager {
    fn report(
        &self,
        task_id: &str,
        attempt_id: &str,
        update: ExecutionUpdate,
    ) -> Result<bool, AppError> {
        Ok(self
            .apply_execution_update(task_id, attempt_id, update)?
            .is_some())
    }
}

#[derive(Clone)]
pub(super) struct ActiveControls {
    pub(super) download: DownloadControl,
    pub(super) process: ProcessControl,
}

#[derive(Clone)]
pub struct AudioExecutor {
    source: Arc<dyn AudioSourcePort>,
    downloader: Arc<dyn ByteDownloaderPort>,
    cover_downloader: Arc<dyn CoverDownloaderPort>,
    workspace: Arc<dyn AudioWorkspace>,
    processor: Arc<dyn MediaProcessorPort>,
    reporter: Arc<dyn ExecutionReporterPort>,
    pub(super) active: Arc<Mutex<HashMap<(String, String), ActiveControls>>>,
}

impl AudioExecutor {
    pub fn new(
        source: Arc<dyn AudioSourcePort>,
        downloader: Arc<dyn ByteDownloaderPort>,
        cover_downloader: Arc<dyn CoverDownloaderPort>,
        workspace: Arc<dyn AudioWorkspace>,
        processor: Arc<dyn MediaProcessorPort>,
        reporter: Arc<dyn ExecutionReporterPort>,
    ) -> Self {
        Self {
            source,
            downloader,
            cover_downloader,
            workspace,
            processor,
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
        if spec.task.mode != DownloadMode::AudioOnly {
            return Err(AppError::internal(
                "The audio executor received an unsupported task",
            ));
        }
        let format = spec
            .task
            .audio_format
            .clone()
            .ok_or_else(|| AppError::internal("The audio task format is missing"))?;
        let profile_id = spec
            .task
            .audio_bitrate_id
            .as_deref()
            .ok_or_else(|| AppError::internal("The audio output profile is missing"))?;
        let output_profile = AudioOutputProfile::try_from_selection(format, profile_id)?;
        let mut bundle = self
            .source
            .resolve(&AudioSourceRequest {
                bvid: spec.task.bvid.clone(),
                cid: spec.task.cid,
                output_profile,
            })
            .await?;
        let paths = self.workspace.prepare(&WorkspaceRequest {
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

        match self
            .cover_downloader
            .download_cover(
                &bundle.metadata.cover_url,
                &paths.cover_path,
                download_control.clone(),
            )
            .await?
        {
            DownloadOutcome::Paused => return self.report_paused(spec),
            DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
            DownloadOutcome::Completed => {}
        }

        let progress = AttemptProgressSink::new(
            spec.task_id.clone(),
            spec.attempt_id.clone(),
            self.reporter.clone(),
            download_control.clone(),
            spec.task.progress_percent.min(90),
        );
        let download = self
            .download_source(
                spec,
                output_profile,
                &paths,
                &mut bundle,
                download_control,
                &progress,
            )
            .await?;
        match download {
            DownloadOutcome::Paused => return self.report_paused(spec),
            DownloadOutcome::Cancelled => return self.report_cancelled(spec, &paths),
            DownloadOutcome::Completed => {}
        }

        if !self.reporter.report(
            &spec.task_id,
            &spec.attempt_id,
            ExecutionUpdate::Processing { can_cancel: true },
        )? {
            return Ok(());
        }
        let process_outcome = self
            .processor
            .process(
                AudioProcessRequest {
                    source_path: paths.source_path.clone(),
                    cover_path: paths.cover_path.clone(),
                    output_path: paths.processed_path.clone(),
                    output_profile,
                    source_tier: bundle.source.tier,
                    title: bundle.metadata.title,
                    uploader: bundle.metadata.uploader,
                },
                process_control,
            )
            .await;
        let process_outcome = match process_outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                self.workspace.discard_processed(&paths)?;
                return Err(error);
            }
        };
        match process_outcome {
            ProcessOutcome::Cancelled => return self.report_cancelled(spec, &paths),
            ProcessOutcome::Completed => {}
        }
        let output_path = match self.workspace.finalize(&paths) {
            Ok(path) => path,
            Err(error) => {
                self.workspace.discard_processed(&paths)?;
                return Err(error);
            }
        };
        self.workspace.cleanup(&paths)?;
        if !std::fs::metadata(&output_path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
        {
            return Err(AppError::internal("The audio output is unavailable"));
        }
        self.reporter.report(
            &spec.task_id,
            &spec.attempt_id,
            ExecutionUpdate::Completed {
                output_path: output_path.to_string_lossy().into_owned(),
            },
        )?;
        Ok(())
    }

    async fn download_source(
        &self,
        spec: &TaskExecutionSpec,
        output_profile: AudioOutputProfile,
        paths: &WorkspacePaths,
        bundle: &mut super::AudioSourceBundle,
        control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        let first = self
            .downloader
            .download(
                download_request(spec, paths, bundle),
                control.clone(),
                progress,
            )
            .await;
        match first {
            Ok(outcome) => return Ok(outcome),
            Err(error) if source_may_be_stale(error.code) => {}
            Err(error) => return Err(error),
        }
        *bundle = self
            .source
            .resolve(&AudioSourceRequest {
                bvid: spec.task.bvid.clone(),
                cid: spec.task.cid,
                output_profile,
            })
            .await?;
        self.downloader
            .download(download_request(spec, paths, bundle), control, progress)
            .await
    }

    fn report_paused(&self, spec: &TaskExecutionSpec) -> Result<(), AppError> {
        self.reporter
            .report(&spec.task_id, &spec.attempt_id, ExecutionUpdate::Paused)?;
        Ok(())
    }

    fn report_cancelled(
        &self,
        spec: &TaskExecutionSpec,
        paths: &WorkspacePaths,
    ) -> Result<(), AppError> {
        self.workspace.cleanup(paths)?;
        self.reporter
            .report(&spec.task_id, &spec.attempt_id, ExecutionUpdate::Cancelled)?;
        Ok(())
    }

    pub(super) fn active_controls(
        &self,
        task_id: &str,
        attempt_id: &str,
    ) -> Option<ActiveControls> {
        self.active
            .lock()
            .ok()?
            .get(&(task_id.to_owned(), attempt_id.to_owned()))
            .cloned()
    }
}

fn source_may_be_stale(code: AppErrorCode) -> bool {
    matches!(
        code,
        AppErrorCode::E004 | AppErrorCode::E005 | AppErrorCode::E006
    )
}

fn download_request(
    spec: &TaskExecutionSpec,
    paths: &WorkspacePaths,
    bundle: &super::AudioSourceBundle,
) -> MediaDownloadRequest {
    MediaDownloadRequest {
        task_id: spec.task_id.clone(),
        bvid: spec.task.bvid.clone(),
        cid: spec.task.cid,
        source: bundle.source.media.clone(),
        workspace: MediaWorkspacePaths {
            source_path: paths.source_path.clone(),
            checkpoint_path: paths.checkpoint_path.clone(),
        },
        connection_count: spec.connection_count,
    }
}

fn registered_paths(paths: &WorkspacePaths) -> Vec<String> {
    [
        &paths.source_path,
        &paths.cover_path,
        &paths.checkpoint_path,
        &paths.processed_path,
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect()
}
