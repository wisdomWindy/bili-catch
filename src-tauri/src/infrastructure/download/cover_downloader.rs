use std::{fs::OpenOptions, io::Write, path::Path};

use async_trait::async_trait;
use reqwest::{header::CONTENT_TYPE, StatusCode};

use crate::{
    models::{AppError, AppErrorCode},
    services::download::{
        CoverDownloaderPort, DownloadControl, DownloadControlState, DownloadOutcome,
    },
};

use super::HttpByteDownloader;

const MAX_COVER_BYTES: u64 = 16 * 1024 * 1024;

#[async_trait]
impl CoverDownloaderPort for HttpByteDownloader {
    async fn download_cover(
        &self,
        value: &str,
        output_path: &Path,
        control: DownloadControl,
    ) -> Result<DownloadOutcome, AppError> {
        match control.state() {
            DownloadControlState::PauseRequested => return Ok(DownloadOutcome::Paused),
            DownloadControlState::CancelRequested => return Ok(DownloadOutcome::Cancelled),
            DownloadControlState::Running => {}
        }
        let mut response = self.request(value, 0).await.map_err(|_| cover_error())?;
        let is_image = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.to_ascii_lowercase().starts_with("image/"));
        if response.status() != StatusCode::OK
            || !is_image
            || response
                .content_length()
                .is_some_and(|length| length == 0 || length > MAX_COVER_BYTES)
        {
            return Err(cover_error());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .map_err(|_| cover_error())?;
        let mut written = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(|_| cover_error())? {
            match control.state() {
                DownloadControlState::PauseRequested => {
                    drop(file);
                    let _ = std::fs::remove_file(output_path);
                    return Ok(DownloadOutcome::Paused);
                }
                DownloadControlState::CancelRequested => {
                    drop(file);
                    let _ = std::fs::remove_file(output_path);
                    return Ok(DownloadOutcome::Cancelled);
                }
                DownloadControlState::Running => {}
            }
            written = written.saturating_add(chunk.len() as u64);
            if written > MAX_COVER_BYTES {
                drop(file);
                let _ = std::fs::remove_file(output_path);
                return Err(cover_error());
            }
            file.write_all(&chunk).map_err(|_| cover_error())?;
        }
        file.flush().map_err(|_| cover_error())?;
        if written == 0 {
            drop(file);
            let _ = std::fs::remove_file(output_path);
            return Err(cover_error());
        }
        Ok(DownloadOutcome::Completed)
    }
}

fn cover_error() -> AppError {
    let mut error = AppError::new(AppErrorCode::E008, "The audio cover could not be prepared");
    error.details = Some("FFMPEG_COVER_INVALID".into());
    error
}
