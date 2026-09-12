use time::{format_description::well_known::Rfc3339, Duration};

use crate::models::{
    AppError, DownloadTask, TaskAction, TaskControlRequest, TaskProgressEvent, TaskRemovedEvent,
    TaskStatus,
};

use super::{
    actions_for_status,
    manager::{now_rfc3339, parse_rfc3339},
    TaskManager,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionUpdate {
    Progress {
        percent: u8,
        bytes_downloaded: String,
        total_bytes: Option<String>,
        speed_bytes_per_second: String,
        eta_seconds: Option<u64>,
    },
    Paused,
    Processing {
        can_cancel: bool,
    },
    Completed {
        output_path: String,
    },
    NetworkFailure {
        error: AppError,
    },
    Failed {
        error: AppError,
    },
    WorkspacePrepared {
        temporary_paths: Vec<String>,
    },
    Cancelled,
}

impl TaskManager {
    pub fn control(&self, task_id: &str, action: TaskAction) -> Result<DownloadTask, AppError> {
        let mut state = self.state.lock().expect("task manager lock poisoned");
        let mut candidate = state.clone();
        let index = candidate
            .tasks
            .iter()
            .position(|record| record.task.id == task_id)
            .ok_or_else(|| AppError::internal("Task not found"))?;
        let current = candidate.tasks[index].task.clone();
        if !actions_for_status(current.status, candidate.tasks[index].can_cancel_processing)
            .contains(&action)
        {
            return Err(AppError::internal("Action is not valid for task state"));
        }
        if action == TaskAction::Delete {
            let removed = candidate.tasks.remove(index).task;
            candidate.sequence += 1;
            let sequence = candidate.sequence;
            self.store.save(&candidate)?;
            *state = candidate;
            self.progress_coalescer.clear(task_id);
            drop(state);
            self.sink.removed(TaskRemovedEvent {
                sequence,
                task_id: task_id.into(),
            });
            return Ok(removed);
        }
        let updated = {
            let record = &mut candidate.tasks[index];
            match action {
                TaskAction::Pause => {
                    record.task.control_request = TaskControlRequest::PauseRequested
                }
                TaskAction::Resume => {
                    record.task.control_request = TaskControlRequest::ResumeRequested
                }
                TaskAction::Retry => {
                    record.task.status = TaskStatus::Queued;
                    record.task.control_request = TaskControlRequest::None;
                    record.task.error = None;
                    record.task.next_retry_at = None;
                    record.task.automatic_retry_count = 0;
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.attempt_id = None;
                }
                TaskAction::Cancel => {
                    if record.attempt_id.is_some()
                        && matches!(
                            record.task.status,
                            TaskStatus::Downloading | TaskStatus::Processing
                        )
                    {
                        record.task.control_request = TaskControlRequest::CancelRequested;
                    } else {
                        record.attempt_id = None;
                        record.can_cancel_processing = false;
                        if let Err(error) = self.cleaner.cleanup(&record.temporary_paths) {
                            record.task.status = TaskStatus::Failed;
                            record.task.error = Some(error);
                        } else {
                            record.temporary_paths.clear();
                            record.task.status = TaskStatus::Cancelled;
                            record.task.control_request = TaskControlRequest::None;
                        }
                    }
                }
                TaskAction::Delete => unreachable!(),
            }
            record.task.revision += 1;
            record.task.updated_at = now_rfc3339()?;
            record.task.clone()
        };
        candidate.sequence += 1;
        let event = TaskProgressEvent {
            sequence: candidate.sequence,
            task: updated,
        };
        self.store.save(&candidate)?;
        *state = candidate;
        self.progress_coalescer.clear(task_id);
        drop(state);
        self.sink.progress(event.clone());
        Ok(event.task)
    }

    pub fn apply_execution_update(
        &self,
        task_id: &str,
        attempt_id: &str,
        update: ExecutionUpdate,
    ) -> Result<Option<DownloadTask>, AppError> {
        self.apply_execution_update_at(task_id, attempt_id, update, &now_rfc3339()?)
    }

    pub fn apply_execution_update_at(
        &self,
        task_id: &str,
        attempt_id: &str,
        update: ExecutionUpdate,
        now: &str,
    ) -> Result<Option<DownloadTask>, AppError> {
        let now_value = parse_rfc3339(now)?;
        let is_progress = matches!(&update, ExecutionUpdate::Progress { .. });
        let mut state = self.state.lock().expect("task manager lock poisoned");
        let mut candidate = state.clone();
        let Some(index) = candidate
            .tasks
            .iter()
            .position(|record| record.task.id == task_id)
        else {
            return Ok(None);
        };
        let record = &candidate.tasks[index];
        if record.attempt_id.as_deref() != Some(attempt_id)
            || matches!(
                record.task.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
            )
        {
            return Ok(None);
        }
        if let ExecutionUpdate::Progress {
            percent,
            ref bytes_downloaded,
            ..
        } = update
        {
            let current = record.task.bytes_downloaded.parse::<u128>().unwrap_or(0);
            let next = bytes_downloaded.parse::<u128>().ok();
            if percent > 100
                || percent < record.task.progress_percent
                || next.is_none()
                || next.unwrap() < current
            {
                return Ok(None);
            }
        }
        let updated = {
            let record = &mut candidate.tasks[index];
            match update {
                ExecutionUpdate::Progress {
                    percent,
                    bytes_downloaded,
                    total_bytes,
                    speed_bytes_per_second,
                    eta_seconds,
                } => {
                    record.task.status = TaskStatus::Downloading;
                    record.task.progress_percent = percent;
                    record.task.bytes_downloaded = bytes_downloaded;
                    record.task.total_bytes = total_bytes;
                    record.task.speed_bytes_per_second = speed_bytes_per_second;
                    record.task.eta_seconds = eta_seconds;
                }
                ExecutionUpdate::Paused => {
                    record.task.status = TaskStatus::Paused;
                    record.task.control_request = TaskControlRequest::None;
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.attempt_id = None;
                }
                ExecutionUpdate::Processing { can_cancel } => {
                    record.task.status = TaskStatus::Processing;
                    record.can_cancel_processing = can_cancel;
                }
                ExecutionUpdate::Completed { output_path } => {
                    if record.task.control_request == TaskControlRequest::CancelRequested {
                        let mut cleanup_paths = record.temporary_paths.clone();
                        cleanup_paths.push(output_path);
                        if let Err(error) = self.cleaner.cleanup(&cleanup_paths) {
                            record.task.status = TaskStatus::Failed;
                            record.task.error = Some(error);
                        } else {
                            record.temporary_paths.clear();
                            record.task.status = TaskStatus::Cancelled;
                            record.task.error = None;
                        }
                    } else {
                        record.task.status = TaskStatus::Completed;
                        record.task.progress_percent = 100;
                        record.task.output_path = Some(output_path);
                    }
                    record.task.control_request = TaskControlRequest::None;
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.task.next_retry_at = None;
                    record.attempt_id = None;
                    record.can_cancel_processing = false;
                }
                ExecutionUpdate::NetworkFailure { error } => {
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.attempt_id = None;
                    if record.task.automatic_retry_count < 3 {
                        let delay = [1_i64, 2, 4][record.task.automatic_retry_count as usize];
                        record.task.automatic_retry_count += 1;
                        record.task.status = TaskStatus::Queued;
                        record.task.next_retry_at = Some(
                            (now_value + Duration::seconds(delay))
                                .format(&Rfc3339)
                                .map_err(|_| AppError::internal("System clock is unavailable"))?,
                        );
                        record.task.error = Some(error);
                    } else {
                        record.task.status = TaskStatus::Failed;
                        record.task.next_retry_at = None;
                        record.task.error = Some(error);
                    }
                }
                ExecutionUpdate::Failed { error } => {
                    record.task.status = TaskStatus::Failed;
                    record.task.error = Some(error);
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.attempt_id = None;
                    record.can_cancel_processing = false;
                }
                ExecutionUpdate::WorkspacePrepared { temporary_paths } => {
                    record.temporary_paths = temporary_paths;
                }
                ExecutionUpdate::Cancelled => {
                    if let Err(error) = self.cleaner.cleanup(&record.temporary_paths) {
                        record.task.status = TaskStatus::Failed;
                        record.task.error = Some(error);
                    } else {
                        record.temporary_paths.clear();
                        record.task.status = TaskStatus::Cancelled;
                        record.task.error = None;
                    }
                    record.task.control_request = TaskControlRequest::None;
                    record.task.speed_bytes_per_second = "0".into();
                    record.task.eta_seconds = None;
                    record.attempt_id = None;
                    record.can_cancel_processing = false;
                }
            }
            record.task.revision += 1;
            record.task.updated_at = now.to_owned();
            record.task.clone()
        };
        if is_progress && !self.progress_coalescer.should_flush(task_id, now_value) {
            *state = candidate;
            self.progress_coalescer.mark_dirty(task_id);
            return Ok(Some(updated));
        }
        candidate.sequence += 1;
        let event = TaskProgressEvent {
            sequence: candidate.sequence,
            task: updated.clone(),
        };
        self.store.save(&candidate)?;
        *state = candidate;
        if is_progress {
            self.progress_coalescer.mark_flushed(task_id, now_value);
        } else {
            self.progress_coalescer.clear(task_id);
        }
        drop(state);
        self.sink.progress(event);
        Ok(Some(updated))
    }

    pub fn flush_progress(&self, task_id: &str) -> Result<Option<DownloadTask>, AppError> {
        self.flush_progress_at(task_id, &now_rfc3339()?)
    }

    pub fn flush_progress_at(
        &self,
        task_id: &str,
        now: &str,
    ) -> Result<Option<DownloadTask>, AppError> {
        let now_value = parse_rfc3339(now)?;
        let mut state = self.state.lock().expect("task manager lock poisoned");
        if !self.progress_coalescer.is_dirty(task_id) {
            return Ok(None);
        }
        let mut candidate = state.clone();
        let Some(task) = candidate
            .tasks
            .iter()
            .find(|record| record.task.id == task_id)
            .map(|record| record.task.clone())
        else {
            self.progress_coalescer.clear(task_id);
            return Ok(None);
        };
        candidate.sequence += 1;
        let event = TaskProgressEvent {
            sequence: candidate.sequence,
            task: task.clone(),
        };
        self.store.save(&candidate)?;
        *state = candidate;
        self.progress_coalescer.mark_flushed(task_id, now_value);
        drop(state);
        self.sink.progress(event);
        Ok(Some(task))
    }
}
