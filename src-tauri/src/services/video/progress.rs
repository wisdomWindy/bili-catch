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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::AppError,
        services::{download::DownloadControlState, tasks::ExecutionUpdate},
    };

    #[derive(Debug, PartialEq, Eq)]
    struct ReportedProgress {
        percent: u8,
        bytes_downloaded: String,
        total_bytes: Option<String>,
    }

    #[derive(Default)]
    struct MonotonicReporter {
        accepted: Mutex<Vec<ReportedProgress>>,
    }

    impl ExecutionReporterPort for MonotonicReporter {
        fn report(
            &self,
            _task_id: &str,
            _attempt_id: &str,
            update: ExecutionUpdate,
        ) -> Result<bool, AppError> {
            let ExecutionUpdate::Progress {
                percent,
                bytes_downloaded,
                total_bytes,
                ..
            } = update
            else {
                return Ok(true);
            };
            let mut accepted = self.accepted.lock().unwrap();
            if accepted
                .last()
                .is_some_and(|current| percent < current.percent)
            {
                return Ok(false);
            }
            accepted.push(ReportedProgress {
                percent,
                bytes_downloaded,
                total_bytes,
            });
            Ok(true)
        }
    }

    #[test]
    fn adding_audio_total_does_not_cancel_a_valid_video_attempt() {
        let reporter = Arc::new(MonotonicReporter::default());
        let control = DownloadControl::new();
        let state = Arc::new(VideoProgressState::new(
            "task-1".into(),
            "attempt-1".into(),
            reporter.clone(),
            control.clone(),
            0,
        ));

        state.sink(VideoTrack::Video).report(DownloadProgress {
            downloaded_bytes: 1_014_439,
            total_bytes: Some(1_514_175),
            speed_bytes_per_second: 500_000,
            eta_seconds: Some(1),
        });
        state.sink(VideoTrack::Audio).report(DownloadProgress {
            downloaded_bytes: 16_384,
            total_bytes: Some(1_610_204),
            speed_bytes_per_second: 100_000,
            eta_seconds: Some(15),
        });

        assert_eq!(control.state(), DownloadControlState::Running);
        let accepted = reporter.accepted.lock().unwrap();
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].percent, 60);
        assert_eq!(
            accepted[1],
            ReportedProgress {
                percent: 60,
                bytes_downloaded: "1030823".to_owned(),
                total_bytes: Some("3124379".to_owned()),
            }
        );
    }
}
