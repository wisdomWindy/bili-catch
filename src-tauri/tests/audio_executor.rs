use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use bilicatch_lib::{
    infrastructure::audio::FsAudioWorkspace,
    models::{
        AppError, AppErrorCode, AudioFormat, DownloadMode, DownloadTask, TaskControlRequest,
        TaskStatus,
    },
    services::{
        audio::{
            AudioExecutor, AudioMetadata, AudioProcessRequest, AudioSourceBundle,
            AudioSourceCandidate, AudioSourcePort, AudioSourceRequest, AudioSourceTier,
            ExecutionReporterPort, MediaProcessorPort, ProcessControl, ProcessOutcome,
        },
        download::{
            ByteDownloaderPort, CoverDownloaderPort, DownloadControl, DownloadControlState,
            DownloadOutcome, DownloadProgress, MediaDownloadRequest, MediaKind,
            MediaSourceCandidate, ProgressSink,
        },
        tasks::{ExecutionUpdate, TaskExecutionSpec, TaskExecutorPort},
    },
};

struct FakeSource {
    tier: AudioSourceTier,
}

#[async_trait]
impl AudioSourcePort for FakeSource {
    async fn resolve(&self, _request: &AudioSourceRequest) -> Result<AudioSourceBundle, AppError> {
        Ok(AudioSourceBundle {
            source: AudioSourceCandidate {
                tier: self.tier,
                media: MediaSourceCandidate {
                    id: 30216,
                    kind: MediaKind::Audio,
                    bandwidth: 64_000,
                    primary_url: "https://a.bilivideo.com/audio.m4s".into(),
                    backup_urls: vec![],
                    mime_type: "audio/mp4".into(),
                    codecs: "mp4a.40.2".into(),
                    content_length: Some(5),
                    etag: Some("fixture".into()),
                },
            },
            metadata: AudioMetadata {
                title: "Fixture Title".into(),
                uploader: "Fixture UP".into(),
                cover_url: "https://i0.hdslb.com/cover.jpg".into(),
            },
        })
    }
}

struct FakeDownloader {
    outcome: DownloadOutcome,
    error: Option<AppError>,
}

#[async_trait]
impl CoverDownloaderPort for FakeDownloader {
    async fn download_cover(
        &self,
        _url: &str,
        output_path: &std::path::Path,
        _control: DownloadControl,
    ) -> Result<DownloadOutcome, AppError> {
        std::fs::write(output_path, b"cover").unwrap();
        Ok(DownloadOutcome::Completed)
    }
}

#[async_trait]
impl ByteDownloaderPort for FakeDownloader {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        _control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        std::fs::write(&request.workspace.source_path, b"audio").unwrap();
        progress.report(DownloadProgress {
            downloaded_bytes: 5,
            total_bytes: Some(5),
            speed_bytes_per_second: 5,
            eta_seconds: Some(0),
        });
        Ok(self.outcome)
    }
}

struct FakeProcessor {
    outcome: ProcessOutcome,
}

#[async_trait]
impl MediaProcessorPort for FakeProcessor {
    async fn process(
        &self,
        request: AudioProcessRequest,
        _control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError> {
        std::fs::write(request.output_path, b"processed").unwrap();
        Ok(self.outcome)
    }
}

#[derive(Default)]
struct RecordingReporter(Mutex<Vec<ExecutionUpdate>>);

impl ExecutionReporterPort for RecordingReporter {
    fn report(
        &self,
        _task_id: &str,
        _attempt_id: &str,
        update: ExecutionUpdate,
    ) -> Result<bool, AppError> {
        self.0.lock().unwrap().push(update);
        Ok(true)
    }
}

fn task(format: AudioFormat, profile: &str, output_dir: PathBuf) -> DownloadTask {
    DownloadTask {
        id: "task-1".into(),
        revision: 1,
        created_at: "2026-09-11T00:00:00Z".into(),
        updated_at: "2026-09-11T00:00:00Z".into(),
        file_name: format!(
            "Track.{}",
            match format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::M4a => "m4a",
                AudioFormat::Flac => "flac",
            }
        ),
        output_dir: output_dir.to_string_lossy().into_owned(),
        output_path: None,
        bvid: "BV1xx411c7BF".into(),
        cid: 1001,
        page: 1,
        part_title: "P1".into(),
        mode: DownloadMode::AudioOnly,
        quality_id: None,
        codec: None,
        audio_format: Some(format),
        audio_bitrate_id: Some(profile.into()),
        status: TaskStatus::Downloading,
        control_request: TaskControlRequest::None,
        progress_percent: 0,
        bytes_downloaded: "0".into(),
        total_bytes: None,
        speed_bytes_per_second: "0".into(),
        eta_seconds: None,
        automatic_retry_count: 0,
        next_retry_at: None,
        error: None,
    }
}

