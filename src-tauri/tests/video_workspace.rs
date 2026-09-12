use std::fs;

use bilicatch_lib::infrastructure::video::{
    FsVideoWorkspace, VideoCheckpointV1, VideoTrack, VideoWorkspace, VideoWorkspaceRequest,
};

fn request(root: &std::path::Path, output: &std::path::Path) -> VideoWorkspaceRequest {
    VideoWorkspaceRequest {
        task_id: "task/with unsafe title".into(),
        attempt_id: "attempt:1".into(),
        temporary_root: root.to_path_buf(),
        output_directory: output.to_path_buf(),
        final_file_name: "Episode.mp4".into(),
    }
}

#[test]
fn prepares_contained_dual_track_paths_and_track_specific_checkpoints() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsVideoWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();

    assert!(paths.video_path.starts_with(&paths.task_root));
    assert!(paths.audio_path.starts_with(&paths.task_root));
    assert!(paths.video_checkpoint_path.starts_with(&paths.task_root));
    assert!(paths.audio_checkpoint_path.starts_with(&paths.task_root));
    assert_ne!(paths.video_path, paths.audio_path);
    assert_ne!(paths.video_checkpoint_path, paths.audio_checkpoint_path);
    assert!(paths
        .processed_path
        .starts_with(output.path().canonicalize().unwrap()));
}

#[test]
fn incompatible_video_checkpoint_resets_only_video_track() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsVideoWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(&paths.video_path, b"video").unwrap();
    fs::write(&paths.audio_path, b"audio").unwrap();
    fs::write(&paths.audio_checkpoint_path, b"keep").unwrap();
    fs::write(
        &paths.video_checkpoint_path,
        r#"{"schemaVersion":1,"taskId":"other","trackKind":"video","sourceIdentityHash":"old","completedBytes":5}"#,
    )
    .unwrap();

    let loaded = workspace
        .load_compatible_checkpoint(
            &paths,
            VideoTrack::Video,
            "task/with unsafe title",
            "new",
            Some(5),
            None,
        )
        .unwrap();

    assert!(loaded.is_none());
    assert!(!paths.video_path.exists());
    assert!(!paths.video_checkpoint_path.exists());
    assert!(paths.audio_path.exists());
    assert!(paths.audio_checkpoint_path.exists());
}

#[test]
fn video_only_finalize_is_non_empty_atomic_and_non_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsVideoWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(output.path().join("Episode.mp4"), b"existing").unwrap();
    fs::write(&paths.video_path, b"video bytes").unwrap();

    let final_path = workspace.finalize_video_only(&paths).unwrap();

    assert_eq!(
        fs::read(output.path().join("Episode.mp4")).unwrap(),
        b"existing"
    );
    assert_eq!(fs::read(final_path).unwrap(), b"video bytes");
    assert!(!paths.video_path.exists());
}

#[test]
fn checkpoint_serialization_contains_track_but_no_url_or_credentials() {
    let checkpoint = VideoCheckpointV1 {
        schema_version: 1,
        task_id: "task-1".into(),
        track_kind: VideoTrack::Audio,
        source_identity_hash: "identity".into(),
        completed_bytes: 5,
        total_bytes: Some(10),
        etag: Some("etag".into()),
    };
    let stored = serde_json::to_string(&checkpoint).unwrap();

    assert!(stored.contains("trackKind"));
    assert!(!stored.contains("http"));
    assert!(!stored.contains("cookie"));
}
