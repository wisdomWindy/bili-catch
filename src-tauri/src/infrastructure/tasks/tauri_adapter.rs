use std::fs;

use tauri::{AppHandle, Emitter};

use crate::{
    models::{AppError, TaskProgressEvent, TaskRemovedEvent},
    services::tasks::{TaskCleanerPort, TaskEventSink},
};

pub const TASK_PROGRESS_EVENT: &str = "download://progress";
pub const TASK_REMOVED_EVENT: &str = "download://removed";

pub struct TauriTaskEventSink {
    app: AppHandle,
}

impl TauriTaskEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl TaskEventSink for TauriTaskEventSink {
    fn progress(&self, event: TaskProgressEvent) {
        let _ = self.app.emit(TASK_PROGRESS_EVENT, event);
    }

    fn removed(&self, event: TaskRemovedEvent) {
        let _ = self.app.emit(TASK_REMOVED_EVENT, event);
    }
}

pub struct FileTaskCleaner;
impl TaskCleanerPort for FileTaskCleaner {
    fn cleanup(&self, paths: &[String]) -> Result<(), AppError> {
        for path in paths {
            let path = std::path::Path::new(path);
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|_| AppError::internal("Unable to remove temporary task file"))?;
            }
        }
        Ok(())
    }
}
