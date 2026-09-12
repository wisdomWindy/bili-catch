use async_trait::async_trait;

use crate::{
    models::{AppError, DownloadMode},
    services::download::DownloadControl,
    services::tasks::{TaskExecutionSpec, TaskExecutorPort},
};

use super::{executor::ActiveControls, AudioExecutor, ProcessControl};

#[async_trait]
impl TaskExecutorPort for AudioExecutor {
    fn is_available(&self) -> bool {
        true
    }

    fn supports(&self, mode: DownloadMode) -> bool {
        mode == DownloadMode::AudioOnly
    }

    async fn start(&self, spec: TaskExecutionSpec) -> Result<(), AppError> {
        let key = (spec.task_id.clone(), spec.attempt_id.clone());
        let controls = ActiveControls {
            download: DownloadControl::new(),
            process: ProcessControl::new(),
        };
        {
            let mut active = self
                .active
                .lock()
                .map_err(|_| AppError::internal("The audio executor is unavailable"))?;
            if active.contains_key(&key) {
                return Err(AppError::internal("The audio attempt is already running"));
            }
            active.insert(key.clone(), controls.clone());
        }
        let executor = self.clone();
        tauri::async_runtime::spawn(async move {
            let _ = executor
                .execute_with_controls(spec, controls.download, controls.process)
                .await;
            if let Ok(mut active) = executor.active.lock() {
                active.remove(&key);
            }
        });
        Ok(())
    }

    async fn pause(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        let controls = self
            .active_controls(task_id, attempt_id)
            .ok_or_else(|| AppError::internal("The audio attempt is no longer active"))?;
        controls.download.request_pause();
        Ok(())
    }

    async fn cancel(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        let controls = self
            .active_controls(task_id, attempt_id)
            .ok_or_else(|| AppError::internal("The audio attempt is no longer active"))?;
        controls.download.request_cancel();
        controls.process.request_cancel();
        Ok(())
    }
}
