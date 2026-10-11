use std::sync::Arc;

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::{
    models::{
        AsrModel, ModelDownloadProgressEvent, TranscribeTask, TranscribeTaskStatus,
        TranscriptionProgressEvent, TranscriptionRemovedEvent,
    },
    services::{settings::SettingsManager, transcription::TranscriptionEventSink},
};

use super::runtime::ModelEventSink;

pub struct TauriTranscriptionEventSink<R: Runtime> {
    app: AppHandle<R>,
    settings: Arc<SettingsManager>,
}

impl<R: Runtime> TauriTranscriptionEventSink<R> {
    pub fn new(app: AppHandle<R>, settings: Arc<SettingsManager>) -> Self {
        Self { app, settings }
    }

    fn notify_terminal(&self, task: &TranscribeTask) {
        let values = self.settings.snapshot().values;
        if !values.notify_on_complete {
            return;
        }
        let chinese = values.locale == "zh-CN";
        let (title, body) = match task.status {
            TranscribeTaskStatus::Completed if chinese => (
                "转写完成",
                format!("{} 已完成转写", task.source_info.file_name),
            ),
            TranscribeTaskStatus::Completed => (
                "Transcription completed",
                format!("{} has been transcribed", task.source_info.file_name),
            ),
            TranscribeTaskStatus::Failed if chinese => (
                "转写失败",
                format!("{} 转写失败，可在任务列表重试", task.source_info.file_name),
            ),
            TranscribeTaskStatus::Failed => (
                "Transcription failed",
                format!(
                    "{} could not be transcribed and can be retried",
                    task.source_info.file_name
                ),
            ),
            _ => return,
        };
        let mut builder = self.app.notification().builder().title(title).body(body);
        if values.completion_sound {
            if let Some(sound) = default_notification_sound() {
                builder = builder.sound(sound);
            }
        }
        let _ = builder.show();
    }
}

impl<R: Runtime> TranscriptionEventSink for TauriTranscriptionEventSink<R> {
    fn task_changed(&self, sequence: u64, task: &TranscribeTask) {
        let _ = self.app.emit(
            "transcribe://progress",
            TranscriptionProgressEvent {
                sequence,
                task: task.clone(),
            },
        );
        self.notify_terminal(task);
    }

    fn task_removed(&self, sequence: u64, task_id: &str) {
        let _ = self.app.emit(
            "transcribe://removed",
            TranscriptionRemovedEvent {
                sequence,
                task_id: task_id.into(),
            },
        );
    }
}

#[cfg(target_os = "windows")]
fn default_notification_sound() -> Option<String> {
    Some("Default".into())
}

#[cfg(target_os = "macos")]
fn default_notification_sound() -> Option<String> {
    Some("Ping".into())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_notification_sound() -> Option<String> {
    Some("message-new-instant".into())
}

impl<R: Runtime> ModelEventSink for TauriTranscriptionEventSink<R> {
    fn model_changed(&self, sequence: u64, model: &AsrModel) {
        let _ = self.app.emit(
            "transcribe://model-progress",
            ModelDownloadProgressEvent {
                sequence,
                model: model.clone(),
            },
        );
    }
}
