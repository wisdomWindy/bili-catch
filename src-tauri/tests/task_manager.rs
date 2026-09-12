use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

use bilicatch_lib::{
    infrastructure::tasks::{PersistedTaskFile, TaskStoreLoad},
    models::{
        CreateDownloadTasksRequest, DownloadMode, DownloadTaskDraft, TaskAction,
        TaskControlRequest, TaskStatus, VideoCodec,
    },
    services::tasks::{
        ExecutionUpdate, SchedulerLimits, TaskCleanerPort, TaskEventSink, TaskManager,
        TaskSettingsPort, TaskStorePort,
    },
};
use tempfile::NamedTempFile;

#[derive(Default)]
struct MemoryStore(Mutex<Option<PersistedTaskFile>>);
impl TaskStorePort for MemoryStore {
    fn load(&self) -> Result<TaskStoreLoad, bilicatch_lib::models::AppError> {
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
    fn save(&self, file: &PersistedTaskFile) -> Result<(), bilicatch_lib::models::AppError> {
        *self.0.lock().unwrap() = Some(file.clone());
        Ok(())
    }
}

#[derive(Default)]
struct ToggleStore {
    file: Mutex<Option<PersistedTaskFile>>,
    fail: AtomicBool,
}
impl TaskStorePort for ToggleStore {
    fn load(&self) -> Result<TaskStoreLoad, bilicatch_lib::models::AppError> {
        Ok(TaskStoreLoad {
            file: self
                .file
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| PersistedTaskFile::new(0, vec![])),
            warning: None,
        })
    }
    fn save(&self, file: &PersistedTaskFile) -> Result<(), bilicatch_lib::models::AppError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(bilicatch_lib::models::AppError::internal("save failed"));
        }
        *self.file.lock().unwrap() = Some(file.clone());
        Ok(())
    }
}

#[derive(Default)]
struct CountingStore {
    file: Mutex<Option<PersistedTaskFile>>,
    saves: AtomicUsize,
}
impl TaskStorePort for CountingStore {
    fn load(&self) -> Result<TaskStoreLoad, bilicatch_lib::models::AppError> {
        Ok(TaskStoreLoad {
            file: self
                .file
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| PersistedTaskFile::new(0, vec![])),
            warning: None,
        })
    }
    fn save(&self, file: &PersistedTaskFile) -> Result<(), bilicatch_lib::models::AppError> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        *self.file.lock().unwrap() = Some(file.clone());
        Ok(())
    }
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<u64>>);
impl TaskEventSink for RecordingSink {
    fn progress(&self, event: bilicatch_lib::models::TaskProgressEvent) {
        self.0.lock().unwrap().push(event.sequence);
    }
    fn removed(&self, event: bilicatch_lib::models::TaskRemovedEvent) {
        self.0.lock().unwrap().push(event.sequence);
    }
}

struct FailingCleaner;
impl bilicatch_lib::services::tasks::TaskCleanerPort for FailingCleaner {
    fn cleanup(&self, _paths: &[String]) -> Result<(), bilicatch_lib::models::AppError> {
        Err(bilicatch_lib::models::AppError::internal("cleanup failed"))
    }
}

struct NoopCleaner;
impl TaskCleanerPort for NoopCleaner {
    fn cleanup(&self, _paths: &[String]) -> Result<(), bilicatch_lib::models::AppError> {
        Ok(())
    }
}

#[derive(Default)]
struct RecordingCleaner(Mutex<Vec<Vec<String>>>);

impl TaskCleanerPort for RecordingCleaner {
    fn cleanup(&self, paths: &[String]) -> Result<(), bilicatch_lib::models::AppError> {
        self.0.lock().unwrap().push(paths.to_vec());
        Ok(())
    }
}

struct MutableTaskSettings(Mutex<SchedulerLimits>);

impl MutableTaskSettings {
    fn new(max_concurrent: u8, connections_per_task: u8) -> Self {
        Self(Mutex::new(SchedulerLimits {
            max_concurrent,
            connections_per_task,
        }))
    }

    fn set(&self, max_concurrent: u8, connections_per_task: u8) {
        *self.0.lock().expect("settings lock") = SchedulerLimits {
            max_concurrent,
            connections_per_task,
        };
    }
}

impl TaskSettingsPort for MutableTaskSettings {
    fn scheduler_limits(&self) -> SchedulerLimits {
        *self.0.lock().expect("settings lock")
    }

    fn temporary_directory(&self) -> String {
        "D:/Temp".into()
    }
}

fn default_settings() -> Arc<dyn TaskSettingsPort> {
    Arc::new(MutableTaskSettings::new(3, 8))
}

