pub mod cache;
pub mod export;
pub mod ports;
pub mod processing;
pub mod scheduler;
pub mod state_machine;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::models::{
    AppError, AppErrorCode, TranscribeTask, TranscribeTaskListSnapshot, TranscribeTaskStatus,
    TranscriptResult, TranscriptionProgress,
};

use self::state_machine::{can_retry, can_transition, is_running, is_terminal};

const TRANSCRIPTION_STORE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionDocument {
    pub schema_version: u16,
    pub sequence: u64,
    pub tasks: Vec<TranscribeTask>,
    pub results: Vec<TranscriptResult>,
}

impl Default for TranscriptionDocument {
    fn default() -> Self {
        Self {
            schema_version: TRANSCRIPTION_STORE_SCHEMA_VERSION,
            sequence: 0,
            tasks: Vec::new(),
            results: Vec::new(),
        }
    }
}

pub trait TranscriptionStore: Send + Sync {
    fn load(&self) -> Result<Option<TranscriptionDocument>, AppError>;
    fn save(&self, document: &TranscriptionDocument) -> Result<(), AppError>;
}

pub trait TranscriptionEventSink: Send + Sync {
    fn task_changed(&self, sequence: u64, task: &TranscribeTask);
    fn task_removed(&self, sequence: u64, task_id: &str);
}

pub struct TranscriptionManager {
    store: Arc<dyn TranscriptionStore>,
    events: Arc<dyn TranscriptionEventSink>,
    document: Mutex<TranscriptionDocument>,
}

impl std::fmt::Debug for TranscriptionManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TranscriptionManager")
            .finish_non_exhaustive()
    }
}

impl TranscriptionManager {
    pub fn load(
        store: Arc<dyn TranscriptionStore>,
        events: Arc<dyn TranscriptionEventSink>,
    ) -> Result<Self, AppError> {
        let mut document = store.load()?.unwrap_or_default();
        if document.schema_version != TRANSCRIPTION_STORE_SCHEMA_VERSION {
            return Err(store_error("TRANSCRIPTION_STORE_UNKNOWN_SCHEMA"));
        }
        let mut recovered = false;
        for task in &mut document.tasks {
            if is_running(task.status) {
                task.status = TranscribeTaskStatus::Failed;
                task.error = Some(AppError {
                    code: AppErrorCode::Internal,
                    message: "The transcription was interrupted before completion".into(),
                    details: Some("TASK_INTERRUPTED".into()),
                });
                task.revision = next_version(task.revision)?;
                document.sequence = next_version(document.sequence)?;
                recovered = true;
            }
        }
        if recovered {
            store.save(&document)?;
        }
        Ok(Self {
            store,
            events,
            document: Mutex::new(document),
        })
    }

    pub fn snapshot(&self) -> TranscribeTaskListSnapshot {
        let document = self
            .document
            .lock()
            .expect("transcription document lock poisoned");
        TranscribeTaskListSnapshot {
            sequence: document.sequence,
            tasks: document.tasks.clone(),
        }
    }

    pub fn insert(&self, task: TranscribeTask) -> Result<TranscribeTask, AppError> {
        if task.status != TranscribeTaskStatus::Pending {
            return Err(action_error("TASK_MUST_START_PENDING"));
        }
        self.commit_task_change(task, true)
    }

    pub fn transition(
        &self,
        task_id: &str,
        next_status: TranscribeTaskStatus,
        error: Option<AppError>,
    ) -> Result<TranscribeTask, AppError> {
        let mut current = self.find_task(task_id)?;
        if !can_transition(current.status, next_status) {
            return Err(action_error("TASK_TRANSITION_INVALID"));
        }
        current.status = next_status;
        current.error = error;
        current.revision = next_version(current.revision)?;
        self.commit_task_change(current, false)
    }

    pub fn update_progress(
        &self,
        task_id: &str,
        progress: TranscriptionProgress,
    ) -> Result<TranscribeTask, AppError> {
        let mut current = self.find_task(task_id)?;
        if is_terminal(current.status) {
            return Err(action_error("TASK_PROGRESS_TERMINAL"));
        }
        current.progress = progress;
        current.revision = next_version(current.revision)?;
        self.commit_task_change(current, false)
    }

    pub fn retry(&self, task_id: &str) -> Result<TranscribeTask, AppError> {
        let mut current = self.find_task(task_id)?;
        if !can_retry(current.status) {
            return Err(action_error("TASK_RETRY_INVALID"));
        }
        current.status = TranscribeTaskStatus::Pending;
        current.error = None;
        current.result_id = None;
        current.output_path = None;
        current.cache_reused = false;
        current.progress = TranscriptionProgress {
            progress: 0.0,
            processed_seconds: 0.0,
            total_seconds: current.progress.total_seconds,
            speed: None,
            eta_seconds: None,
            message: String::new(),
        };
        current.revision = next_version(current.revision)?;
        self.commit_task_change(current, false)
    }

    pub fn delete(&self, task_id: &str) -> Result<(), AppError> {
        let mut document = self.lock_document()?;
        let index = document
            .tasks
            .iter()
            .position(|task| task.id == task_id)
            .ok_or_else(|| action_error("TASK_NOT_FOUND"))?;
        if !is_terminal(document.tasks[index].status) {
            return Err(action_error("TASK_DELETE_NON_TERMINAL"));
        }
        let mut candidate = document.clone();
        candidate.tasks.remove(index);
        candidate.results.retain(|result| result.task_id != task_id);
        candidate.sequence = next_version(candidate.sequence)?;
        self.store.save(&candidate)?;
        *document = candidate.clone();
        self.events.task_removed(candidate.sequence, task_id);
        Ok(())
    }

