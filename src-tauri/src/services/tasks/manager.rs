use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    infrastructure::tasks::{PersistedTaskFile, StoredTaskRecord},
    models::{
        AppError, BatchTaskFailure, BatchTaskResult, ClearTasksResult, CreateDownloadTasksRequest,
        CreateTasksResult, DownloadMode, DownloadTask, OpenTarget, TaskAction, TaskControlRequest,
        TaskListSnapshot, TaskProgressEvent, TaskRemovedEvent, TaskStatus,
    },
};

use super::{
    coalescing::ProgressCoalescer, filename::audio_extension, sanitize_audio_filename,
    sanitize_filename, validate_create_request, TaskCleanerPort, TaskEventSink, TaskExecutionSpec,
    TaskSettingsPort, TaskStorePort,
};

pub struct TaskManager {
    pub(super) state: Mutex<PersistedTaskFile>,
    pub(super) store: Arc<dyn TaskStorePort>,
    pub(super) sink: Arc<dyn TaskEventSink>,
    pub(super) cleaner: Arc<dyn TaskCleanerPort>,
    pub(super) progress_coalescer: ProgressCoalescer,
    settings: Arc<dyn TaskSettingsPort>,
}

impl TaskManager {
    pub fn new(
        store: Arc<dyn TaskStorePort>,
        sink: Arc<dyn TaskEventSink>,
        cleaner: Arc<dyn TaskCleanerPort>,
        settings: Arc<dyn TaskSettingsPort>,
    ) -> Result<Self, AppError> {
        let loaded = store.load()?;
        let mut file = loaded.file;
        let mut recovery_changed = false;
        for record in &mut file.tasks {
            let processed_paths = record
                .temporary_paths
                .iter()
                .filter(|path| is_internal_processed_path(path))
                .cloned()
                .collect::<Vec<_>>();
            if processed_paths.is_empty() {
                continue;
            }
            cleaner.cleanup(&processed_paths)?;
            record
                .temporary_paths
                .retain(|path| !is_internal_processed_path(path));
            recovery_changed = true;
        }
        if recovery_changed {
            store.save(&file)?;
        }
        Ok(Self {
            state: Mutex::new(file),
            store,
            sink,
            cleaner,
            progress_coalescer: ProgressCoalescer::default(),
            settings,
        })
    }

    pub fn list(&self) -> TaskListSnapshot {
        let state = self.state.lock().expect("task manager lock poisoned");
        TaskListSnapshot {
            sequence: state.sequence,
            capacity: 100,
            tasks: public_tasks(&state),
        }
    }

    pub fn create(
        &self,
        request: CreateDownloadTasksRequest,
    ) -> Result<CreateTasksResult, AppError> {
        let mut state = self.state.lock().expect("task manager lock poisoned");
        let trimmed_id = request.request_id.trim().to_owned();
        if let Some(tasks) = existing_request_tasks(&state, &trimmed_id) {
            return Ok(CreateTasksResult {
                sequence: state.sequence,
                capacity: 100,
                tasks,
                reused: true,
            });
        }
        let validated =
            validate_create_request(&request.request_id, request.drafts, state.tasks.len())?;
        let mut candidate = state.clone();
        let now = now_rfc3339()?;
        let first_sequence = candidate.sequence + 1;
        let mut created = Vec::with_capacity(validated.drafts.len());
        for (index, draft) in validated.drafts.into_iter().enumerate() {
            let extension = match draft.mode {
                DownloadMode::AudioOnly => draft
                    .audio_format
                    .clone()
                    .map(audio_extension)
                    .unwrap_or("m4a"),
                DownloadMode::VideoAudio | DownloadMode::VideoOnly => "mp4",
            };
            let file_name = match draft.mode {
                DownloadMode::AudioOnly => sanitize_audio_filename(
                    &draft.video_title,
                    &draft.part_title,
                    &draft.bvid,
                    draft.page,
                    draft.part_count,
                    extension,
                ),
                DownloadMode::VideoAudio | DownloadMode::VideoOnly => {
                    sanitize_filename(&draft.part_title, &draft.bvid, draft.page, extension)
                }
            };
            let task = DownloadTask {
                id: format!("{}:{index}", validated.request_id),
                revision: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
                file_name,
                output_dir: draft.output_dir,
                output_path: None,
                bvid: draft.bvid,
                cid: draft.cid,
                page: draft.page,
                part_title: draft.part_title,
                mode: draft.mode,
                quality_id: draft.quality_id,
                codec: draft.codec,
                audio_format: draft.audio_format,
                audio_bitrate_id: draft.audio_bitrate_id,
                status: TaskStatus::Queued,
                control_request: TaskControlRequest::None,
                progress_percent: 0,
                bytes_downloaded: "0".into(),
                total_bytes: None,
                speed_bytes_per_second: "0".into(),
                eta_seconds: None,
                automatic_retry_count: 0,
                next_retry_at: None,
                error: None,
            };
            candidate
                .tasks
                .push(StoredTaskRecord::new(&validated.request_id, task.clone()));
            created.push(task);
        }
        candidate.sequence += created.len() as u64;
        self.store.save(&candidate)?;
        let final_sequence = candidate.sequence;
        *state = candidate;
        drop(state);
        for (index, task) in created.iter().cloned().enumerate() {
            self.sink.progress(TaskProgressEvent {
                sequence: first_sequence + index as u64,
                task,
            });
        }
        Ok(CreateTasksResult {
            sequence: final_sequence,
            capacity: 100,
            tasks: created,
            reused: false,
        })
    }

