use std::time::Instant;

use reqwest::Response;

use crate::{
    models::{AppError, AppErrorCode},
    services::download::DownloadProgress,
};

pub(super) struct ContentRange {
    pub(super) start: Option<u64>,
    pub(super) total: u64,
}

pub(super) fn parse_content_range(value: &str) -> Option<ContentRange> {
    let value = value.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let total = total.parse().ok()?;
    if range == "*" {
        return Some(ContentRange { start: None, total });
    }
    let (start, end) = range.split_once('-')?;
    let start = start.parse::<u64>().ok()?;
    let end = end.parse::<u64>().ok()?;
    (start <= end && end < total).then_some(ContentRange {
        start: Some(start),
        total,
    })
}

pub(super) fn header_text(
    response: &Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

pub(super) fn progress_snapshot(
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    started: Instant,
) -> DownloadProgress {
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let speed_bytes_per_second = (downloaded_bytes as f64 / elapsed) as u64;
    let eta_seconds = total_bytes.and_then(|total| {
        (speed_bytes_per_second > 0 && total >= downloaded_bytes)
            .then(|| (total - downloaded_bytes) / speed_bytes_per_second)
    });
    DownloadProgress {
        downloaded_bytes,
        total_bytes,
        speed_bytes_per_second,
        eta_seconds,
    }
}

pub(super) fn network_error(_: reqwest::Error) -> AppError {
    network_failure("The audio download was interrupted")
}

pub(super) fn network_failure(message: &str) -> AppError {
    AppError::new(AppErrorCode::E009, message)
}

pub(super) fn download_io_error(error: std::io::Error) -> AppError {
    if matches!(error.raw_os_error(), Some(28 | 112)) {
        AppError::new(AppErrorCode::E007, "There is not enough disk space")
    } else {
        AppError::internal("Unable to write the audio download")
    }
}
