use bilicatch_lib::models::{
    AudioFormat, CloseBehavior, SettingsDocument, SettingsPatch, SettingsValues, ThemePreference,
    UpdateSettingRequest, VideoQualityId,
};
use serde_json::json;

fn values() -> SettingsValues {
    SettingsValues {
        download_directory: "D:/Media/BiliCatch".into(),
        temporary_directory: "D:/Temp".into(),
        max_concurrent_tasks: 3,
        connections_per_task: 8,
        default_video_quality: VideoQualityId::P1080,
        default_audio_format: AudioFormat::Mp3,
        theme: ThemePreference::System,
        locale: "zh-CN".into(),
        notify_on_complete: true,
        close_behavior: CloseBehavior::MinimizeToTray,
        auto_check_updates: true,
    }
}

#[test]
fn settings_snapshot_serializes_to_the_frontend_contract() {
    let document = SettingsDocument {
        schema_version: 1,
        revision: 4,
        values: values(),
    };

    assert_eq!(
        serde_json::to_value(document).expect("settings document should serialize"),
        json!({
            "schemaVersion": 1,
            "revision": 4,
            "values": {
                "downloadDirectory": "D:/Media/BiliCatch",
                "temporaryDirectory": "D:/Temp",
                "maxConcurrentTasks": 3,
                "connectionsPerTask": 8,
                "defaultVideoQuality": "80",
                "defaultAudioFormat": "mp3",
                "theme": "system",
                "locale": "zh-CN",
                "notifyOnComplete": true,
                "closeBehavior": "minimizeToTray",
                "autoCheckUpdates": true
            }
        })
    );
}

#[test]
fn setting_patch_uses_a_field_and_type_matched_value() {
    assert_eq!(
        serde_json::to_value(UpdateSettingRequest {
            patch: SettingsPatch::ConnectionsPerTask(32),
        })
        .expect("settings patch should serialize"),
        json!({
            "patch": {
                "field": "connectionsPerTask",
                "value": 32
            }
        })
    );
}
