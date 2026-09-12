use std::sync::Arc;

use tauri::State;

use crate::{
    models::{
        AppError, BatchTaskResult, ClearTasksResult, ControlDownloadTaskRequest,
        CreateDownloadTasksRequest, CreateTasksResult, DownloadTask, OpenDownloadTaskRequest,
        TaskListSnapshot,
    },
    services::tasks::TaskManager,
};

#[tauri::command]
pub fn list_download_tasks(manager: State<'_, Arc<TaskManager>>) -> TaskListSnapshot {
    manager.list()
}

#[tauri::command]
pub fn create_download_tasks(
    request: CreateDownloadTasksRequest,
    manager: State<'_, Arc<TaskManager>>,
) -> Result<CreateTasksResult, AppError> {
    manager.create(request)
}

#[tauri::command]
pub fn control_download_task(
    request: ControlDownloadTaskRequest,
    manager: State<'_, Arc<TaskManager>>,
) -> Result<DownloadTask, AppError> {
    manager.control(&request.task_id, request.action)
}

#[tauri::command]
pub fn pause_all_download_tasks(manager: State<'_, Arc<TaskManager>>) -> BatchTaskResult {
    manager.pause_all()
}

#[tauri::command]
pub fn clear_completed_tasks(
    manager: State<'_, Arc<TaskManager>>,
) -> Result<ClearTasksResult, AppError> {
    manager.clear_completed()
}

#[tauri::command]
pub fn open_download_task(
    request: OpenDownloadTaskRequest,
    manager: State<'_, Arc<TaskManager>>,
) -> Result<(), AppError> {
    let path = manager.resolve_open_path(&request.task_id, request.target)?;
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|_| AppError::internal("Unable to open task output"))
}