fn spec(
    format: AudioFormat,
    profile: &str,
    temp: &std::path::Path,
    output: &std::path::Path,
) -> TaskExecutionSpec {
    TaskExecutionSpec {
        task_id: "task-1".into(),
        attempt_id: "attempt-1".into(),
        connection_count: 4,
        temporary_directory: temp.to_string_lossy().into_owned(),
        task: task(format, profile, output.into()),
    }
}

fn executor(
    tier: AudioSourceTier,
    download: DownloadOutcome,
    error: Option<AppError>,
    process: ProcessOutcome,
    reporter: Arc<RecordingReporter>,
) -> AudioExecutor {
    let downloader = Arc::new(FakeDownloader {
        outcome: download,
        error,
    });
    AudioExecutor::new(
        Arc::new(FakeSource { tier }),
        downloader.clone(),
        downloader,
        Arc::new(FsAudioWorkspace),
        Arc::new(FakeProcessor { outcome: process }),
        reporter,
    )
}

#[test]
fn happy_path_reports_workspace_progress_processing_and_completed_in_order() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let executor = executor(
            AudioSourceTier::Lossy,
            DownloadOutcome::Completed,
            None,
            ProcessOutcome::Completed,
            reporter.clone(),
        );

        executor
            .execute(spec(AudioFormat::Mp3, "320", temp.path(), output.path()))
            .await
            .unwrap();

        let updates = reporter.0.lock().unwrap();
        assert!(matches!(
            updates[0],
            ExecutionUpdate::WorkspacePrepared { .. }
        ));
        assert!(matches!(
            updates[1],
            ExecutionUpdate::Progress { percent: 90, .. }
        ));
        assert!(matches!(
            updates[2],
            ExecutionUpdate::Processing { can_cancel: true }
        ));
        let final_path = match &updates[3] {
            ExecutionUpdate::Completed { output_path } => PathBuf::from(output_path),
            other => panic!("unexpected final update: {other:?}"),
        };
        assert!(final_path.is_file());
        assert_eq!(std::fs::read(final_path).unwrap(), b"processed");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    });
}

#[test]
fn pause_keeps_workspace_cancel_cleans_it_and_neither_processes() {
    tauri::async_runtime::block_on(async {
        for (outcome, expected_cancelled) in [
            (DownloadOutcome::Paused, false),
            (DownloadOutcome::Cancelled, true),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let output = tempfile::tempdir().unwrap();
            let reporter = Arc::new(RecordingReporter::default());
            let executor = executor(
                AudioSourceTier::Lossy,
                outcome,
                None,
                ProcessOutcome::Completed,
                reporter.clone(),
            );
            executor
                .execute(spec(AudioFormat::M4a, "source", temp.path(), output.path()))
                .await
                .unwrap();
            let updates = reporter.0.lock().unwrap();
            assert!(
                matches!(updates.last().unwrap(), ExecutionUpdate::Paused) != expected_cancelled
            );
            assert!(updates
                .iter()
                .all(|update| !matches!(update, ExecutionUpdate::Processing { .. })));
            assert_eq!(
                std::fs::read_dir(temp.path()).unwrap().count() == 0,
                expected_cancelled
            );
        }
    });
}

#[test]
fn network_errors_use_retryable_update_while_other_errors_fail() {
    tauri::async_runtime::block_on(async {
        for (code, retryable) in [(AppErrorCode::E009, true), (AppErrorCode::E007, false)] {
            let temp = tempfile::tempdir().unwrap();
            let output = tempfile::tempdir().unwrap();
            let reporter = Arc::new(RecordingReporter::default());
            let executor = executor(
                AudioSourceTier::Lossy,
                DownloadOutcome::Completed,
                Some(AppError::new(code, "fixture")),
                ProcessOutcome::Completed,
                reporter.clone(),
            );
            executor
                .execute(spec(AudioFormat::Mp3, "192", temp.path(), output.path()))
                .await
                .unwrap();
            let updates = reporter.0.lock().unwrap();
            assert_eq!(
                matches!(
                    updates.last().unwrap(),
                    ExecutionUpdate::NetworkFailure { .. }
                ),
                retryable
            );
            assert_eq!(
                matches!(updates.last().unwrap(), ExecutionUpdate::Failed { .. }),
                !retryable
            );
        }
    });
}

