use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bilicatch_lib::{
    infrastructure::tasks::{PersistedTaskFile, TaskStoreLoad},
    models::{
        AppError, AudioFormat, CreateDownloadTasksRequest, DownloadMode, DownloadTaskDraft,
        TaskAction, TaskProgressEvent, TaskRemovedEvent, TaskStatus,
    },
    services::{
        download_runtime::DownloadRuntimeRunner,
        tasks::{
            SchedulerLimits, TaskCleanerPort, TaskEventSink, TaskExecutionSpec, TaskExecutorPort,
            TaskManager, TaskSettingsPort, TaskStorePort,
        },
    },
};

#[derive(Default)]
struct MemoryStore(Mutex<Option<PersistedTaskFile>>);

impl TaskStorePort for MemoryStore {
    fn load(&self) -> Result<TaskStoreLoad, AppError> {
        Ok(TaskStoreLoad {
            file: self
                .0
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| PersistedTaskFile::new(0, vec![])),
            warning: None,
        })
    }

    fn save(&self, file: &PersistedTaskFile) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(file.clone());
        Ok(())
    }
}

struct NoopSink;
impl TaskEventSink for NoopSink {
    fn progress(&self, _event: TaskProgressEvent) {}
    fn removed(&self, _event: TaskRemovedEvent) {}
}

struct NoopCleaner;
impl TaskCleanerPort for NoopCleaner {
    fn cleanup(&self, _paths: &[String]) -> Result<(), AppError> {
        Ok(())
    }
}

struct Settings;
impl TaskSettingsPort for Settings {
    fn scheduler_limits(&self) -> SchedulerLimits {
        SchedulerLimits {
            max_concurrent: 3,
            connections_per_task: 8,
        }
    }

    fn temporary_directory(&self) -> String {
        "D:/Temp".into()
    }
}

#[derive(Default)]
struct RecordingExecutor {
    started: Mutex<Vec<TaskExecutionSpec>>,
    cancelled: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl TaskExecutorPort for RecordingExecutor {
    fn is_available(&self) -> bool {
        true
    }

    fn supports(&self, mode: DownloadMode) -> bool {
        mode == DownloadMode::AudioOnly
    }

    async fn start(&self, spec: TaskExecutionSpec) -> Result<(), AppError> {
        self.started.lock().unwrap().push(spec);
        Ok(())
    }

    async fn pause(&self, _task_id: &str, _attempt_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn cancel(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        self.cancelled
            .lock()
            .unwrap()
            .push((task_id.into(), attempt_id.into()));
        Ok(())
    }
}

fn draft(mode: DownloadMode, page: u32) -> DownloadTaskDraft {
    DownloadTaskDraft {
        canonical_url: "https://www.bilibili.com/video/BV1".into(),
        bvid: "BV1".into(),
        cid: u64::from(page),
        page,
        part_title: format!("P{page}"),
        video_title: "Example".into(),
        part_count: 2,
        mode,
        output_dir: "D:/Downloads".into(),
        quality_id: (mode != DownloadMode::AudioOnly).then(|| "80".into()),
        codec: (mode != DownloadMode::AudioOnly).then_some(bilicatch_lib::models::VideoCodec::Avc),
        audio_format: (mode == DownloadMode::AudioOnly).then_some(AudioFormat::Mp3),
        audio_bitrate_id: (mode == DownloadMode::AudioOnly).then(|| "320".into()),
    }
}

#[tokio::test]
async fn runner_claims_only_supported_tasks_and_forwards_active_cancel() {
    let manager = Arc::new(
        TaskManager::new(
            Arc::new(MemoryStore::default()),
            Arc::new(NoopSink),
            Arc::new(NoopCleaner),
            Arc::new(Settings),
        )
        .unwrap(),
    );
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "runtime".into(),
            drafts: vec![
                draft(DownloadMode::VideoOnly, 1),
                draft(DownloadMode::AudioOnly, 2),
            ],
        })
        .unwrap();
    let executor = Arc::new(RecordingExecutor::default());
    let runner = DownloadRuntimeRunner::new(manager.clone(), executor.clone());

    runner.tick().await;
    let tasks = manager.list().tasks;
    assert_eq!(tasks[0].status, TaskStatus::Queued);
    assert_eq!(tasks[1].status, TaskStatus::Downloading);
    assert_eq!(executor.started.lock().unwrap().len(), 1);

    manager.control(&tasks[1].id, TaskAction::Cancel).unwrap();
    runner.tick().await;
    assert_eq!(executor.cancelled.lock().unwrap().len(), 1);
    assert_eq!(manager.list().tasks[1].status, TaskStatus::Downloading);
}
