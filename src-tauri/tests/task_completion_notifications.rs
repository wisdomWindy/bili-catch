use std::sync::{Arc, Mutex};

use bilicatch_lib::{
    models::{DownloadTask, TaskProgressEvent, TaskRemovedEvent},
    services::{
        system::TaskCompletionNotification,
        tasks::{
            CompletionNotificationSettings, CompletionNotificationSettingsPort,
            CompletionNotifyingTaskEventSink, TaskCompletionNotifier, TaskEventSink,
        },
    },
};
use serde_json::json;

#[derive(Default)]
struct RecordingEventSink {
    progress_events: Mutex<Vec<TaskProgressEvent>>,
}

impl TaskEventSink for RecordingEventSink {
    fn progress(&self, event: TaskProgressEvent) {
        self.progress_events
            .lock()
            .expect("progress lock")
            .push(event);
    }

    fn removed(&self, _event: TaskRemovedEvent) {}
}

struct StaticNotificationSettings(CompletionNotificationSettings);

impl CompletionNotificationSettingsPort for StaticNotificationSettings {
    fn completion_notification_settings(&self) -> CompletionNotificationSettings {
        self.0.clone()
    }
}

#[derive(Default)]
struct RecordingNotifier {
    notifications: Mutex<Vec<TaskCompletionNotification>>,
}

impl TaskCompletionNotifier for RecordingNotifier {
    fn show(&self, notification: TaskCompletionNotification) {
        self.notifications
            .lock()
            .expect("notification lock")
            .push(notification);
    }
}

fn completed_event() -> TaskProgressEvent {
    let task: DownloadTask = serde_json::from_value(json!({
        "id": "task-1",
        "revision": 4,
        "createdAt": "2026-09-16T00:00:00Z",
        "updatedAt": "2026-09-16T00:01:00Z",
        "fileName": "episode.mp4",
        "outputDir": "D:/Downloads",
        "outputPath": "D:/Downloads/episode.mp4",
        "bvid": "BV1example",
        "cid": 1,
        "page": 1,
        "partTitle": "Episode",
        "mode": "video-audio",
        "qualityId": "80",
        "codec": "avc",
        "audioFormat": null,
        "audioBitrateId": null,
        "status": "completed",
        "controlRequest": "none",
        "progressPercent": 100,
        "bytesDownloaded": "1024",
        "totalBytes": "1024",
        "speedBytesPerSecond": "0",
        "etaSeconds": 0,
        "automaticRetryCount": 0,
        "nextRetryAt": null,
        "error": null
    }))
    .expect("task fixture should deserialize");
    TaskProgressEvent { sequence: 9, task }
}

#[test]
fn completed_events_are_forwarded_and_emit_the_configured_notification() {
    let events = Arc::new(RecordingEventSink::default());
    let notifier = Arc::new(RecordingNotifier::default());
    let settings = Arc::new(StaticNotificationSettings(CompletionNotificationSettings {
        notify_on_complete: true,
        completion_sound: false,
        locale: "zh-CN".into(),
    }));
    let sink = CompletionNotifyingTaskEventSink::new(events.clone(), settings, notifier.clone());

    sink.progress(completed_event());

    assert_eq!(
        events.progress_events.lock().expect("progress lock").len(),
        1
    );
    assert_eq!(
        *notifier.notifications.lock().expect("notification lock"),
        vec![TaskCompletionNotification {
            title: "下载完成".into(),
            body: "episode.mp4 已下载完成".into(),
            play_sound: false,
        }]
    );
}