#[test]
fn flac_execution_preserves_lossless_tier_and_supports_process_cancellation() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let executor = executor(
            AudioSourceTier::HiResLossless,
            DownloadOutcome::Completed,
            None,
            ProcessOutcome::Cancelled,
            reporter.clone(),
        );

        executor
            .execute(spec(
                AudioFormat::Flac,
                "lossless",
                temp.path(),
                output.path(),
            ))
            .await
            .unwrap();

        let updates = reporter.0.lock().unwrap();
        assert!(matches!(
            updates[2],
            ExecutionUpdate::Processing { can_cancel: true }
        ));
        assert!(matches!(
            updates.last().unwrap(),
            ExecutionUpdate::Cancelled
        ));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    });
}

struct BlockingDownloader {
    started: AtomicBool,
}

#[async_trait]
impl CoverDownloaderPort for BlockingDownloader {
    async fn download_cover(
        &self,
        _url: &str,
        output_path: &std::path::Path,
        _control: DownloadControl,
    ) -> Result<DownloadOutcome, AppError> {
        std::fs::write(output_path, b"cover").unwrap();
        Ok(DownloadOutcome::Completed)
    }
}

#[async_trait]
impl ByteDownloaderPort for BlockingDownloader {
    async fn download(
        &self,
        _request: MediaDownloadRequest,
        control: DownloadControl,
        _progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        self.started.store(true, Ordering::SeqCst);
        loop {
            match control.state() {
                DownloadControlState::PauseRequested => return Ok(DownloadOutcome::Paused),
                DownloadControlState::CancelRequested => return Ok(DownloadOutcome::Cancelled),
                DownloadControlState::Running => tokio::task::yield_now().await,
            }
        }
    }
}

#[test]
fn task_executor_start_is_non_blocking_and_duplicate_start_keeps_original_controls() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let downloader = Arc::new(BlockingDownloader {
            started: AtomicBool::new(false),
        });
        let executor = AudioExecutor::new(
            Arc::new(FakeSource {
                tier: AudioSourceTier::Lossy,
            }),
            downloader.clone(),
            downloader.clone(),
            Arc::new(FsAudioWorkspace),
            Arc::new(FakeProcessor {
                outcome: ProcessOutcome::Completed,
            }),
            reporter.clone(),
        );
        let execution = spec(AudioFormat::M4a, "source", temp.path(), output.path());

        TaskExecutorPort::start(&executor, execution.clone())
            .await
            .unwrap();
        while !downloader.started.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
        assert!(TaskExecutorPort::start(&executor, execution.clone())
            .await
            .is_err());
        TaskExecutorPort::pause(&executor, &execution.task_id, &execution.attempt_id)
            .await
            .unwrap();
        for _ in 0..1_000 {
            if reporter
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|update| matches!(update, ExecutionUpdate::Paused))
            {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert!(reporter
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|update| matches!(update, ExecutionUpdate::Paused)));
    });
}

struct BlockingProcessor {
    started: AtomicBool,
}

#[async_trait]
impl MediaProcessorPort for BlockingProcessor {
    async fn process(
        &self,
        request: AudioProcessRequest,
        control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError> {
        std::fs::write(request.output_path, b"partial").unwrap();
        self.started.store(true, Ordering::SeqCst);
        loop {
            if control.state()
                == bilicatch_lib::services::audio::ProcessControlState::CancelRequested
            {
                return Ok(ProcessOutcome::Cancelled);
            }
            tokio::task::yield_now().await;
        }
    }
}

#[test]
fn processing_cancel_waits_for_ack_then_cleans_before_reporting_cancelled() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let downloader = Arc::new(FakeDownloader {
            outcome: DownloadOutcome::Completed,
            error: None,
        });
        let processor = Arc::new(BlockingProcessor {
            started: AtomicBool::new(false),
        });
        let executor = AudioExecutor::new(
            Arc::new(FakeSource {
                tier: AudioSourceTier::Lossy,
            }),
            downloader.clone(),
            downloader,
            Arc::new(FsAudioWorkspace),
            processor.clone(),
            reporter.clone(),
        );
        let execution = spec(AudioFormat::Mp3, "128", temp.path(), output.path());

        TaskExecutorPort::start(&executor, execution.clone())
            .await
            .unwrap();
        while !processor.started.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
        assert!(matches!(
            reporter.0.lock().unwrap().last().unwrap(),
            ExecutionUpdate::Processing { .. }
        ));
        TaskExecutorPort::cancel(&executor, &execution.task_id, &execution.attempt_id)
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if reporter
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|update| matches!(update, ExecutionUpdate::Cancelled))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("processing cancellation should be acknowledged");

        assert!(matches!(
            reporter.0.lock().unwrap().last().unwrap(),
            ExecutionUpdate::Cancelled
        ));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
    });
}

