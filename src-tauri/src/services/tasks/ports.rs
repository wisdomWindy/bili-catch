use async_trait::async_trait;

use crate::{
    infrastructure::tasks::{JsonTaskStore, PersistedTaskFile, TaskStoreLoad},
    models::{
        AppError, DownloadMode, DownloadTask, TaskAction, TaskProgressEvent, TaskRemovedEvent,
    },
};

pub trait TaskStorePort: Send + Sync {
    fn load(&self) -> Result<TaskStoreLoad, AppError>;
    fn save(&self, file: &PersistedTaskFile) -> Result<(), AppError>;
}

impl TaskStorePort for JsonTaskStore {
    fn load(&self) -> Result<TaskStoreLoad, AppError> {
        JsonTaskStore::load(self)
    }
    fn save(&self, file: &PersistedTaskFile) -> Result<(), AppError> {
        JsonTaskStore::save(self, file)
    }
}

pub trait TaskEventSink: Send + Sync {
    fn progress(&self, event: TaskProgressEvent);
    fn removed(&self, event: TaskRemovedEvent);
}

pub trait TaskCleanerPort: Send + Sync {
    fn cleanup(&self, paths: &[String]) -> Result<(), AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerLimits {
    pub max_concurrent: u8,
    pub connections_per_task: u8,
}

pub trait TaskSettingsPort: Send + Sync {
    fn scheduler_limits(&self) -> SchedulerLimits;
    fn temporary_directory(&self) -> String;
}

impl TaskSettingsPort for crate::services::settings::SettingsManager {
    fn scheduler_limits(&self) -> SchedulerLimits {
        let (max_concurrent, connections_per_task) =
            crate::services::settings::SettingsManager::scheduler_limits(self);
        SchedulerLimits {
            max_concurrent,
            connections_per_task,
        }
    }

    fn temporary_directory(&self) -> String {
        self.snapshot().values.temporary_directory
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskExecutionSpec {
    pub task_id: String,
    pub attempt_id: String,
    pub connection_count: u8,
    pub temporary_directory: String,
    pub task: DownloadTask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionControlSpec {
    pub task_id: String,
    pub attempt_id: String,
    pub action: TaskAction,
}

#[async_trait]
pub trait TaskExecutorPort: Send + Sync {
    fn is_available(&self) -> bool;
    fn supports(&self, mode: DownloadMode) -> bool;
    async fn start(&self, spec: TaskExecutionSpec) -> Result<(), AppError>;
    async fn pause(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError>;
    async fn cancel(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError>;
}

pub struct DeferredTaskExecutor;

#[async_trait]
impl TaskExecutorPort for DeferredTaskExecutor {
    fn is_available(&self) -> bool {
        false
    }
    fn supports(&self, _mode: DownloadMode) -> bool {
        false
    }
    async fn start(&self, _spec: TaskExecutionSpec) -> Result<(), AppError> {
        Err(unavailable())
    }
    async fn pause(&self, _task_id: &str, _attempt_id: &str) -> Result<(), AppError> {
        Err(unavailable())
    }
    async fn cancel(&self, _task_id: &str, _attempt_id: &str) -> Result<(), AppError> {
        Err(unavailable())
    }
}

fn unavailable() -> AppError {
    let mut error = AppError::internal("Task executor is not installed");
    error.details = Some("TASK_EXECUTOR_DEFERRED".into());
    error
}
