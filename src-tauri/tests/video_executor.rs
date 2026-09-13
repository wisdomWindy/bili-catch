use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use bilicatch_lib::{
    infrastructure::video::FsVideoWorkspace,
    models::{
        AppError, AppErrorCode, DownloadMode, DownloadTask, TaskControlRequest, TaskStatus,
        VideoCodec,
    },
    services::{
        audio::{ExecutionReporterPort, ProcessControl},
        download::{
            ByteDownloaderPort, DownloadControl, DownloadOutcome, DownloadProgress,
            MediaDownloadRequest, ProgressSink,
        },
        tasks::{ExecutionUpdate, TaskExecutionSpec},
        video::{
            VideoExecutor, VideoMuxOutcome, VideoMuxRequest, VideoMuxerPort, VideoSourceBundle,
            VideoSourcePort, VideoSourceRequest,
        },
    },
};

struct FakeSource {
    bundle: VideoSourceBundle,
}

#[async_trait]
impl VideoSourcePort for FakeSource {
    async fn resolve(
        &self,
        _request: &VideoSourceRequest,
    ) -> Result<VideoSourceBundle, bilicatch_lib::models::AppError> {
        Ok(self.bundle.clone())
    }
}

struct CountingSource {
    bundle: VideoSourceBundle,
    resolves: Arc<AtomicUsize>,
}

#[async_trait]
impl VideoSourcePort for CountingSource {
    async fn resolve(&self, _request: &VideoSourceRequest) -> Result<VideoSourceBundle, AppError> {
        self.resolves.fetch_add(1, Ordering::SeqCst);
        Ok(self.bundle.clone())
    }
}

struct ExpiringDownloader {
    calls: AtomicUsize,
}

#[async_trait]
impl ByteDownloaderPort for ExpiringDownloader {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        _control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(AppError::new(AppErrorCode::E006, "expired media URL"));
        }
        std::fs::write(&request.workspace.source_path, b"track").unwrap();
        progress.report(DownloadProgress {
            downloaded_bytes: 5,
            total_bytes: Some(5),
            speed_bytes_per_second: 5,
            eta_seconds: Some(0),
        });
        Ok(DownloadOutcome::Completed)
    }
}

struct FakeDownloader {
    connections: Mutex<Vec<u8>>,
}

#[async_trait]
impl ByteDownloaderPort for FakeDownloader {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        _control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, bilicatch_lib::models::AppError> {
        self.connections
            .lock()
            .unwrap()
            .push(request.connection_count);
        std::fs::write(&request.workspace.source_path, b"track").unwrap();
        progress.report(DownloadProgress {
            downloaded_bytes: 5,
            total_bytes: Some(5),
            speed_bytes_per_second: 5,
            eta_seconds: Some(0),
        });
        Ok(DownloadOutcome::Completed)
    }
}

struct FakeMuxer {
    calls: Mutex<u32>,
}

#[async_trait]
impl VideoMuxerPort for FakeMuxer {
    async fn mux(
        &self,
        request: VideoMuxRequest,
        _control: ProcessControl,
    ) -> Result<VideoMuxOutcome, bilicatch_lib::models::AppError> {
        *self.calls.lock().unwrap() += 1;
        std::fs::write(request.output_path, b"muxed").unwrap();
        Ok(VideoMuxOutcome::Completed)
    }
}

struct RecordingReporter {
    updates: Mutex<Vec<ExecutionUpdate>>,
}

impl ExecutionReporterPort for RecordingReporter {
    fn report(
        &self,
        _task_id: &str,
        _attempt_id: &str,
        update: ExecutionUpdate,
    ) -> Result<bool, bilicatch_lib::models::AppError> {
        self.updates.lock().unwrap().push(update);
        Ok(true)
    }
}

fn source(
    kind: bilicatch_lib::services::download::MediaKind,
    id: u32,
    mime: &str,
) -> bilicatch_lib::services::download::MediaSourceCandidate {
    bilicatch_lib::services::download::MediaSourceCandidate {
        id,
        kind,
        bandwidth: 1,
        primary_url: "https://example.com/media".into(),
        backup_urls: vec![],
        mime_type: mime.into(),
        codecs: "avc1".into(),
        content_length: Some(5),
        etag: None,
    }
}

fn spec(mode: DownloadMode, temporary: &Path, output: &Path) -> TaskExecutionSpec {
    TaskExecutionSpec {
        task_id: "task-1".into(),
        attempt_id: "attempt-1".into(),
        connection_count: 4,
        temporary_directory: temporary.to_string_lossy().into_owned(),
        task: DownloadTask {
            id: "task-1".into(),
            revision: 1,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            file_name: "episode.mp4".into(),
            output_dir: output.to_string_lossy().into_owned(),
            output_path: None,
            bvid: "BV1xx411c7BF".into(),
            cid: 1,
            page: 1,
            part_title: "Episode".into(),
            mode,
            quality_id: Some("80".into()),
            codec: Some(VideoCodec::Avc),
            audio_format: None,
            audio_bitrate_id: None,
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
        },
    }
}

