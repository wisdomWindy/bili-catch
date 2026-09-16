use std::fs;

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::{
    models::{AppError, TaskProgressEvent, TaskRemovedEvent},
    services::{
        system::TaskCompletionNotification,
        tasks::{TaskCleanerPort, TaskCompletionNotifier, TaskEventSink},
    },
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

pub struct TauriTaskCompletionNotifier {
    app: AppHandle,
}

impl TauriTaskCompletionNotifier {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl TaskCompletionNotifier for TauriTaskCompletionNotifier {
    fn show(&self, notification: TaskCompletionNotification) {
        let mut builder = self
            .app
            .notification()
            .builder()
            .title(notification.title)
            .body(notification.body);
        if notification.play_sound {
            if let Some(sound) = default_completion_sound() {
                builder = builder.sound(sound);
            }
        }
        let _ = builder.show();
    }
}

#[cfg(target_os = "windows")]
fn default_completion_sound() -> Option<String> {
    Some("Default".into())
}

#[cfg(target_os = "macos")]
fn default_completion_sound() -> Option<String> {
    Some("Ping".into())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_completion_sound() -> Option<String> {
    Some("message-new-instant".into())
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
