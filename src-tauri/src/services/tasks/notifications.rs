use std::sync::Arc;

use crate::{
    models::{TaskProgressEvent, TaskRemovedEvent},
    services::system::{task_completion_notification, TaskCompletionNotification},
};

use super::ports::TaskEventSink;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionNotificationSettings {
    pub notify_on_complete: bool,
    pub completion_sound: bool,
    pub locale: String,
}

pub trait CompletionNotificationSettingsPort: Send + Sync {
    fn completion_notification_settings(&self) -> CompletionNotificationSettings;
}

impl CompletionNotificationSettingsPort for crate::services::settings::SettingsManager {
    fn completion_notification_settings(&self) -> CompletionNotificationSettings {
        let values = self.snapshot().values;
        CompletionNotificationSettings {
            notify_on_complete: values.notify_on_complete,
            completion_sound: values.completion_sound,
            locale: values.locale,
        }
    }
}

pub trait TaskCompletionNotifier: Send + Sync {
    fn show(&self, notification: TaskCompletionNotification);
}

pub struct CompletionNotifyingTaskEventSink {
    events: Arc<dyn TaskEventSink>,
    settings: Arc<dyn CompletionNotificationSettingsPort>,
    notifier: Arc<dyn TaskCompletionNotifier>,
}

impl CompletionNotifyingTaskEventSink {
    pub fn new(
        events: Arc<dyn TaskEventSink>,
        settings: Arc<dyn CompletionNotificationSettingsPort>,
        notifier: Arc<dyn TaskCompletionNotifier>,
    ) -> Self {
        Self {
            events,
            settings,
            notifier,
        }
    }
}

impl TaskEventSink for CompletionNotifyingTaskEventSink {
    fn progress(&self, event: TaskProgressEvent) {
        self.events.progress(event.clone());
        let settings = self.settings.completion_notification_settings();
        if let Some(notification) = task_completion_notification(
            event.task.status,
            &event.task.file_name,
            &settings.locale,
            settings.notify_on_complete,
            settings.completion_sound,
        ) {
            self.notifier.show(notification);
        }
    }

    fn removed(&self, event: TaskRemovedEvent) {
        self.events.removed(event);
    }
}
