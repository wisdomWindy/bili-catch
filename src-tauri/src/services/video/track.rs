use std::{path::PathBuf, sync::Arc};

use crate::{
    infrastructure::video::{VideoTrack, VideoWorkspacePaths},
    models::{AppError, AppErrorCode, DownloadMode},
    services::{
        download::{
            media_source_identity, DownloadControl, DownloadOutcome, MediaDownloadRequest,
            MediaSourceCandidate, MediaWorkspacePaths, ProgressSink,
        },
        tasks::{ExecutionUpdate, TaskExecutionSpec},
    },
};

use super::{executor::VideoExecutor, VideoSourceRequest};

impl VideoExecutor {
    pub(super) async fn download_track(
        &self,
        spec: &TaskExecutionSpec,
        track: VideoTrack,
        source: MediaSourceCandidate,
        paths: &VideoWorkspacePaths,
        connection_count: u8,
        control: DownloadControl,
        progress: Arc<dyn ProgressSink>,
    ) -> Result<DownloadOutcome, AppError> {
        let first = self
            .download_track_once(
                spec,
                track,
                source,
                paths,
                connection_count,
                control.clone(),
                progress.clone(),
            )
            .await;
        match first {
            Ok(outcome) => Ok(outcome),
            Err(error) if source_may_be_stale(error.code) => {
                let quality_id = spec
                    .task
                    .quality_id
                    .clone()
                    .ok_or_else(|| AppError::internal("The video quality is missing"))?;
                let codec = spec
                    .task
                    .codec
                    .clone()
                    .ok_or_else(|| AppError::internal("The video codec is missing"))?;
                let refreshed = self
                    .source
                    .resolve(&VideoSourceRequest {
                        bvid: spec.task.bvid.clone(),
                        cid: spec.task.cid,
                        mode: spec.task.mode,
                        quality_id,
                        codec,
                    })
                    .await?;
                let refreshed_source = match track {
                    VideoTrack::Video => refreshed.video,
                    VideoTrack::Audio => refreshed.audio.ok_or_else(|| {
                        AppError::new(AppErrorCode::E004, "The audio source is unavailable")
                    })?,
                };
                self.download_track_once(
                    spec,
                    track,
                    refreshed_source,
                    paths,
                    connection_count,
                    control,
                    progress,
                )
                .await
            }
            Err(error) => Err(error),
        }
    }

    async fn download_track_once(
        &self,
        spec: &TaskExecutionSpec,
        track: VideoTrack,
        source: MediaSourceCandidate,
        paths: &VideoWorkspacePaths,
        connection_count: u8,
        control: DownloadControl,
        progress: Arc<dyn ProgressSink>,
    ) -> Result<DownloadOutcome, AppError> {
        let workspace = MediaWorkspacePaths {
            source_path: track_path(paths, track).clone(),
            checkpoint_path: track_checkpoint_path(paths, track).clone(),
        };
        let identity = media_source_identity(&spec.task.bvid, spec.task.cid, &source);
        let _ = self.workspace.load_compatible_checkpoint(
            paths,
            track,
            &spec.task_id,
            &identity,
            source.content_length,
            source.etag.as_deref(),
        )?;
        let request = MediaDownloadRequest {
            task_id: spec.task_id.clone(),
            bvid: spec.task.bvid.clone(),
            cid: spec.task.cid,
            source,
            workspace,
            connection_count,
        };
        self.downloader
            .download(request, control, progress.as_ref())
            .await
    }

    pub(super) fn report_paused(&self, spec: &TaskExecutionSpec) -> Result<(), AppError> {
        self.reporter
            .report(&spec.task_id, &spec.attempt_id, ExecutionUpdate::Paused)?;
        Ok(())
    }

    pub(super) fn report_cancelled(
        &self,
        spec: &TaskExecutionSpec,
        paths: &VideoWorkspacePaths,
    ) -> Result<(), AppError> {
        self.workspace.cleanup(paths)?;
        self.reporter
            .report(&spec.task_id, &spec.attempt_id, ExecutionUpdate::Cancelled)?;
        Ok(())
    }
}

fn source_may_be_stale(code: AppErrorCode) -> bool {
    matches!(
        code,
        AppErrorCode::E004 | AppErrorCode::E005 | AppErrorCode::E006
    )
}

pub(super) fn connection_budget(total: u8, mode: DownloadMode, track: VideoTrack) -> u8 {
    if mode == DownloadMode::VideoOnly {
        return total.max(1);
    }
    if total <= 1 {
        return 1;
    }
    let video = total / 2;
    if track == VideoTrack::Video {
        video.max(1)
    } else {
        (total - video).max(1)
    }
}

fn track_path(paths: &VideoWorkspacePaths, track: VideoTrack) -> &PathBuf {
    match track {
        VideoTrack::Video => &paths.video_path,
        VideoTrack::Audio => &paths.audio_path,
    }
}

fn track_checkpoint_path(paths: &VideoWorkspacePaths, track: VideoTrack) -> &PathBuf {
    match track {
        VideoTrack::Video => &paths.video_checkpoint_path,
        VideoTrack::Audio => &paths.audio_checkpoint_path,
    }
}

pub(super) fn registered_paths(paths: &VideoWorkspacePaths) -> Vec<String> {
    [
        &paths.video_path,
        &paths.audio_path,
        &paths.video_checkpoint_path,
        &paths.audio_checkpoint_path,
        &paths.processed_path,
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::connection_budget;
    use crate::{infrastructure::video::VideoTrack, models::DownloadMode};

    #[test]
    fn connection_budget_preserves_single_and_splits_dual_limits() {
        assert_eq!(
            connection_budget(0, DownloadMode::VideoOnly, VideoTrack::Video),
            1
        );
        assert_eq!(
            connection_budget(1, DownloadMode::VideoAudio, VideoTrack::Video),
            1
        );
        assert_eq!(
            connection_budget(1, DownloadMode::VideoAudio, VideoTrack::Audio),
            1
        );
        assert_eq!(
            connection_budget(4, DownloadMode::VideoAudio, VideoTrack::Video),
            2
        );
        assert_eq!(
            connection_budget(4, DownloadMode::VideoAudio, VideoTrack::Audio),
            2
        );
        assert_eq!(
            connection_budget(3, DownloadMode::VideoAudio, VideoTrack::Video),
            1
        );
        assert_eq!(
            connection_budget(3, DownloadMode::VideoAudio, VideoTrack::Audio),
            2
        );
    }
}
