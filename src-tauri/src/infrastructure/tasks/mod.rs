use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::{AppError, DownloadTask, TaskControlRequest, TaskStatus};

mod tauri_adapter;
pub use tauri_adapter::{FileTaskCleaner, TauriTaskEventSink};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTaskRecord {
    pub source_request_id: String,
    pub attempt_id: Option<String>,
    pub temporary_paths: Vec<String>,
    #[serde(default)]
    pub can_cancel_processing: bool,
    pub task: DownloadTask,
}

impl StoredTaskRecord {
    pub fn new(source_request_id: impl Into<String>, task: DownloadTask) -> Self {
        Self {
            source_request_id: source_request_id.into(),
            attempt_id: None,
            temporary_paths: vec![],
            can_cancel_processing: false,
            task,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTaskFile {
    pub schema_version: u32,
    pub sequence: u64,
    pub tasks: Vec<StoredTaskRecord>,
}

impl PersistedTaskFile {
    pub fn new(sequence: u64, tasks: Vec<StoredTaskRecord>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            sequence,
            tasks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStoreLoad {
    pub file: PersistedTaskFile,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JsonTaskStore {
    path: PathBuf,
}

impl JsonTaskStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<TaskStoreLoad, AppError> {
        if !self.path.exists() {
            return Ok(TaskStoreLoad {
                file: PersistedTaskFile::new(0, vec![]),
                warning: None,
            });
        }

        match self.read_file(&self.path) {
            Ok(file) => Ok(TaskStoreLoad {
                file: recover(file),
                warning: None,
            }),
            Err(primary_error) if is_invalid_json(&primary_error) => {
                let backup = self.backup_path();
                let file = self.read_file(&backup).map_err(|_| primary_error)?;
                Ok(TaskStoreLoad {
                    file: recover(file),
                    warning: Some("TASK_STORE_RECOVERED_FROM_BACKUP".into()),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, file: &PersistedTaskFile) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(store_io_error)?;
        }
        let bytes = serde_json::to_vec_pretty(file)
            .map_err(|_| AppError::internal("Unable to serialize task store"))?;
        let next = self.next_path();
        fs::write(&next, bytes).map_err(store_io_error)?;

        if self.path.exists() {
            if self.read_file(&self.path).is_ok() {
                fs::copy(&self.path, self.backup_path()).map_err(store_io_error)?;
            }
            fs::remove_file(&self.path).map_err(store_io_error)?;
        }
        fs::rename(next, &self.path).map_err(store_io_error)
    }

    fn read_file(&self, path: &PathBuf) -> Result<PersistedTaskFile, AppError> {
        let source = fs::read_to_string(path).map_err(store_io_error)?;
        let value: serde_json::Value = serde_json::from_str(&source).map_err(|_| {
            tagged_error(
                "Task store contains invalid JSON",
                "TASK_STORE_INVALID_JSON",
            )
        })?;
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64);
        if version != Some(u64::from(SCHEMA_VERSION)) {
            return Err(tagged_error(
                "Unsupported task store schema",
                "TASK_STORE_UNKNOWN_SCHEMA",
            ));
        }
        serde_json::from_value(value)
            .map_err(|_| tagged_error("Task store data is invalid", "TASK_STORE_INVALID_JSON"))
    }

    fn next_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.next", self.path.display()))
    }

    fn backup_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.bak", self.path.display()))
    }
}

fn recover(mut file: PersistedTaskFile) -> PersistedTaskFile {
    for record in &mut file.tasks {
        if matches!(
            record.task.status,
            TaskStatus::Downloading | TaskStatus::Processing
        ) {
            record.task.status = TaskStatus::Queued;
            record.attempt_id = None;
        }
        record.task.control_request = TaskControlRequest::None;
        record.task.speed_bytes_per_second = "0".into();
        record.task.eta_seconds = None;
        record.task.next_retry_at = None;
        record.can_cancel_processing = false;
    }
    file
}

fn tagged_error(message: &str, details: &str) -> AppError {
    let mut error = AppError::internal(message);
    error.details = Some(details.into());
    error
}

fn is_invalid_json(error: &AppError) -> bool {
    error.details.as_deref() == Some("TASK_STORE_INVALID_JSON")
}

fn store_io_error(_: std::io::Error) -> AppError {
    AppError::internal("Unable to access task store")
}
