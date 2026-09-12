use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use bilicatch_lib::{
    models::{SettingsDocument, SettingsPatch, ThemePreference},
    services::settings::{SettingsDefaults, SettingsManager, SettingsStorePort},
};
use serde_json::{json, Value};
use tempfile::TempDir;

#[derive(Default)]
struct MemorySettingsStore {
    raw: Mutex<Option<Value>>,
    saved: Mutex<Vec<SettingsDocument>>,
    fail_load: AtomicBool,
    fail_save: AtomicBool,
}

impl MemorySettingsStore {
    fn with_raw(raw: Value) -> Self {
        Self {
            raw: Mutex::new(Some(raw)),
            ..Self::default()
        }
    }

    fn saved(&self) -> Vec<SettingsDocument> {
        self.saved.lock().expect("saved lock").clone()
    }
}

impl SettingsStorePort for MemorySettingsStore {
    fn load_raw(&self) -> Result<Option<Value>, bilicatch_lib::models::AppError> {
        if self.fail_load.load(Ordering::SeqCst) {
            return Err(bilicatch_lib::models::AppError::internal(
                "Settings storage is unavailable",
            ));
        }
        Ok(self.raw.lock().expect("raw lock").clone())
    }

    fn save_document(
        &self,
        document: &SettingsDocument,
    ) -> Result<(), bilicatch_lib::models::AppError> {
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(bilicatch_lib::models::AppError::internal(
                "Settings storage is unavailable",
            ));
        }
        self.saved
            .lock()
            .expect("saved lock")
            .push(document.clone());
        *self.raw.lock().expect("raw lock") =
            Some(serde_json::to_value(document).expect("settings document should serialize"));
        Ok(())
    }
}

fn defaults() -> (TempDir, SettingsDefaults) {
    let root = tempfile::tempdir().expect("temp root");
    let downloads = root.path().join("Downloads");
    let temporary = root.path().join("Temporary");
    std::fs::create_dir_all(&temporary).expect("temporary directory");
    let defaults = SettingsDefaults::from_system_paths(downloads, temporary)
        .expect("default paths should resolve");
    (root, defaults)
}

fn details(error: &bilicatch_lib::models::AppError) -> &str {
    error.details.as_deref().expect("stable error details")
}

#[test]
fn missing_store_persists_complete_runtime_defaults() {
    let (_root, defaults) = defaults();
    let expected_download = defaults.download_directory.clone();
    let expected_temporary = defaults.temporary_directory.clone();
    let store = Arc::new(MemorySettingsStore::default());

    let manager = SettingsManager::load(store.clone(), defaults).expect("settings should load");
    let snapshot = manager.snapshot();

    assert_eq!(snapshot.schema_version, 1);
    assert_eq!(snapshot.revision, 0);
    assert_eq!(snapshot.values.download_directory, expected_download);
    assert_eq!(snapshot.values.temporary_directory, expected_temporary);
    assert_eq!(snapshot.values.max_concurrent_tasks, 3);
    assert_eq!(snapshot.values.connections_per_task, 8);
    assert_eq!(snapshot.values.locale, "zh-CN");
    assert_eq!(store.saved(), vec![snapshot]);
}

#[test]
fn migration_repairs_invalid_fields_and_drops_unknown_fields() {
    let (_root, defaults) = defaults();
    let store = Arc::new(MemorySettingsStore::with_raw(json!({
        "schemaVersion": 1,
        "revision": 7,
        "values": {
            "maxConcurrentTasks": 99,
            "connectionsPerTask": 12,
            "theme": "dark",
            "locale": "invalid-locale",
            "unknownField": true
        },
        "unknownDocumentField": "discard"
    })));

    let manager = SettingsManager::load(store.clone(), defaults).expect("migration should succeed");
    let snapshot = manager.snapshot();

    assert_eq!(snapshot.revision, 7);
    assert_eq!(snapshot.values.max_concurrent_tasks, 3);
    assert_eq!(snapshot.values.connections_per_task, 12);
    assert_eq!(snapshot.values.theme, ThemePreference::Dark);
    assert_eq!(snapshot.values.locale, "zh-CN");
    assert_eq!(store.saved(), vec![snapshot]);
}