    pub fn claim_next(&self) -> Option<TaskExecutionSpec> {
        self.claim_next_for(&[
            DownloadMode::VideoAudio,
            DownloadMode::VideoOnly,
            DownloadMode::AudioOnly,
        ])
    }

    pub fn claim_next_for(&self, supported_modes: &[DownloadMode]) -> Option<TaskExecutionSpec> {
        let now = now_rfc3339().ok()?;
        self.claim_next_for_at(supported_modes, &now)
    }

    pub fn claim_next_at(&self, now: &str) -> Option<TaskExecutionSpec> {
        self.claim_next_for_at(
            &[
                DownloadMode::VideoAudio,
                DownloadMode::VideoOnly,
                DownloadMode::AudioOnly,
            ],
            now,
        )
    }

    fn claim_next_for_at(
        &self,
        supported_modes: &[DownloadMode],
        now: &str,
    ) -> Option<TaskExecutionSpec> {
        let now = parse_rfc3339(now).ok()?;
        let limits = self.settings.scheduler_limits();
        let temporary_directory = self.settings.temporary_directory();
        let mut state = self.state.lock().ok()?;
        let mut candidate = state.clone();
        let active = candidate
            .tasks
            .iter()
            .filter(|record| {
                matches!(
                    record.task.status,
                    TaskStatus::Downloading | TaskStatus::Processing
                )
            })
            .count();
        if active >= usize::from(limits.max_concurrent) {
            return None;
        }
        let index = candidate.tasks.iter().position(|record| {
            supported_modes.contains(&record.task.mode) && is_eligible(record, now)
        })?;
        candidate.sequence += 1;
        let attempt_id = format!("attempt-{}", candidate.sequence);
        let updated = {
            let record = &mut candidate.tasks[index];
            record.attempt_id = Some(attempt_id.clone());
            record.can_cancel_processing = false;
            record.task.status = TaskStatus::Downloading;
            record.task.revision += 1;
            record.task.updated_at = now_rfc3339().ok()?;
            record.task.clone()
        };
        let event = TaskProgressEvent {
            sequence: candidate.sequence,
            task: updated,
        };
        self.store.save(&candidate).ok()?;
        *state = candidate;
        self.progress_coalescer.clear(&event.task.id);
        drop(state);
        self.sink.progress(event.clone());
        Some(TaskExecutionSpec {
            task_id: event.task.id.clone(),
            attempt_id,
            connection_count: limits.connections_per_task,
            temporary_directory,
            task: event.task,
        })
    }

    pub fn pending_controls(&self) -> Vec<super::ExecutionControlSpec> {
        let state = self.state.lock().expect("task manager lock poisoned");
        state
            .tasks
            .iter()
            .filter_map(|record| {
                let action = match record.task.control_request {
                    TaskControlRequest::PauseRequested => TaskAction::Pause,
                    TaskControlRequest::CancelRequested => TaskAction::Cancel,
                    TaskControlRequest::None | TaskControlRequest::ResumeRequested => return None,
                };
                Some(super::ExecutionControlSpec {
                    task_id: record.task.id.clone(),
                    attempt_id: record.attempt_id.clone()?,
                    action,
                })
            })
            .collect()
    }

