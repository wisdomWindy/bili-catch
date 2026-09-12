use async_trait::async_trait;

use crate::{
    models::{AppError, DownloadMode},
    services::{
        audio::ProcessControl,
        download::DownloadControl,
        tasks::{TaskExecutionSpec, TaskExecutorPort},
    },
};

use super::executor::{VideoActiveControls, VideoExecutor};

#[async_trait]
impl TaskExecutorPort for VideoExecutor {
    fn is_available(&self) -> bool {
        true
    }

    fn supports(&self, mode: DownloadMode) -> bool {
        matches!(mode, DownloadMode::VideoOnly | DownloadMode::VideoAudio)
    }

    async fn start(&self, spec: TaskExecutionSpec) -> Result<(), AppError> {
        let key = (spec.task_id.clone(), spec.attempt_id.clone());
        let controls = VideoActiveControls {
            download: DownloadControl::new(),
            process: ProcessControl::new(),
        };
        {
            let mut active = self
                .active
                .lock()
                .map_err(|_| AppError::internal("The video executor is unavailable"))?;
            if active.contains_key(&key) {
                return Err(AppError::internal("The video attempt is already running"));
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
            .ok_or_else(|| AppError::internal("The video attempt is no longer active"))?;
        controls.download.request_pause();
        Ok(())
    }

    async fn cancel(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        let controls = self
            .active_controls(task_id, attempt_id)
            .ok_or_else(|| AppError::internal("The video attempt is no longer active"))?;
        controls.download.request_cancel();
        controls.process.request_cancel();
        Ok(())
    }
}