fn executor(
    muxer: Arc<FakeMuxer>,
    downloader: Arc<FakeDownloader>,
    reporter: Arc<RecordingReporter>,
) -> VideoExecutor {
    VideoExecutor::new(
        Arc::new(FakeSource {
            bundle: VideoSourceBundle {
                video: source(
                    bilicatch_lib::services::download::MediaKind::Video,
                    80,
                    "video/mp4",
                ),
                audio: Some(source(
                    bilicatch_lib::services::download::MediaKind::Audio,
                    30216,
                    "audio/mp4",
                )),
            },
        }),
        downloader,
        Arc::new(FsVideoWorkspace),
        muxer,
        reporter,
    )
}

#[test]
fn video_only_finalizes_without_mux() {
    tauri::async_runtime::block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let muxer = Arc::new(FakeMuxer {
            calls: Mutex::new(0),
        });
        let reporter = Arc::new(RecordingReporter {
            updates: Mutex::new(Vec::new()),
        });
        let result = executor(
            muxer.clone(),
            Arc::new(FakeDownloader {
                connections: Mutex::new(Vec::new()),
            }),
            reporter.clone(),
        )
        .execute(spec(
            DownloadMode::VideoOnly,
            temporary.path(),
            output.path(),
        ))
        .await;

        assert!(result.is_ok());
        assert_eq!(*muxer.calls.lock().unwrap(), 0);
        let completed_path = reporter
            .updates
            .lock()
            .unwrap()
            .iter()
            .find_map(|update| match update {
                ExecutionUpdate::Completed { output_path } => {
                    Some(Path::new(output_path).to_path_buf())
                }
                _ => None,
            })
            .expect("video-only execution should report a completed output path");
        assert!(
            std::fs::metadata(&completed_path).is_ok_and(|metadata| metadata.len() > 0),
            "finalized output is empty or missing: {completed_path:?}"
        );
    });
}

#[test]
fn video_audio_reports_processing_muxes_once_and_splits_budget() {
    tauri::async_runtime::block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let muxer = Arc::new(FakeMuxer {
            calls: Mutex::new(0),
        });
        let downloader = Arc::new(FakeDownloader {
            connections: Mutex::new(Vec::new()),
        });
        let reporter = Arc::new(RecordingReporter {
            updates: Mutex::new(Vec::new()),
        });
        let result = executor(muxer.clone(), downloader.clone(), reporter.clone())
            .execute(spec(
                DownloadMode::VideoAudio,
                temporary.path(),
                output.path(),
            ))
            .await;

        assert!(result.is_ok());
        assert_eq!(*muxer.calls.lock().unwrap(), 1);
        assert_eq!(*downloader.connections.lock().unwrap(), vec![2, 2]);
        assert!(reporter
            .updates
            .lock()
            .unwrap()
            .iter()
            .any(|update| matches!(update, ExecutionUpdate::Processing { .. })));
        let completed_path = reporter
            .updates
            .lock()
            .unwrap()
            .iter()
            .find_map(|update| match update {
                ExecutionUpdate::Completed { output_path } => {
                    Some(Path::new(output_path).to_path_buf())
                }
                _ => None,
            })
            .expect("video-audio execution should report a completed output path");
        assert!(
            std::fs::metadata(&completed_path).is_ok_and(|metadata| metadata.len() > 0),
            "muxed output is empty or missing: {completed_path:?}"
        );
    });
}

#[test]
fn stale_video_source_is_resolved_again_before_retrying_download() {
    tauri::async_runtime::block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let resolves = Arc::new(AtomicUsize::new(0));
        let source = Arc::new(CountingSource {
            bundle: VideoSourceBundle {
                video: source(
                    bilicatch_lib::services::download::MediaKind::Video,
                    80,
                    "video/mp4",
                ),
                audio: None,
            },
            resolves: resolves.clone(),
        });
        let reporter = Arc::new(RecordingReporter {
            updates: Mutex::new(Vec::new()),
        });
        let muxer = Arc::new(FakeMuxer {
            calls: Mutex::new(0),
        });
        let executor = VideoExecutor::new(
            source,
            Arc::new(ExpiringDownloader {
                calls: AtomicUsize::new(0),
            }),
            Arc::new(FsVideoWorkspace),
            muxer,
            reporter.clone(),
        );

        executor
            .execute(spec(
                DownloadMode::VideoOnly,
                temporary.path(),
                output.path(),
            ))
            .await
            .unwrap();

        assert_eq!(resolves.load(Ordering::SeqCst), 2);
        assert!(reporter
            .updates
            .lock()
            .unwrap()
            .iter()
            .any(|update| matches!(update, ExecutionUpdate::Completed { .. })));
    });
}