    pub fn pause_all(&self) -> BatchTaskResult {
        let ids: Vec<String> = self
            .list()
            .tasks
            .into_iter()
            .filter(|task| task.status == TaskStatus::Downloading)
            .map(|task| task.id)
            .collect();
        let mut affected = vec![];
        let mut failures = vec![];
        for id in ids {
            match self.control(&id, TaskAction::Pause) {
                Ok(_) => affected.push(id),
                Err(error) => failures.push(BatchTaskFailure { task_id: id, error }),
            }
        }
        BatchTaskResult {
            sequence: self.list().sequence,
            affected_task_ids: affected,
            failures,
        }
    }

    pub fn resolve_open_path(
        &self,
        task_id: &str,
        target: OpenTarget,
    ) -> Result<PathBuf, AppError> {
        let state = self.state.lock().expect("task manager lock poisoned");
        let task = state
            .tasks
            .iter()
            .find(|record| record.task.id == task_id)
            .map(|record| &record.task)
            .ok_or_else(|| AppError::internal("Task not found"))?;
        if task.status != TaskStatus::Completed {
            return Err(AppError::internal("Only completed tasks can be opened"));
        }
        let output = task
            .output_path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| AppError::internal("Task output path is unavailable"))?;
        if !output.is_file() {
            return Err(AppError::internal("Task output file does not exist"));
        }
        match target {
            OpenTarget::File => Ok(output),
            OpenTarget::Directory => output
                .parent()
                .map(PathBuf::from)
                .ok_or_else(|| AppError::internal("Task output directory is unavailable")),
        }
    }

    pub fn clear_completed(&self) -> Result<ClearTasksResult, AppError> {
        let mut state = self.state.lock().expect("task manager lock poisoned");
        let mut candidate = state.clone();
        let ids: Vec<String> = candidate
            .tasks
            .iter()
            .filter(|record| record.task.status == TaskStatus::Completed)
            .map(|record| record.task.id.clone())
            .collect();
        candidate
            .tasks
            .retain(|record| record.task.status != TaskStatus::Completed);
        if !ids.is_empty() {
            candidate.sequence += ids.len() as u64;
            self.store.save(&candidate)?;
            *state = candidate;
        }
        let first = state
            .sequence
            .saturating_sub(ids.len() as u64)
            .saturating_add(1);
        let sequence = state.sequence;
        drop(state);
        for (offset, id) in ids.iter().enumerate() {
            self.sink.removed(TaskRemovedEvent {
                sequence: first + offset as u64,
                task_id: id.clone(),
            });
        }
        Ok(ClearTasksResult {
            sequence,
            removed_task_ids: ids,
        })
    }
}

fn is_internal_processed_path(value: &str) -> bool {
    let path = std::path::Path::new(value);
    if !path.is_absolute() {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(".bilicatch-") && name.contains(".processing."))
}

fn public_tasks(state: &PersistedTaskFile) -> Vec<DownloadTask> {
    state
        .tasks
        .iter()
        .map(|record| record.task.clone())
        .collect()
}

fn existing_request_tasks(
    state: &PersistedTaskFile,
    request_id: &str,
) -> Option<Vec<DownloadTask>> {
    let tasks: Vec<_> = state
        .tasks
        .iter()
        .filter(|record| record.source_request_id == request_id)
        .map(|record| record.task.clone())
        .collect();
    (!tasks.is_empty()).then_some(tasks)
}

pub(super) fn now_rfc3339() -> Result<String, AppError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| AppError::internal("System clock is unavailable"))
}

pub(super) fn parse_rfc3339(value: &str) -> Result<OffsetDateTime, AppError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| AppError::internal("Invalid task timestamp"))
}

fn is_eligible(record: &StoredTaskRecord, now: OffsetDateTime) -> bool {
    let wants_resume = record.task.status == TaskStatus::Paused
        && record.task.control_request == TaskControlRequest::ResumeRequested;
    if record.task.status != TaskStatus::Queued && !wants_resume {
        return false;
    }
    record
        .task
        .next_retry_at
        .as_deref()
        .and_then(|value| parse_rfc3339(value).ok())
        .is_none_or(|retry_at| retry_at <= now)
}