struct RefreshingSource {
    calls: AtomicUsize,
}

#[async_trait]
impl AudioSourcePort for RefreshingSource {
    async fn resolve(&self, _request: &AudioSourceRequest) -> Result<AudioSourceBundle, AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(AudioSourceBundle {
            source: AudioSourceCandidate {
                tier: AudioSourceTier::Lossy,
                media: MediaSourceCandidate {
                    id: 30216,
                    kind: MediaKind::Audio,
                    bandwidth: 64_000,
                    primary_url: format!("https://a.bilivideo.com/audio-{call}.m4s"),
                    backup_urls: vec![],
                    mime_type: "audio/mp4".into(),
                    codecs: "mp4a.40.2".into(),
                    content_length: Some(5),
                    etag: Some("fixture".into()),
                },
            },
            metadata: AudioMetadata {
                title: "Fixture Title".into(),
                uploader: "Fixture UP".into(),
                cover_url: "https://i0.hdslb.com/cover.jpg".into(),
            },
        })
    }
}

struct ExpiringDownloader {
    calls: AtomicUsize,
}

#[async_trait]
impl CoverDownloaderPort for ExpiringDownloader {
    async fn download_cover(
        &self,
        _url: &str,
        output_path: &std::path::Path,
        _control: DownloadControl,
    ) -> Result<DownloadOutcome, AppError> {
        std::fs::write(output_path, b"cover").unwrap();
        Ok(DownloadOutcome::Completed)
    }
}

#[async_trait]
impl ByteDownloaderPort for ExpiringDownloader {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        _control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call == 1 {
            return Err(AppError::new(AppErrorCode::E006, "expired media URL"));
        }
        assert!(request.source.primary_url.ends_with("audio-2.m4s"));
        std::fs::write(&request.workspace.source_path, b"audio").unwrap();
        progress.report(DownloadProgress {
            downloaded_bytes: 5,
            total_bytes: Some(5),
            speed_bytes_per_second: 5,
            eta_seconds: Some(0),
        });
        Ok(DownloadOutcome::Completed)
    }
}

#[test]
fn expired_source_is_resolved_once_more_before_completing_the_attempt() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let source = Arc::new(RefreshingSource {
            calls: AtomicUsize::new(0),
        });
        let downloader = Arc::new(ExpiringDownloader {
            calls: AtomicUsize::new(0),
        });
        let executor = AudioExecutor::new(
            source.clone(),
            downloader.clone(),
            downloader.clone(),
            Arc::new(FsAudioWorkspace),
            Arc::new(FakeProcessor {
                outcome: ProcessOutcome::Completed,
            }),
            reporter.clone(),
        );

        executor
            .execute(spec(AudioFormat::Mp3, "320", temp.path(), output.path()))
            .await
            .unwrap();

        assert_eq!(source.calls.load(Ordering::SeqCst), 2);
        assert_eq!(downloader.calls.load(Ordering::SeqCst), 2);
        assert!(matches!(
            reporter.0.lock().unwrap().last(),
            Some(ExecutionUpdate::Completed { .. })
        ));
    });
}

struct PartialFailingProcessor;

#[async_trait]
impl MediaProcessorPort for PartialFailingProcessor {
    async fn process(
        &self,
        request: AudioProcessRequest,
        _control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError> {
        std::fs::write(request.output_path, b"partial output").unwrap();
        Err(AppError::new(
            AppErrorCode::E008,
            "fixture processing failure",
        ))
    }
}

#[test]
fn processing_failure_removes_partial_output_but_preserves_download_workspace() {
    tauri::async_runtime::block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let reporter = Arc::new(RecordingReporter::default());
        let downloader = Arc::new(FakeDownloader {
            outcome: DownloadOutcome::Completed,
            error: None,
        });
        let executor = AudioExecutor::new(
            Arc::new(FakeSource {
                tier: AudioSourceTier::Lossy,
            }),
            downloader.clone(),
            downloader,
            Arc::new(FsAudioWorkspace),
            Arc::new(PartialFailingProcessor),
            reporter.clone(),
        );

        executor
            .execute(spec(AudioFormat::Mp3, "192", temp.path(), output.path()))
            .await
            .unwrap();

        assert!(matches!(
            reporter.0.lock().unwrap().last(),
            Some(ExecutionUpdate::Failed { error }) if error.code == AppErrorCode::E008
        ));
        assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
        let task_roots = std::fs::read_dir(temp.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(task_roots.len(), 1);
        assert!(task_roots[0].path().join("source.partial").is_file());
    });
}
