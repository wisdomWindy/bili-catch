use bilicatch_lib::models::{
    AsrModelId, AudioFormat, CloseBehavior, GpuMode, SettingsDocument, SettingsPatch,
    SettingsValues, ThemePreference, TranscriptFormat, TranscriptionLanguage, UpdateSettingRequest,
    VideoQualityId,
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
        completion_sound: true,
        close_behavior: CloseBehavior::MinimizeToTray,
        auto_check_updates: true,
        default_asr_model: AsrModelId::Base,
        default_transcription_language: TranscriptionLanguage::Auto,
        default_transcript_format: TranscriptFormat::Srt,
        transcription_timestamps: true,
        transcription_punctuation: true,
        prefer_platform_subtitles: true,
        auto_transcribe_downloads: false,
        max_concurrent_transcriptions: 1,
        asr_model_directory: "D:/AppData/BiliCatch/models/whisper".into(),
        asr_gpu_mode: GpuMode::Auto,
        asr_model_source: String::new(),
    }
}

#[test]
fn settings_snapshot_serializes_to_the_frontend_contract() {
    let document = SettingsDocument {
        schema_version: 2,
        revision: 4,
        values: values(),
    };

    assert_eq!(
        serde_json::to_value(document).expect("settings document should serialize"),
        json!({
            "schemaVersion": 2,
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
                "completionSound": true,
                "closeBehavior": "minimizeToTray",
                "autoCheckUpdates": true,
                "defaultAsrModel": "base",
                "defaultTranscriptionLanguage": "auto",
                "defaultTranscriptFormat": "srt",
                "transcriptionTimestamps": true,
                "transcriptionPunctuation": true,
                "preferPlatformSubtitles": true,
                "autoTranscribeDownloads": false,
                "maxConcurrentTranscriptions": 1,
                "asrModelDirectory": "D:/AppData/BiliCatch/models/whisper",
                "asrGpuMode": "auto",
                "asrModelSource": ""
            }
        })
    );
}

#[test]
fn completion_sound_patch_uses_the_boolean_frontend_contract() {
    let request: UpdateSettingRequest = serde_json::from_value(json!({
        "patch": {
            "field": "completionSound",
            "value": false
        }
    }))
    .expect("completion sound patch should deserialize");

    assert_eq!(
        serde_json::to_value(request).expect("completion sound patch should serialize"),
        json!({
            "patch": {
                "field": "completionSound",
                "value": false
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
