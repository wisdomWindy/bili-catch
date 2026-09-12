use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

use async_trait::async_trait;

use crate::models::{AppError, AudioOutputProfile};

use super::AudioSourceTier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessControlState {
    Running = 0,
    PauseRequested = 1,
    CancelRequested = 2,
}

#[derive(Clone)]
pub struct ProcessControl {
    state: Arc<AtomicU8>,
}

impl ProcessControl {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(ProcessControlState::Running as u8)),
        }
    }

    pub fn request_pause(&self) {
        let _ = self.state.compare_exchange(
            ProcessControlState::Running as u8,
            ProcessControlState::PauseRequested as u8,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    pub fn request_cancel(&self) {
        self.state
            .store(ProcessControlState::CancelRequested as u8, Ordering::SeqCst);
    }

    pub fn state(&self) -> ProcessControlState {
        match self.state.load(Ordering::SeqCst) {
            1 => ProcessControlState::PauseRequested,
            2 => ProcessControlState::CancelRequested,
            _ => ProcessControlState::Running,
        }
    }
}

impl Default for ProcessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AudioProcessRequest {
    pub source_path: PathBuf,
    pub cover_path: PathBuf,
    pub output_path: PathBuf,
    pub output_profile: AudioOutputProfile,
    pub source_tier: AudioSourceTier,
    pub title: String,
    pub uploader: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    Completed,
    Cancelled,
}

#[async_trait]
pub trait MediaProcessorPort: Send + Sync {
    async fn process(
        &self,
        request: AudioProcessRequest,
        control: ProcessControl,
    ) -> Result<ProcessOutcome, AppError>;
}
