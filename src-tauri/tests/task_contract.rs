use bilicatch_lib::models::{
    AudioFormat, DownloadMode, DownloadTask, TaskAction, TaskControlRequest, TaskStatus, VideoCodec,
};
use serde_json::json;

#[test]
fn task_snapshot_serializes_to_the_frontend_contract() {
    let task = DownloadTask {
        id: "batch-1:0".into(),
        revision: 3,
        created_at: "2026-09-10T00:00:00Z".into(),
        updated_at: "2026-09-10T00:01:00Z".into(),
        file_name: "P1.mp4".into(),
        output_dir: "Downloads".into(),
        output_path: None,
        bvid: "BV1xx411c7BF".into(),
        cid: 1001,
        page: 1,
        part_title: "P1".into(),
        mode: DownloadMode::VideoAudio,
        quality_id: Some("80".into()),
        codec: Some(VideoCodec::Avc),
        audio_format: Some(AudioFormat::M4a),
        audio_bitrate_id: None,
        status: TaskStatus::Downloading,
        control_request: TaskControlRequest::None,
        progress_percent: 50,
        bytes_downloaded: "9007199254740993".into(),
        total_bytes: Some("18014398509481986".into()),
        speed_bytes_per_second: "1048576".into(),
        eta_seconds: Some(30),
        automatic_retry_count: 1,
        next_retry_at: None,
        error: None,
    };

    assert_eq!(
        serde_json::to_value(task).expect("task should serialize"),
        json!({
            "id": "batch-1:0",
            "revision": 3,
            "createdAt": "2026-09-10T00:00:00Z",
            "updatedAt": "2026-09-10T00:01:00Z",
            "fileName": "P1.mp4",
            "outputDir": "Downloads",
            "outputPath": null,
            "bvid": "BV1xx411c7BF",
            "cid": 1001,
            "page": 1,
            "partTitle": "P1",
            "mode": "video-audio",
            "qualityId": "80",
            "codec": "avc",
            "audioFormat": "m4a",
            "audioBitrateId": null,
            "status": "downloading",
            "controlRequest": "none",
            "progressPercent": 50,
            "bytesDownloaded": "9007199254740993",
            "totalBytes": "18014398509481986",
            "speedBytesPerSecond": "1048576",
            "etaSeconds": 30,
            "automaticRetryCount": 1,
            "nextRetryAt": null,
            "error": null
        })
    );
}

#[test]
fn task_actions_use_the_stable_wire_values() {
    assert_eq!(
        serde_json::to_value([
            TaskAction::Pause,
            TaskAction::Resume,
            TaskAction::Cancel,
            TaskAction::Retry,
            TaskAction::Delete,
        ])
        .expect("actions should serialize"),
        json!(["pause", "resume", "cancel", "retry", "delete"])
    );
}
