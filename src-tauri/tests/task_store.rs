use std::fs;

use bilicatch_lib::{
    infrastructure::tasks::{JsonTaskStore, PersistedTaskFile, StoredTaskRecord},
    models::{DownloadMode, DownloadTask, TaskControlRequest, TaskStatus},
};
use tempfile::tempdir;

fn task(status: TaskStatus) -> DownloadTask {
    DownloadTask {
        id: "batch:0".into(),
        revision: 1,
        created_at: "2026-09-10T00:00:00Z".into(),
        updated_at: "2026-09-10T00:00:00Z".into(),
        file_name: "P1.mp4".into(),
        output_dir: "Downloads".into(),
        output_path: None,
        bvid: "BV1".into(),
        cid: 1,
        page: 1,
        part_title: "P1".into(),
        mode: DownloadMode::VideoOnly,
        quality_id: Some("80".into()),
        codec: None,
        audio_format: None,
        audio_bitrate_id: None,
        status,
        control_request: TaskControlRequest::PauseRequested,
        progress_percent: 40,
        bytes_downloaded: "400".into(),
        total_bytes: Some("1000".into()),
        speed_bytes_per_second: "100".into(),
        eta_seconds: Some(6),
        automatic_retry_count: 1,
        next_retry_at: Some("2026-09-10T00:00:01Z".into()),
        error: None,
    }
}

#[test]
fn store_roundtrips_and_recovers_transient_state() {
    let dir = tempdir().unwrap();
    let store = JsonTaskStore::new(dir.path().join("tasks.json"));
    store
        .save(&PersistedTaskFile::new(
            4,
            vec![StoredTaskRecord::new(
                "request",
                task(TaskStatus::Downloading),
            )],
        ))
        .unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.file.sequence, 4);
    assert_eq!(loaded.file.tasks[0].task.status, TaskStatus::Queued);
    assert_eq!(
        loaded.file.tasks[0].task.control_request,
        TaskControlRequest::None
    );
    assert_eq!(loaded.file.tasks[0].task.speed_bytes_per_second, "0");
    assert!(loaded.warning.is_none());
}

#[test]
fn corrupt_primary_recovers_from_the_last_valid_backup() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    let store = JsonTaskStore::new(path.clone());
    store
        .save(&PersistedTaskFile::new(
            1,
            vec![StoredTaskRecord::new("first", task(TaskStatus::Paused))],
        ))
        .unwrap();
    store
        .save(&PersistedTaskFile::new(
            2,
            vec![StoredTaskRecord::new("second", task(TaskStatus::Completed))],
        ))
        .unwrap();
    fs::write(&path, "not-json").unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.file.sequence, 1);
    assert_eq!(
        loaded.warning.as_deref(),
        Some("TASK_STORE_RECOVERED_FROM_BACKUP")
    );
}

#[test]
fn unknown_schema_is_reported_without_overwriting_the_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    fs::write(&path, r#"{"schemaVersion":2,"sequence":0,"tasks":[]}"#).unwrap();
    let original = fs::read_to_string(&path).unwrap();

    assert!(JsonTaskStore::new(path.clone()).load().is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}

#[test]
fn two_corrupt_copies_fail_without_replacing_the_primary() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    fs::write(&path, "broken-primary").unwrap();
    fs::write(format!("{}.bak", path.display()), "broken-backup").unwrap();

    assert!(JsonTaskStore::new(path.clone()).load().is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "broken-primary");
}

#[test]
fn saving_after_backup_recovery_does_not_replace_the_good_backup_with_corrupt_data() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tasks.json");
    let store = JsonTaskStore::new(path.clone());
    store.save(&PersistedTaskFile::new(1, vec![])).unwrap();
    store.save(&PersistedTaskFile::new(2, vec![])).unwrap();
    fs::write(&path, "corrupt-primary").unwrap();
    assert_eq!(store.load().unwrap().file.sequence, 1);

    store.save(&PersistedTaskFile::new(3, vec![])).unwrap();
    fs::write(&path, "corrupt-new-primary").unwrap();
    assert_eq!(store.load().unwrap().file.sequence, 1);
}