    pub fn clear_completed(&self) -> Result<Vec<String>, AppError> {
        let ids = self
            .snapshot()
            .tasks
            .into_iter()
            .filter(|task| task.status == TranscribeTaskStatus::Completed)
            .map(|task| task.id)
            .collect::<Vec<_>>();
        for id in &ids {
            self.delete(id)?;
        }
        Ok(ids)
    }

    pub fn result(&self, task_id: &str) -> Result<TranscriptResult, AppError> {
        self.lock_document()?
            .results
            .iter()
            .find(|result| result.task_id == task_id)
            .cloned()
            .ok_or_else(|| result_error("RESULT_NOT_FOUND"))
    }

    pub fn find_cached_result(&self, fingerprint: &str) -> Option<TranscriptResult> {
        self.lock_document()
            .ok()?
            .results
            .iter()
            .rev()
            .find(|result| result.source_fingerprint == fingerprint)
            .cloned()
    }

    pub fn complete_with_result(
        &self,
        task_id: &str,
        result: TranscriptResult,
        output_path: String,
        cache_reused: bool,
    ) -> Result<TranscribeTask, AppError> {
        if result.task_id != task_id || result.id.trim().is_empty() || output_path.trim().is_empty()
        {
            return Err(result_error("RESULT_INVALID"));
        }
        let mut document = self.lock_document()?;
        let mut candidate = document.clone();
        if candidate.results.iter().any(|item| item.id == result.id) {
            return Err(result_error("RESULT_ALREADY_EXISTS"));
        }
        let task = candidate
            .tasks
            .iter_mut()
            .find(|task| task.id == task_id)
            .ok_or_else(|| action_error("TASK_NOT_FOUND"))?;
        if !can_transition(task.status, TranscribeTaskStatus::Completed) {
            return Err(action_error("TASK_TRANSITION_INVALID"));
        }
        task.status = TranscribeTaskStatus::Completed;
        task.result_id = Some(result.id.clone());
        task.output_path = Some(output_path);
        task.cache_reused = cache_reused;
        task.error = None;
        task.progress.progress = 1.0;
        task.revision = next_version(task.revision)?;
        let completed = task.clone();
        candidate.results.push(result);
        candidate.sequence = next_version(candidate.sequence)?;
        self.store.save(&candidate)?;
        *document = candidate.clone();
        self.events.task_changed(candidate.sequence, &completed);
        Ok(completed)
    }

    pub fn save_edits(
        &self,
        task_id: &str,
        expected_revision: u64,
        texts: Vec<String>,
    ) -> Result<TranscriptResult, AppError> {
        let mut document = self.lock_document()?;
        let mut candidate = document.clone();
        let result = candidate
            .results
            .iter_mut()
            .find(|result| result.task_id == task_id)
            .ok_or_else(|| result_error("RESULT_NOT_FOUND"))?;
        if result.revision != expected_revision {
            return Err(result_error("RESULT_REVISION_CONFLICT"));
        }
        if result.segments.len() != texts.len() {
            return Err(result_error("RESULT_SEGMENT_COUNT_MISMATCH"));
        }
        for (segment, text) in result.segments.iter_mut().zip(texts) {
            segment.text = text.trim().to_owned();
        }
        result.plain_text = result
            .segments
            .iter()
            .filter_map(|segment| {
                let text = segment.text.trim();
                (!text.is_empty()).then_some(text)
            })
            .collect::<Vec<_>>()
            .join("\n");
        result.revision = next_version(result.revision)?;
        let updated = result.clone();
        self.store.save(&candidate)?;
        *document = candidate;
        Ok(updated)
    }

    fn find_task(&self, task_id: &str) -> Result<TranscribeTask, AppError> {
        self.lock_document()?
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .ok_or_else(|| action_error("TASK_NOT_FOUND"))
    }

    fn commit_task_change(
        &self,
        task: TranscribeTask,
        is_new: bool,
    ) -> Result<TranscribeTask, AppError> {
        let mut document = self.lock_document()?;
        let mut candidate = document.clone();
        if is_new {
            if candidate
                .tasks
                .iter()
                .any(|existing| existing.id == task.id)
            {
                return Err(action_error("TASK_ALREADY_EXISTS"));
            }
            candidate.tasks.push(task.clone());
        } else {
            let existing = candidate
                .tasks
                .iter_mut()
                .find(|existing| existing.id == task.id)
                .ok_or_else(|| action_error("TASK_NOT_FOUND"))?;
            *existing = task.clone();
        }
        candidate.sequence = next_version(candidate.sequence)?;
        self.store.save(&candidate)?;
        *document = candidate.clone();
        self.events.task_changed(candidate.sequence, &task);
        Ok(task)
    }

    fn lock_document(&self) -> Result<std::sync::MutexGuard<'_, TranscriptionDocument>, AppError> {
        self.document
            .lock()
            .map_err(|_| store_error("TRANSCRIPTION_STORE_LOCK_POISONED"))
    }
}

fn next_version(value: u64) -> Result<u64, AppError> {
    value
        .checked_add(1)
        .ok_or_else(|| store_error("TRANSCRIPTION_VERSION_OVERFLOW"))
}

fn action_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: "The transcription task action is invalid".into(),
        details: Some(details.into()),
    }
}

fn store_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: "Transcription storage is unavailable".into(),
        details: Some(details.into()),
    }
}

fn result_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: "The transcript result is invalid".into(),
        details: Some(details.into()),
    }
}