fn draft(page: u32) -> DownloadTaskDraft {
    DownloadTaskDraft {
        canonical_url: "https://www.bilibili.com/video/BV1".into(),
        bvid: "BV1".into(),
        cid: u64::from(page),
        page,
        part_title: format!("P{page}"),
        video_title: "Example video".into(),
        part_count: 4,
        mode: DownloadMode::VideoOnly,
        output_dir: "Downloads".into(),
        quality_id: Some("80".into()),
        codec: Some(VideoCodec::Avc),
        audio_format: None,
        audio_bitrate_id: None,
    }
}

fn audio_draft(page: u32) -> DownloadTaskDraft {
    let mut value = draft(page);
    value.mode = DownloadMode::AudioOnly;
    value.quality_id = None;
    value.codec = None;
    value.audio_format = Some(bilicatch_lib::models::AudioFormat::Mp3);
    value.audio_bitrate_id = Some("320".into());
    value
}

#[test]
fn supported_claim_returns_the_full_audio_execution_context() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "supported".into(),
            drafts: vec![draft(1), audio_draft(2)],
        })
        .unwrap();

    let claimed = manager
        .claim_next_for(&[DownloadMode::AudioOnly])
        .expect("the audio task should be claimed");
    assert_eq!(claimed.task.id, "supported:1");
    assert_eq!(claimed.task.mode, DownloadMode::AudioOnly);
    assert_eq!(claimed.temporary_directory, "D:/Temp");
    assert_eq!(manager.list().tasks[0].status, TaskStatus::Queued);
}

#[test]
fn active_cancel_waits_for_executor_acknowledgement() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "cancel_ack".into(),
            drafts: vec![audio_draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();

    let pending = manager
        .control(&attempt.task_id, TaskAction::Cancel)
        .unwrap();
    assert_eq!(pending.status, TaskStatus::Downloading);
    assert_eq!(pending.control_request, TaskControlRequest::CancelRequested);
    assert_eq!(manager.pending_controls().len(), 1);

    let cancelled = manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Cancelled,
        )
        .unwrap()
        .unwrap();
    assert_eq!(cancelled.status, TaskStatus::Cancelled);
    assert_eq!(cancelled.control_request, TaskControlRequest::None);
    assert!(manager.pending_controls().is_empty());
}

#[test]
fn startup_discards_only_internal_processed_paths_and_preserves_resume_files() {
    let store = Arc::new(MemoryStore::default());
    let manager = TaskManager::new(
        store.clone(),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "recovery".into(),
            drafts: vec![audio_draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.partial");
    let checkpoint = temp.path().join("checkpoint.json");
    let processed = temp.path().join(".bilicatch-task-attempt.processing.mp3");
    manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::WorkspacePrepared {
                temporary_paths: vec![
                    source.to_string_lossy().into_owned(),
                    checkpoint.to_string_lossy().into_owned(),
                    processed.to_string_lossy().into_owned(),
                ],
            },
        )
        .unwrap();
    manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Processing { can_cancel: true },
        )
        .unwrap();
    drop(manager);
    let cleaner = Arc::new(RecordingCleaner::default());

    TaskManager::new(
        store.clone(),
        Arc::new(RecordingSink::default()),
        cleaner.clone(),
        default_settings(),
    )
    .unwrap();

    assert_eq!(
        cleaner.0.lock().unwrap().as_slice(),
        &[vec![processed.to_string_lossy().into_owned()]]
    );
    let stored = store.0.lock().unwrap();
    let paths = &stored.as_ref().unwrap().tasks[0].temporary_paths;
    assert_eq!(paths.len(), 2);
    assert!(paths.contains(&source.to_string_lossy().into_owned()));
    assert!(paths.contains(&checkpoint.to_string_lossy().into_owned()));
}

#[test]
fn create_is_atomic_idempotent_and_emits_after_save() {
    let store = Arc::new(MemoryStore::default());
    let sink = Arc::new(RecordingSink::default());
    let manager = TaskManager::new(
        store.clone(),
        sink.clone(),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    let request = CreateDownloadTasksRequest {
        request_id: "batch_1".into(),
        drafts: vec![draft(1), draft(2)],
    };

    let created = manager.create(request.clone()).unwrap();
    let reused = manager.create(request).unwrap();
    assert_eq!(created.tasks.len(), 2);
    assert!(!created.reused);
    assert!(reused.reused);
    assert_eq!(manager.list().tasks.len(), 2);
    assert_eq!(store.0.lock().unwrap().as_ref().unwrap().tasks.len(), 2);
    assert!(!sink.0.lock().unwrap().is_empty());
}

#[test]
fn scheduler_claims_only_three_fifo_tasks() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "batch_2".into(),
            drafts: vec![draft(1), draft(2), draft(3), draft(4)],
        })
        .unwrap();

    assert_eq!(manager.claim_next().unwrap().task_id, "batch_2:0");
    assert_eq!(manager.claim_next().unwrap().task_id, "batch_2:1");
    assert_eq!(manager.claim_next().unwrap().task_id, "batch_2:2");
    assert!(manager.claim_next().is_none());
}

