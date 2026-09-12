use std::path::PathBuf;

use async_trait::async_trait;

use crate::{models::AppError, services::audio::ProcessControl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoMuxRequest {
    pub video_path: PathBuf,
    pub audio_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMuxOutcome {
    Completed,
    Cancelled,
}

#[async_trait]
pub trait VideoMuxerPort: Send + Sync {
    async fn mux(
        &self,
        request: VideoMuxRequest,
        control: ProcessControl,
    ) -> Result<VideoMuxOutcome, AppError>;
}
