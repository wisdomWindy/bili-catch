use std::fs;

use bilicatch_lib::infrastructure::audio::{
    AudioWorkspace, CheckpointV1, FsAudioWorkspace, WorkspaceRequest,
};

fn request(root: &std::path::Path, output: &std::path::Path) -> WorkspaceRequest {
    WorkspaceRequest {
        task_id: "task/with unsafe title".into(),
        attempt_id: "attempt:1".into(),
        temporary_root: root.to_path_buf(),
        output_directory: output.to_path_buf(),
        final_file_name: "Track.mp3".into(),
    }
}

#[test]
fn prepares_hashed_contained_paths_and_a_same_volume_processed_file() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsAudioWorkspace;

    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();

    assert!(paths
        .task_root
        .starts_with(temp.path().canonicalize().unwrap()));
    assert!(!paths.task_root.to_string_lossy().contains("unsafe title"));
    assert!(paths.source_path.starts_with(&paths.task_root));
    assert!(paths.cover_path.starts_with(&paths.task_root));
    assert!(paths.checkpoint_path.starts_with(&paths.task_root));
    assert!(paths
        .processed_path
        .starts_with(output.path().canonicalize().unwrap()));
    assert_ne!(paths.processed_path, paths.final_path);
}

#[test]
fn checkpoint_round_trip_contains_no_remote_or_credential_fields() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsAudioWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(&paths.source_path, b"12345").unwrap();
    let checkpoint = CheckpointV1 {
        schema_version: 1,
        task_id: "task/with unsafe title".into(),
        source_identity_hash: "identity-without-url".into(),
        completed_bytes: 5,
        total_bytes: Some(10),
        etag: Some("fixture-etag".into()),
    };

    workspace.save_checkpoint(&paths, &checkpoint).unwrap();
    let stored = fs::read_to_string(&paths.checkpoint_path).unwrap();
    let loaded = workspace
        .load_compatible_checkpoint(
            &paths,
            "task/with unsafe title",
            "identity-without-url",
            Some(10),
            Some("fixture-etag"),
        )
        .unwrap()
        .unwrap();

    assert_eq!(loaded, checkpoint);
    assert!(!stored.contains("http"));
    assert!(!stored.contains("cookie"));
    assert!(!stored.contains("header"));
}

#[test]
fn incompatible_checkpoint_discards_the_partial_and_restarts_from_zero() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsAudioWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(&paths.source_path, b"12345").unwrap();
    fs::write(
        &paths.checkpoint_path,
        r#"{"schemaVersion":1,"taskId":"other","sourceIdentityHash":"old","completedBytes":5}"#,
    )
    .unwrap();

    let loaded = workspace
        .load_compatible_checkpoint(&paths, "task/with unsafe title", "new", Some(10), None)
        .unwrap();

    assert!(loaded.is_none());
    assert!(!paths.source_path.exists());
    assert!(!paths.checkpoint_path.exists());
}

#[test]
fn finalize_never_overwrites_and_cleanup_is_idempotent_and_contained() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsAudioWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(output.path().join("Track.mp3"), b"existing").unwrap();
    fs::write(&paths.processed_path, b"new audio").unwrap();
    let unrelated = temp.path().join("keep.txt");
    fs::write(&unrelated, b"keep").unwrap();

    let final_path = workspace.finalize(&paths).unwrap();
    workspace.cleanup(&paths).unwrap();
    workspace.cleanup(&paths).unwrap();

    assert_eq!(
        fs::read(output.path().join("Track.mp3")).unwrap(),
        b"existing"
    );
    assert_eq!(fs::read(final_path).unwrap(), b"new audio");
    assert!(unrelated.exists());
    assert!(!paths.task_root.exists());
    assert!(!paths.processed_path.exists());
}

#[test]
fn finalize_rejects_an_empty_processed_file() {
    let temp = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let workspace = FsAudioWorkspace;
    let paths = workspace
        .prepare(&request(temp.path(), output.path()))
        .unwrap();
    fs::write(&paths.processed_path, []).unwrap();

    assert!(workspace.finalize(&paths).is_err());
    assert!(!paths.final_path.exists());
}