#[test]
fn scheduler_limits_are_validated_and_applied() {
    let settings = Arc::new(MutableTaskSettings::new(2, 8));
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        settings.clone(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "limits".into(),
            drafts: vec![draft(1), draft(2), draft(3)],
        })
        .unwrap();
    assert_eq!(manager.claim_next().unwrap().connection_count, 8);
    let second = manager.claim_next().unwrap();
    settings.set(1, 32);
    assert!(manager.claim_next().is_none());
    assert_eq!(
        manager
            .list()
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Downloading)
            .count(),
        2
    );
    manager
        .apply_execution_update(
            &second.task_id,
            &second.attempt_id,
            ExecutionUpdate::Completed {
                output_path: "D:/Downloads/P2.mp4".into(),
            },
        )
        .unwrap();
    assert!(manager.claim_next().is_none());
    let first = manager
        .list()
        .tasks
        .iter()
        .find(|task| task.status == TaskStatus::Downloading)
        .expect("first task remains active")
        .id
        .clone();
    let first_attempt = "attempt-4";
    manager
        .apply_execution_update(
            &first,
            first_attempt,
            ExecutionUpdate::Completed {
                output_path: "D:/Downloads/P1.mp4".into(),
            },
        )
        .unwrap();
    assert_eq!(manager.claim_next().unwrap().connection_count, 32);
}

#[test]
fn queued_cancel_and_terminal_delete_follow_the_state_machine() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    let task = manager
        .create(CreateDownloadTasksRequest {
            request_id: "batch_3".into(),
            drafts: vec![draft(1)],
        })
        .unwrap()
        .tasks[0]
        .clone();
    let cancelled = manager.control(&task.id, TaskAction::Cancel).unwrap();
    assert_eq!(cancelled.status, TaskStatus::Cancelled);
    manager.control(&task.id, TaskAction::Delete).unwrap();
    assert!(manager.list().tasks.is_empty());
}

#[test]
fn cancel_request_wins_when_completion_arrives_before_control_is_forwarded() {
    let cleaner = Arc::new(RecordingCleaner::default());
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        cleaner.clone(),
        default_settings(),
    )
    .unwrap();
    let task = manager
        .create(CreateDownloadTasksRequest {
            request_id: "cancel-complete-race".into(),
            drafts: vec![draft(1)],
        })
        .unwrap()
        .tasks[0]
        .clone();
    let attempt = manager.claim_next().unwrap();
    let temporary_path = "D:/Downloads/.bilicatch-task.processing.mp3".to_owned();
    let output_path = "D:/Downloads/final.mp3".to_owned();
    manager
        .apply_execution_update(
            &task.id,
            &attempt.attempt_id,
            ExecutionUpdate::WorkspacePrepared {
                temporary_paths: vec![temporary_path.clone()],
            },
        )
        .unwrap();
    manager.control(&task.id, TaskAction::Cancel).unwrap();

    let updated = manager
        .apply_execution_update(
            &task.id,
            &attempt.attempt_id,
            ExecutionUpdate::Completed {
                output_path: output_path.clone(),
            },
        )
        .unwrap()
        .expect("completion update is converted into cancellation");

    assert_eq!(updated.status, TaskStatus::Cancelled);
    assert_eq!(updated.output_path, None);
    assert_eq!(updated.control_request, TaskControlRequest::None);
    assert_eq!(
        cleaner.0.lock().unwrap().as_slice(),
        &[vec![temporary_path, output_path]]
    );
}

#[test]
fn cleanup_failure_never_claims_the_task_was_cancelled() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(FailingCleaner),
        default_settings(),
    )
    .unwrap();
    let task = manager
        .create(CreateDownloadTasksRequest {
            request_id: "cleanup".into(),
            drafts: vec![draft(1)],
        })
        .unwrap()
        .tasks[0]
        .clone();
    let result = manager.control(&task.id, TaskAction::Cancel).unwrap();
    assert_eq!(result.status, TaskStatus::Failed);
    assert_eq!(result.error.unwrap().message, "cleanup failed");
}

