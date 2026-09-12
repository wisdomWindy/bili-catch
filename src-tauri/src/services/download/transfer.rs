use std::path::Path;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use async_trait::async_trait;

use crate::models::AppError;

use super::MediaSourceCandidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DownloadControlState {
    Running = 0,
    PauseRequested = 1,
    CancelRequested = 2,
}

#[derive(Clone)]
pub struct DownloadControl {
    state: Arc<AtomicU8>,
}

impl DownloadControl {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(DownloadControlState::Running as u8)),
        }
    }

    pub fn request_pause(&self) {
        let _ = self.state.compare_exchange(
            DownloadControlState::Running as u8,
            DownloadControlState::PauseRequested as u8,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    pub fn request_cancel(&self) {
        self.state.store(
            DownloadControlState::CancelRequested as u8,
            Ordering::SeqCst,
        );
    }

    pub fn state(&self) -> DownloadControlState {
        match self.state.load(Ordering::SeqCst) {
            1 => DownloadControlState::PauseRequested,
            2 => DownloadControlState::CancelRequested,
            _ => DownloadControlState::Running,
        }
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_second: u64,
    pub eta_seconds: Option<u64>,
}

pub trait ProgressSink: Send + Sync {
    fn report(&self, progress: DownloadProgress);
}

#[derive(Clone)]
pub struct MediaWorkspacePaths {
    pub source_path: std::path::PathBuf,
    pub checkpoint_path: std::path::PathBuf,
}

#[derive(Clone)]
pub struct MediaDownloadRequest {
    pub task_id: String,
    pub bvid: String,
    pub cid: u64,
    pub source: MediaSourceCandidate,
    pub workspace: MediaWorkspacePaths,
    pub connection_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed,
    Paused,
    Cancelled,
}

#[async_trait]
pub trait ByteDownloaderPort: Send + Sync {
    async fn download(
        &self,
        request: MediaDownloadRequest,
        control: DownloadControl,
        progress: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError>;
}

#[async_trait]
pub trait CoverDownloaderPort: Send + Sync {
    async fn download_cover(
        &self,
        url: &str,
        output_path: &Path,
        control: DownloadControl,
    ) -> Result<DownloadOutcome, AppError>;
}

pub fn download_percent(downloaded: u64, total: Option<u64>, current: u8) -> u8 {
    let Some(total) = total.filter(|total| *total > 0) else {
        return current.min(90);
    };
    downloaded
        .saturating_mul(90)
        .checked_div(total)
        .unwrap_or(0)
        .min(90) as u8
}
