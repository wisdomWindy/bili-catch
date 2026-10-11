use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use bilicatch_lib::{
    models::{
        AppError, AppErrorCode, AsrModelId, GpuMode, MediaKind, MediaSourceInfo, TranscribeTask,
        TranscribeTaskStatus, TranscriptFormat, TranscriptResult, TranscriptSegment,
        TranscriptionLanguage, TranscriptionOptions, TranscriptionProgress, TranscriptionSource,
    },
    services::transcription::{
        TranscriptionDocument, TranscriptionEventSink, TranscriptionManager, TranscriptionStore,
    },
};

#[derive(Default)]
struct MemoryStore {
    document: Mutex<Option<TranscriptionDocument>>,
    fail_save: AtomicBool,
}

impl MemoryStore {
    fn with_document(document: TranscriptionDocument) -> Self {
        Self {
            document: Mutex::new(Some(document)),
            fail_save: AtomicBool::new(false),
        }
    }
}

impl TranscriptionStore for MemoryStore {
    fn load(&self) -> Result<Option<TranscriptionDocument>, AppError> {
        Ok(self.document.lock().expect("document lock").clone())
    }

    fn save(&self, document: &TranscriptionDocument) -> Result<(), AppError> {
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(AppError::internal("injected store failure"));
        }
        *self.document.lock().expect("document lock") = Some(document.clone());
        Ok(())
    }
}

#[derive(Default)]
struct RecordingEvents {
    changed: Mutex<Vec<(u64, String, u64)>>,
    removed: Mutex<Vec<(u64, String)>>,
}

impl TranscriptionEventSink for RecordingEvents {
    fn task_changed(&self, sequence: u64, task: &TranscribeTask) {
        self.changed
            .lock()
            .expect("changed lock")
            .push((sequence, task.id.clone(), task.revision));
    }

    fn task_removed(&self, sequence: u64, task_id: &str) {
        self.removed
            .lock()
            .expect("removed lock")
            .push((sequence, task_id.into()));
    }
}

fn task(id: &str, status: TranscribeTaskStatus) -> TranscribeTask {
    TranscribeTask {
        id: id.into(),
        revision: 0,
        source: TranscriptionSource::LocalFile {
            path: format!("D:/Media/{id}.wav"),
        },
        source_info: MediaSourceInfo {
            file_name: format!("{id}.wav"),
            path: format!("D:/Media/{id}.wav"),
            duration_seconds: 10,
            size_bytes: 100,
            media_kind: MediaKind::Audio,
            format: "wav".into(),
        },
        options: TranscriptionOptions {
            model_id: AsrModelId::Base,
            language: TranscriptionLanguage::Auto,
            output_format: TranscriptFormat::Srt,
            timestamps: true,
            punctuation: true,
            output_directory: "D:/Media/Transcribe".into(),
            gpu_mode: GpuMode::Auto,
        },
        status,
        progress: TranscriptionProgress {
            progress: 0.0,
            processed_seconds: 0.0,
            total_seconds: Some(10.0),
            speed: None,
            eta_seconds: None,
            message: String::new(),
        },
        result_id: None,
        output_path: None,
        error: None,
        cache_reused: false,
        created_at: "2026-10-10T00:00:00Z".into(),
        updated_at: "2026-10-10T00:00:00Z".into(),
    }
}

#[test]
fn transition_persists_before_emitting_and_increments_both_versions() {
    let store = Arc::new(MemoryStore::default());
    let events = Arc::new(RecordingEvents::default());
    let manager = TranscriptionManager::load(store.clone(), events.clone()).expect("manager");
    manager
        .insert(task("task-1", TranscribeTaskStatus::Pending))
        .expect("insert");

    let updated = manager
        .transition("task-1", TranscribeTaskStatus::Preprocessing, None)
        .expect("transition");

    assert_eq!(updated.revision, 1);
    assert_eq!(manager.snapshot().sequence, 2);
    assert_eq!(
        events.changed.lock().expect("changed lock").as_slice(),
        &[(1, "task-1".into(), 0), (2, "task-1".into(), 1)]
    );
    assert_eq!(
        store
            .document
            .lock()
            .expect("document lock")
            .as_ref()
            .expect("saved document")
            .tasks[0]
            .status,
        TranscribeTaskStatus::Preprocessing
    );
}

#[test]
fn failed_persistence_keeps_the_previous_snapshot_and_emits_nothing() {
    let store = Arc::new(MemoryStore::default());
    let events = Arc::new(RecordingEvents::default());
    let manager = TranscriptionManager::load(store.clone(), events.clone()).expect("manager");
    manager
        .insert(task("task-1", TranscribeTaskStatus::Pending))
        .expect("insert");
    store.fail_save.store(true, Ordering::SeqCst);

    manager
        .transition("task-1", TranscribeTaskStatus::Preprocessing, None)
        .expect_err("save failure must abort transition");

    assert_eq!(
        manager.snapshot().tasks[0].status,
        TranscribeTaskStatus::Pending
    );
    assert_eq!(events.changed.lock().expect("changed lock").len(), 1);
}

