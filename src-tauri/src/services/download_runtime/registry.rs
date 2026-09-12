use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    models::{AppError, DownloadMode},
    services::tasks::{TaskExecutionSpec, TaskExecutorPort},
};

#[derive(Clone)]
pub struct StrategyRegistry {
    audio: Arc<dyn TaskExecutorPort>,
    video: Arc<dyn TaskExecutorPort>,
    active: Arc<Mutex<HashMap<(String, String), DownloadMode>>>,
}

pub type TaskExecutorRegistry = StrategyRegistry;

impl StrategyRegistry {
    pub fn new(audio: Arc<dyn TaskExecutorPort>, video: Arc<dyn TaskExecutorPort>) -> Self {
        Self {
            audio,
            video,
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn strategy(&self, mode: DownloadMode) -> &Arc<dyn TaskExecutorPort> {
        match mode {
            DownloadMode::AudioOnly => &self.audio,
            DownloadMode::VideoOnly | DownloadMode::VideoAudio => &self.video,
        }
    }

    fn active_mode(&self, task_id: &str, attempt_id: &str) -> Option<DownloadMode> {
        self.active
            .lock()
            .ok()?
            .get(&(task_id.to_owned(), attempt_id.to_owned()))
            .copied()
    }
}

#[async_trait]
impl TaskExecutorPort for StrategyRegistry {
    fn is_available(&self) -> bool {
        self.audio.is_available() || self.video.is_available()
    }

    fn supports(&self, mode: DownloadMode) -> bool {
        self.strategy(mode).is_available() && self.strategy(mode).supports(mode)
    }

    async fn start(&self, spec: TaskExecutionSpec) -> Result<(), AppError> {
        let strategy = self.strategy(spec.task.mode).clone();
        if !strategy.supports(spec.task.mode) {
            return Err(AppError::internal(
                "No executor is registered for this task",
            ));
        }
        let key = (spec.task_id.clone(), spec.attempt_id.clone());
        {
            let mut active = self
                .active
                .lock()
                .map_err(|_| AppError::internal("The strategy registry is unavailable"))?;
            if active.contains_key(&key) {
                return Err(AppError::internal("The task attempt is already routed"));
            }
            active.insert(key.clone(), spec.task.mode);
        }
        let result = strategy.start(spec).await;
        if result.is_err() {
            if let Ok(mut active) = self.active.lock() {
                active.remove(&key);
            }
        }
        result
    }

    async fn pause(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        let mode = self
            .active_mode(task_id, attempt_id)
            .ok_or_else(|| AppError::internal("The task attempt is no longer routed"))?;
        self.strategy(mode).pause(task_id, attempt_id).await
    }

    async fn cancel(&self, task_id: &str, attempt_id: &str) -> Result<(), AppError> {
        let mode = self
            .active_mode(task_id, attempt_id)
            .ok_or_else(|| AppError::internal("The task attempt is no longer routed"))?;
        self.strategy(mode).cancel(task_id, attempt_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::StrategyRegistry;
    use crate::{
        models::{AppError, DownloadMode},
        services::tasks::TaskExecutorPort,
    };

    struct ModeExecutor(DownloadMode);

    #[async_trait::async_trait]
    impl TaskExecutorPort for ModeExecutor {
        fn is_available(&self) -> bool {
            true
        }

        fn supports(&self, mode: DownloadMode) -> bool {
            mode == self.0
        }

        async fn start(
            &self,
            _spec: crate::services::tasks::TaskExecutionSpec,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn pause(&self, _task_id: &str, _attempt_id: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn cancel(&self, _task_id: &str, _attempt_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn registry_exposes_only_modes_supported_by_each_strategy() {
        let registry = StrategyRegistry::new(
            std::sync::Arc::new(ModeExecutor(DownloadMode::AudioOnly)),
            std::sync::Arc::new(ModeExecutor(DownloadMode::VideoOnly)),
        );
        assert!(registry.supports(DownloadMode::AudioOnly));
        assert!(registry.supports(DownloadMode::VideoOnly));
        assert!(!registry.supports(DownloadMode::VideoAudio));
    }
}