#[test]
fn future_schema_and_store_load_failures_do_not_overwrite_data() {
    let (_root, defaults) = defaults();
    let store = Arc::new(MemorySettingsStore::with_raw(json!({
        "schemaVersion": 2,
        "revision": 0,
        "values": {}
    })));

    let error = SettingsManager::load(store.clone(), defaults.clone())
        .expect_err("future schemas must be rejected");
    assert_eq!(details(&error), "SETTINGS_STORE_UNAVAILABLE");
    assert!(store.saved().is_empty());

    let corrupt_store = Arc::new(MemorySettingsStore::with_raw(json!("not-a-document")));
    let error = SettingsManager::load(corrupt_store.clone(), defaults.clone())
        .expect_err("structurally corrupt documents must be rejected");
    assert_eq!(details(&error), "SETTINGS_STORE_UNAVAILABLE");
    assert!(corrupt_store.saved().is_empty());

    let failing_store = Arc::new(MemorySettingsStore::default());
    failing_store.fail_load.store(true, Ordering::SeqCst);
    let error = SettingsManager::load(failing_store.clone(), defaults)
        .expect_err("load errors must be surfaced");
    assert_eq!(details(&error), "SETTINGS_STORE_UNAVAILABLE");
    assert!(failing_store.saved().is_empty());
}

#[test]
fn updates_validate_ranges_paths_and_skip_noop_writes() {
    let (_root, defaults) = defaults();
    let store = Arc::new(MemorySettingsStore::default());
    let manager = SettingsManager::load(store.clone(), defaults).expect("settings should load");
    let initial_save_count = store.saved().len();

    let snapshot = manager
        .update(SettingsPatch::MaxConcurrentTasks(3))
        .expect("no-op should succeed");
    assert_eq!(snapshot.revision, 0);
    assert_eq!(store.saved().len(), initial_save_count);

    let snapshot = manager
        .update(SettingsPatch::ConnectionsPerTask(32))
        .expect("upper bound should be valid");
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.values.connections_per_task, 32);

    let error = manager
        .update(SettingsPatch::ConnectionsPerTask(33))
        .expect_err("connections above 32 must fail");
    assert_eq!(details(&error), "SETTINGS_INVALID_VALUE");
    let error = manager
        .update(SettingsPatch::MaxConcurrentTasks(11))
        .expect_err("concurrency above 10 must fail");
    assert_eq!(details(&error), "SETTINGS_INVALID_VALUE");
    let error = manager
        .update(SettingsPatch::DownloadDirectory("relative/path".into()))
        .expect_err("relative paths must fail");
    assert_eq!(details(&error), "SETTINGS_PATH_INVALID");

    let missing = _root.path().join("Missing");
    let error = manager
        .update(SettingsPatch::TemporaryDirectory(
            missing.to_string_lossy().into_owned(),
        ))
        .expect_err("missing directories must fail");
    assert_eq!(details(&error), "SETTINGS_PATH_INVALID");

    let file = _root.path().join("not-a-directory.txt");
    std::fs::write(&file, "test").expect("test file");
    let error = manager
        .update(SettingsPatch::TemporaryDirectory(
            file.to_string_lossy().into_owned(),
        ))
        .expect_err("files must fail directory validation");
    assert_eq!(details(&error), "SETTINGS_PATH_INVALID");

    let existing = _root.path().join("Selected");
    std::fs::create_dir_all(&existing).expect("selected directory");
    let snapshot = manager
        .update(SettingsPatch::DownloadDirectory(
            existing.to_string_lossy().into_owned(),
        ))
        .expect("existing absolute paths should succeed");
    assert_eq!(snapshot.revision, 2);
}

#[test]
fn failed_save_keeps_the_previous_snapshot_and_revision() {
    let (_root, defaults) = defaults();
    let store = Arc::new(MemorySettingsStore::default());
    let manager = SettingsManager::load(store.clone(), defaults).expect("settings should load");
    store.fail_save.store(true, Ordering::SeqCst);

    let error = manager
        .update(SettingsPatch::Theme(ThemePreference::Dark))
        .expect_err("save failures must be surfaced");

    assert_eq!(details(&error), "SETTINGS_STORE_UNAVAILABLE");
    assert_eq!(manager.snapshot().revision, 0);
    assert_eq!(manager.snapshot().values.theme, ThemePreference::System);
}

#[test]
fn sequential_field_updates_preserve_both_values_and_latest_scheduler_limits() {
    let (_root, defaults) = defaults();
    let store = Arc::new(MemorySettingsStore::default());
    let manager = SettingsManager::load(store, defaults).expect("settings should load");

    manager
        .update(SettingsPatch::MaxConcurrentTasks(6))
        .expect("concurrency should update");
    let snapshot = manager
        .update(SettingsPatch::ConnectionsPerTask(24))
        .expect("connections should update");

    assert_eq!(snapshot.revision, 2);
    assert_eq!(snapshot.values.max_concurrent_tasks, 6);
    assert_eq!(snapshot.values.connections_per_task, 24);
    assert_eq!(manager.scheduler_limits(), (6, 24));
}
