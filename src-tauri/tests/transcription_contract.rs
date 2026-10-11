use bilicatch_lib::models::{
    AsrModelId, CreateTranscribeTaskRequest, GpuMode, TranscriptFormat, TranscriptionLanguage,
    TranscriptionOptions, TranscriptionSource,
};
use serde_json::json;

#[test]
fn create_transcription_request_serializes_to_the_frontend_contract() {
    let request = CreateTranscribeTaskRequest {
        source: TranscriptionSource::LocalFile {
            path: "D:/Media/interview.mp4".into(),
        },
        options: TranscriptionOptions {
            model_id: AsrModelId::Base,
            language: TranscriptionLanguage::Auto,
            output_format: TranscriptFormat::Srt,
            timestamps: true,
            punctuation: true,
            output_directory: "D:/Media/Transcribe".into(),
            gpu_mode: GpuMode::Auto,
        },
        force: false,
    };

    assert_eq!(
        serde_json::to_value(request).expect("request should serialize"),
        json!({
            "source": {
                "kind": "localFile",
                "path": "D:/Media/interview.mp4"
            },
            "options": {
                "modelId": "base",
                "language": "auto",
                "outputFormat": "srt",
                "timestamps": true,
                "punctuation": true,
                "outputDirectory": "D:/Media/Transcribe",
                "gpuMode": "auto"
            },
            "force": false
        })
    );
}

#[test]
fn download_source_preserves_platform_subtitle_identity() {
    let source = TranscriptionSource::DownloadTask {
        task_id: "download-7".into(),
        output_path: "D:/Media/video.mp4".into(),
        bvid: Some("BV1example".into()),
        cid: Some(42),
    };

    assert_eq!(
        serde_json::to_value(source).expect("source should serialize"),
        json!({
            "kind": "downloadTask",
            "taskId": "download-7",
            "outputPath": "D:/Media/video.mp4",
            "bvid": "BV1example",
            "cid": 42
        })
    );
}
