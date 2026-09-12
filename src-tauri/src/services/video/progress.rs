use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex,
};

use crate::services::{
    audio::ExecutionReporterPort,
    download::{download_percent, DownloadControl, DownloadProgress, ProgressSink},
    tasks::ExecutionUpdate,
};

use crate::infrastructure::video::VideoTrack;

use super::aggregate_video_progress;

pub(super) struct VideoProgressState {
    task_id: String,
    attempt_id: String,
    reporter: Arc<dyn ExecutionReporterPort>,
    control: DownloadControl,
    percent: Arc<AtomicU8>,
    video: Arc<Mutex<DownloadProgress>>,
    audio: Arc<Mutex<Option<DownloadProgress>>>,
}

impl VideoProgressState {
    pub(super) fn new(
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
            percent: Arc::new(AtomicU8::new(percent)),
            video: Arc::new(Mutex::new(empty_progress())),
            audio: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn sink(self: &Arc<Self>, track: VideoTrack) -> Arc<dyn ProgressSink> {
        Arc::new(VideoProgressSink {
            task_id: self.task_id.clone(),
            attempt_id: self.attempt_id.clone(),
            track,
            reporter: self.reporter.clone(),
            control: self.control.clone(),
            percent: self.percent.clone(),
            video: self.video.clone(),
            audio: self.audio.clone(),
        })
    }
}

struct VideoProgressSink {
    task_id: String,
    attempt_id: String,
    track: VideoTrack,
    reporter: Arc<dyn ExecutionReporterPort>,
    control: DownloadControl,
    percent: Arc<AtomicU8>,
    video: Arc<Mutex<DownloadProgress>>,
    audio: Arc<Mutex<Option<DownloadProgress>>>,
}

impl ProgressSink for VideoProgressSink {
    fn report(&self, progress: DownloadProgress) {
        if self.track == VideoTrack::Video {
            if let Ok(mut value) = self.video.lock() {
                *value = progress;
            }
        } else if let Ok(mut value) = self.audio.lock() {
            *value = Some(progress);
        }
        let video = self
            .video
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| empty_progress());
        let audio = self.audio.lock().ok().and_then(|value| value.clone());
        let aggregate = aggregate_video_progress(&video, audio.as_ref());
        let current = self.percent.load(Ordering::SeqCst);
        let percent = download_percent(aggregate.downloaded_bytes, aggregate.total_bytes, current);
        self.percent.store(percent, Ordering::SeqCst);
        let accepted = self.reporter.report(
            &self.task_id,
            &self.attempt_id,
            ExecutionUpdate::Progress {
                percent,
                bytes_downloaded: aggregate.downloaded_bytes.to_string(),
                total_bytes: aggregate.total_bytes.map(|value| value.to_string()),
                speed_bytes_per_second: aggregate.speed_bytes_per_second.to_string(),
                eta_seconds: aggregate.eta_seconds,
            },
        );
        if !matches!(accepted, Ok(true)) {
            self.control.request_cancel();
        }
    }
}

fn empty_progress() -> DownloadProgress {
    DownloadProgress {
        downloaded_bytes: 0,
        total_bytes: None,
        speed_bytes_per_second: 0,
        eta_seconds: None,
    }
}
