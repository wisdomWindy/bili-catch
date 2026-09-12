use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::{
    models::{DownloadMode, TaskAction},
    services::tasks::{ExecutionUpdate, TaskExecutorPort, TaskManager},
};

pub struct DownloadRuntimeRunner {
    manager: Arc<TaskManager>,
    executor: Arc<dyn TaskExecutorPort>,
    forwarded_controls: Mutex<HashSet<(String, String, TaskAction)>>,
}

impl DownloadRuntimeRunner {
    pub fn new(manager: Arc<TaskManager>, executor: Arc<dyn TaskExecutorPort>) -> Self {
        Self {
            manager,
            executor,
            forwarded_controls: Mutex::new(HashSet::new()),
        }
    }

    pub async fn tick(&self) {
        if !self.executor.is_available() {
            return;
        }

        self.forward_controls().await;
        let supported_modes = [
            DownloadMode::AudioOnly,
            DownloadMode::VideoAudio,
            DownloadMode::VideoOnly,
        ]
        .into_iter()
        .filter(|mode| self.executor.supports(*mode))
        .collect::<Vec<_>>();

        while let Some(spec) = self.manager.claim_next_for(&supported_modes) {
            if let Err(error) = self.executor.start(spec.clone()).await {
                let _ = self.manager.apply_execution_update(
                    &spec.task_id,
                    &spec.attempt_id,
                    ExecutionUpdate::Failed { error },
                );
            }
        }
    }

    async fn forward_controls(&self) {
        let pending_controls = self.manager.pending_controls();
        let pending_keys = pending_controls
            .iter()
            .map(|control| {
                (
                    control.task_id.clone(),
                    control.attempt_id.clone(),
                    control.action,
                )
            })
            .collect::<HashSet<_>>();
        if let Ok(mut forwarded) = self.forwarded_controls.lock() {
            forwarded.retain(|key| pending_keys.contains(key));
        }

        for control in pending_controls {
            let key = (
                control.task_id.clone(),
                control.attempt_id.clone(),
                control.action,
            );
            let should_forward = self
                .forwarded_controls
                .lock()
                .map(|mut controls| controls.insert(key))
                .unwrap_or(false);
            if !should_forward {
                continue;
            }

            let result = match control.action {
                TaskAction::Pause => {
                    self.executor
                        .pause(&control.task_id, &control.attempt_id)
                        .await
                }
                TaskAction::Cancel => {
                    self.executor
                        .cancel(&control.task_id, &control.attempt_id)
                        .await
                }
                TaskAction::Resume | TaskAction::Retry | TaskAction::Delete => continue,
            };
            if let Err(error) = result {
                let _ = self.manager.apply_execution_update(
                    &control.task_id,
                    &control.attempt_id,
                    ExecutionUpdate::Failed { error },
                );
            }
        }
    }
}
