use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use crate::services::{
    download::{download_percent, DownloadControl, DownloadProgress, ProgressSink},
    tasks::ExecutionUpdate,
};

use super::ExecutionReporterPort;

pub(crate) struct AttemptProgressSink {
    task_id: String,
    attempt_id: String,
    reporter: Arc<dyn ExecutionReporterPort>,
    control: DownloadControl,
    percent: AtomicU8,
}

impl AttemptProgressSink {
    pub(crate) fn new(
        task_id: String,
        attempt_id: String,
        reporter: Arc<dyn ExecutionReporterPort>,
        control: DownloadControl,
        percent: u8,
    ) -> Self {
        Self {
            task_id,
            attempt_id,
            reporter,
            control,
            percent: AtomicU8::new(percent),
        }
    }
}

impl ProgressSink for AttemptProgressSink {
    fn report(&self, progress: DownloadProgress) {
        let current = self.percent.load(Ordering::SeqCst);
        let percent = download_percent(progress.downloaded_bytes, progress.total_bytes, current);
        self.percent.store(percent, Ordering::SeqCst);
        let update = ExecutionUpdate::Progress {
            percent,
            bytes_downloaded: progress.downloaded_bytes.to_string(),
            total_bytes: progress.total_bytes.map(|value| value.to_string()),
            speed_bytes_per_second: progress.speed_bytes_per_second.to_string(),
            eta_seconds: progress.eta_seconds,
        };
        if !matches!(
            self.reporter
                .report(&self.task_id, &self.attempt_id, update),
            Ok(true)
        ) {
            self.control.request_cancel();
        }
    }
}