#[test]
fn completed_delete_keeps_output_and_opener_resolves_the_stored_path() {
    let output = NamedTempFile::new().unwrap();
    let path = output.path().to_path_buf();
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "complete".into(),
            drafts: vec![draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();
    manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Completed {
                output_path: path.display().to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        manager
            .resolve_open_path(&attempt.task_id, bilicatch_lib::models::OpenTarget::File)
            .unwrap(),
        path
    );
    manager
        .control(&attempt.task_id, TaskAction::Delete)
        .unwrap();
    assert!(output.path().exists());
}

#[test]
fn failed_persistence_does_not_commit_an_in_memory_create() {
    let store = Arc::new(ToggleStore::default());
    let manager = TaskManager::new(
        store.clone(),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    store.fail.store(true, Ordering::SeqCst);
    assert!(manager
        .create(CreateDownloadTasksRequest {
            request_id: "atomic".into(),
            drafts: vec![draft(1)]
        })
        .is_err());
    assert!(manager.list().tasks.is_empty());
}

#[test]
fn failed_persistence_rolls_back_control_and_clear_transactions() {
    let store = Arc::new(ToggleStore::default());
    let manager = TaskManager::new(
        store.clone(),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    let queued = manager
        .create(CreateDownloadTasksRequest {
            request_id: "rollback".into(),
            drafts: vec![draft(1), draft(2)],
        })
        .unwrap()
        .tasks;
    let attempt = manager.claim_next().unwrap();
    manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Completed {
                output_path: "D:/Downloads/P1.mp4".into(),
            },
        )
        .unwrap();
    store.fail.store(true, Ordering::SeqCst);

    assert!(manager.control(&queued[1].id, TaskAction::Cancel).is_err());
    assert_eq!(
        manager
            .list()
            .tasks
            .iter()
            .find(|task| task.id == queued[1].id)
            .unwrap()
            .status,
        TaskStatus::Queued
    );
    assert!(manager.clear_completed().is_err());
    assert_eq!(
        manager
            .list()
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .count(),
        1
    );
}

#[test]
fn opener_rejects_non_completed_tasks_without_accepting_a_path() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    let task = manager
        .create(CreateDownloadTasksRequest {
            request_id: "open_1".into(),
            drafts: vec![draft(1)],
        })
        .unwrap()
        .tasks[0]
        .clone();

    assert!(manager
        .resolve_open_path(&task.id, bilicatch_lib::models::OpenTarget::File)
        .is_err());
}

#[test]
fn finishing_an_attempt_releases_the_fourth_fifo_task() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "slots".into(),
            drafts: vec![draft(1), draft(2), draft(3), draft(4)],
        })
        .unwrap();
    let first = manager.claim_next().unwrap();
    manager.claim_next().unwrap();
    manager.claim_next().unwrap();
    assert!(manager.claim_next().is_none());
    manager
        .apply_execution_update(
            &first.task_id,
            &first.attempt_id,
            ExecutionUpdate::Completed {
                output_path: "D:/Downloads/P1.mp4".into(),
            },
        )
        .unwrap();
    assert_eq!(manager.claim_next().unwrap().task_id, "slots:3");
}

#[test]
fn stale_and_regressing_progress_updates_are_ignored() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "progress".into(),
            drafts: vec![draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();
    assert!(manager
        .apply_execution_update(
            &attempt.task_id,
            "stale",
            ExecutionUpdate::Progress {
                percent: 50,
                bytes_downloaded: "500".into(),
                total_bytes: Some("1000".into()),
                speed_bytes_per_second: "100".into(),
                eta_seconds: Some(5)
            }
        )
        .unwrap()
        .is_none());
    manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Progress {
                percent: 50,
                bytes_downloaded: "500".into(),
                total_bytes: Some("1000".into()),
                speed_bytes_per_second: "100".into(),
                eta_seconds: Some(5),
            },
        )
        .unwrap();
    assert!(manager
        .apply_execution_update(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Progress {
                percent: 49,
                bytes_downloaded: "499".into(),
                total_bytes: Some("1000".into()),
                speed_bytes_per_second: "100".into(),
                eta_seconds: Some(6)
            }
        )
        .unwrap()
        .is_none());
}