#[test]
fn startup_recovers_active_tasks_as_retryable_failures() {
    let store = Arc::new(MemoryStore::with_document(TranscriptionDocument {
        schema_version: 1,
        sequence: 4,
        tasks: vec![task("active", TranscribeTaskStatus::Transcribing)],
        results: vec![],
    }));
    let events = Arc::new(RecordingEvents::default());

    let manager = TranscriptionManager::load(store.clone(), events).expect("manager");
    let recovered = &manager.snapshot().tasks[0];

    assert_eq!(recovered.status, TranscribeTaskStatus::Failed);
    assert_eq!(recovered.revision, 1);
    assert_eq!(
        recovered.error.as_ref().unwrap().details.as_deref(),
        Some("TASK_INTERRUPTED")
    );
    assert_eq!(manager.snapshot().sequence, 5);
    assert_eq!(
        store
            .document
            .lock()
            .expect("document lock")
            .as_ref()
            .unwrap()
            .tasks[0]
            .status,
        TranscribeTaskStatus::Failed
    );
}

#[test]
fn retry_resets_failure_and_delete_is_limited_to_terminal_tasks() {
    let store = Arc::new(MemoryStore::default());
    let events = Arc::new(RecordingEvents::default());
    let manager = TranscriptionManager::load(store, events.clone()).expect("manager");
    manager
        .insert(task("failed", TranscribeTaskStatus::Pending))
        .expect("insert pending task");
    manager
        .transition(
            "failed",
            TranscribeTaskStatus::Failed,
            Some(AppError::new(AppErrorCode::E014, "failed")),
        )
        .expect("fail pending task");

    let retried = manager.retry("failed").expect("retry");
    assert_eq!(retried.status, TranscribeTaskStatus::Pending);
    assert!(retried.error.is_none());
    assert_eq!(retried.progress.progress, 0.0);
    manager
        .delete("failed")
        .expect_err("pending task cannot be deleted");
    manager
        .transition("failed", TranscribeTaskStatus::Cancelled, None)
        .expect("cancel pending");
    manager.delete("failed").expect("delete terminal task");
    assert!(manager.snapshot().tasks.is_empty());
    assert_eq!(events.removed.lock().expect("removed lock").len(), 1);
}

#[test]
fn completing_and_editing_a_result_is_atomic_and_revision_checked() {
    let store = Arc::new(MemoryStore::default());
    let events = Arc::new(RecordingEvents::default());
    let manager = TranscriptionManager::load(store, events).unwrap();
    manager
        .insert(task("saving", TranscribeTaskStatus::Pending))
        .unwrap();
    for status in [
        TranscribeTaskStatus::Preprocessing,
        TranscribeTaskStatus::LoadingModel,
        TranscribeTaskStatus::Transcribing,
        TranscribeTaskStatus::Processing,
        TranscribeTaskStatus::Saving,
    ] {
        manager.transition("saving", status, None).unwrap();
    }
    let result = TranscriptResult {
        id: "result-1".into(),
        task_id: "saving".into(),
        revision: 0,
        source_fingerprint: "fingerprint".into(),
        model_id: AsrModelId::Base,
        requested_language: TranscriptionLanguage::Auto,
        detected_language: "en".into(),
        segments: vec![TranscriptSegment {
            index: 0,
            start_ms: 10,
            end_ms: 20,
            text: "first".into(),
        }],
        plain_text: "first".into(),
        created_at: "2026-10-10T00:00:00Z".into(),
        updated_at: "2026-10-10T00:00:00Z".into(),
    };

    let completed = manager
        .complete_with_result("saving", result, "D:/out/saving.en.srt".into(), false)
        .unwrap();
    assert_eq!(completed.status, TranscribeTaskStatus::Completed);
    assert_eq!(completed.result_id.as_deref(), Some("result-1"));

    let edited = manager
        .save_edits("saving", 0, vec!["  updated  ".into()])
        .unwrap();
    assert_eq!(edited.revision, 1);
    assert_eq!(edited.segments[0].start_ms, 10);
    assert_eq!(edited.segments[0].text, "updated");
    assert_eq!(edited.plain_text, "updated");

    let conflict = manager
        .save_edits("saving", 0, vec!["stale".into()])
        .unwrap_err();
    assert_eq!(
        conflict.details.as_deref(),
        Some("RESULT_REVISION_CONFLICT")
    );
}
