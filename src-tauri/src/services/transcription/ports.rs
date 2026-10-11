use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;

use crate::{
    models::{AppError, GpuMode, TranscriptSegment, TranscriptionLanguage},
    services::audio::ProcessControl,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriberRequest {
    pub model_path: PathBuf,
    pub audio_path: PathBuf,
    pub output_base: PathBuf,
    pub language: TranscriptionLanguage,
    pub gpu_mode: GpuMode,
    pub offset_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriberOutput {
    pub detected_language: String,
    pub segments: Vec<TranscriptSegment>,
}

pub type TranscriberProgress = Arc<dyn Fn(f64) + Send + Sync>;

#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(
        &self,
        request: TranscriberRequest,
        control: ProcessControl,
        progress: TranscriberProgress,
    ) -> Result<TranscriberOutput, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleLookup {
    Available(TranscriberOutput),
    Unavailable,
    Failed(AppError),
}

#[async_trait]
pub trait SubtitleProvider: Send + Sync {
    async fn lookup(&self, bvid: &str, cid: u64) -> SubtitleLookup;
}

pub trait DownloadActivityProbe: Send + Sync {
    fn active_download_count(&self) -> usize;
}