#[test]
fn network_failures_use_one_two_four_second_backoff_then_fail() {
    let manager = TaskManager::new(
        Arc::new(MemoryStore::default()),
        Arc::new(RecordingSink::default()),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "retry".into(),
            drafts: vec![draft(1)],
        })
        .unwrap();
    let mut attempt = manager.claim_next().unwrap();
    for (index, expected) in [
        "2026-09-10T00:00:01Z",
        "2026-09-10T00:00:02Z",
        "2026-09-10T00:00:04Z",
    ]
    .iter()
    .enumerate()
    {
        let updated = manager
            .apply_execution_update_at(
                &attempt.task_id,
                &attempt.attempt_id,
                ExecutionUpdate::NetworkFailure {
                    error: bilicatch_lib::models::AppError::new(
                        bilicatch_lib::models::AppErrorCode::E001,
                        "offline",
                    ),
                },
                "2026-09-10T00:00:00Z",
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.automatic_retry_count, index as u8 + 1);
        assert_eq!(updated.next_retry_at.as_deref(), Some(*expected));
        attempt = manager.claim_next_at(expected).unwrap();
    }
    let failed = manager
        .apply_execution_update_at(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::NetworkFailure {
                error: bilicatch_lib::models::AppError::new(
                    bilicatch_lib::models::AppErrorCode::E001,
                    "offline",
                ),
            },
            "2026-09-10T00:00:04Z",
        )
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, TaskStatus::Failed);
}

#[test]
fn ordinary_progress_is_coalesced_and_explicit_flush_persists_the_latest_snapshot() {
    let store = Arc::new(CountingStore::default());
    let sink = Arc::new(RecordingSink::default());
    let manager = TaskManager::new(
        store.clone(),
        sink.clone(),
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "coalesce".into(),
            drafts: vec![draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();

    for (now, percent) in [
        ("2026-09-10T00:00:00Z", 10),
        ("2026-09-10T00:00:00.100Z", 20),
        ("2026-09-10T00:00:00.999Z", 30),
    ] {
        manager
            .apply_execution_update_at(
                &attempt.task_id,
                &attempt.attempt_id,
                ExecutionUpdate::Progress {
                    percent,
                    bytes_downloaded: (u64::from(percent) * 10).to_string(),
                    total_bytes: Some("1000".into()),
                    speed_bytes_per_second: "100".into(),
                    eta_seconds: Some(5),
                },
                now,
            )
            .unwrap();
    }

    assert_eq!(manager.list().tasks[0].progress_percent, 30);
    assert_eq!(store.saves.load(Ordering::SeqCst), 3);
    assert_eq!(sink.0.lock().unwrap().len(), 3);
    assert_eq!(
        store.file.lock().unwrap().as_ref().unwrap().tasks[0]
            .task
            .progress_percent,
        10
    );

    let flushed = manager
        .flush_progress_at(&attempt.task_id, "2026-09-10T00:00:00.999Z")
        .unwrap()
        .unwrap();
    assert_eq!(flushed.progress_percent, 30);
    assert_eq!(store.saves.load(Ordering::SeqCst), 4);
    assert_eq!(sink.0.lock().unwrap().len(), 4);
    assert_eq!(
        store.file.lock().unwrap().as_ref().unwrap().tasks[0]
            .task
            .progress_percent,
        30
    );
}

#[test]
fn terminal_update_flushes_cached_progress_immediately() {
    let store = Arc::new(CountingStore::default());
    let sink = Arc::new(RecordingSink::default());
    let manager = TaskManager::new(
        store.clone(),
        sink,
        Arc::new(NoopCleaner),
        default_settings(),
    )
    .unwrap();
    manager
        .create(CreateDownloadTasksRequest {
            request_id: "terminal-flush".into(),
            drafts: vec![draft(1)],
        })
        .unwrap();
    let attempt = manager.claim_next().unwrap();
    for (now, percent) in [
        ("2026-09-10T00:00:00Z", 10),
        ("2026-09-10T00:00:00.100Z", 75),
    ] {
        manager
            .apply_execution_update_at(
                &attempt.task_id,
                &attempt.attempt_id,
                ExecutionUpdate::Progress {
                    percent,
                    bytes_downloaded: (u64::from(percent) * 10).to_string(),
                    total_bytes: Some("1000".into()),
                    speed_bytes_per_second: "100".into(),
                    eta_seconds: Some(1),
                },
                now,
            )
            .unwrap();
    }
    manager
        .apply_execution_update_at(
            &attempt.task_id,
            &attempt.attempt_id,
            ExecutionUpdate::Completed {
                output_path: "D:/Downloads/P1.mp4".into(),
            },
            "2026-09-10T00:00:00.200Z",
        )
        .unwrap();

    let persisted = store.file.lock().unwrap().as_ref().unwrap().tasks[0]
        .task
        .clone();
    assert_eq!(persisted.status, TaskStatus::Completed);
    assert_eq!(persisted.progress_percent, 100);
    assert_eq!(store.saves.load(Ordering::SeqCst), 4);
}
