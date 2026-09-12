mod mux;

pub use mux::{
    build_video_mux_args, validate_video_mux_inputs, DeferredFfmpegVideoMuxer, FfmpegVideoMuxer,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::{AppError, AppErrorCode};

const CHECKPOINT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoWorkspaceRequest {
    pub task_id: String,
    pub attempt_id: String,
    pub temporary_root: PathBuf,
    pub output_directory: PathBuf,
    pub final_file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoWorkspacePaths {
    pub temporary_root: PathBuf,
    pub output_directory: PathBuf,
    pub task_root: PathBuf,
    pub video_path: PathBuf,
    pub audio_path: PathBuf,
    pub video_checkpoint_path: PathBuf,
    pub audio_checkpoint_path: PathBuf,
    pub processed_path: PathBuf,
    pub final_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoTrack {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCheckpointV1 {
    pub schema_version: u32,
    pub task_id: String,
    pub track_kind: VideoTrack,
    pub source_identity_hash: String,
    pub completed_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

pub trait VideoWorkspace: Send + Sync {
    fn prepare(&self, request: &VideoWorkspaceRequest) -> Result<VideoWorkspacePaths, AppError>;
    fn save_checkpoint(
        &self,
        paths: &VideoWorkspacePaths,
        checkpoint: &VideoCheckpointV1,
    ) -> Result<(), AppError>;
    fn load_compatible_checkpoint(
        &self,
        paths: &VideoWorkspacePaths,
        track: VideoTrack,
        task_id: &str,
        source_identity_hash: &str,
        total_bytes: Option<u64>,
        etag: Option<&str>,
    ) -> Result<Option<VideoCheckpointV1>, AppError>;
    fn discard_track(&self, paths: &VideoWorkspacePaths, track: VideoTrack)
        -> Result<(), AppError>;
    fn finalize_video_only(&self, paths: &VideoWorkspacePaths) -> Result<PathBuf, AppError>;
    fn finalize_processed(&self, paths: &VideoWorkspacePaths) -> Result<PathBuf, AppError>;
    fn discard_processed(&self, paths: &VideoWorkspacePaths) -> Result<(), AppError>;
    fn cleanup(&self, paths: &VideoWorkspacePaths) -> Result<(), AppError>;
}

pub struct FsVideoWorkspace;

fn io_error(error: std::io::Error) -> AppError {
    if matches!(error.raw_os_error(), Some(28 | 112)) {
        AppError::new(AppErrorCode::E007, "There is not enough disk space")
    } else {
        AppError::internal("Unable to access the video workspace")
    }
}

fn invalid_workspace() -> AppError {
    AppError::internal("The video workspace path is invalid")
}

fn canonical_directory(path: &Path) -> Result<PathBuf, AppError> {
    fs::create_dir_all(path).map_err(io_error)?;
    path.canonicalize().map_err(io_error)
}

fn validate_file_name(value: &str) -> Result<(), AppError> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || path
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
        || path.file_name().is_none()
    {
        Err(invalid_workspace())
    } else {
        Ok(())
    }
}

fn task_hash(value: &str) -> String {
    format!("{:x}", md5::compute(value.as_bytes()))
}

fn checkpoint_path(paths: &VideoWorkspacePaths, track: VideoTrack) -> &Path {
    match track {
        VideoTrack::Video => &paths.video_checkpoint_path,
        VideoTrack::Audio => &paths.audio_checkpoint_path,
    }
}

fn partial_path(paths: &VideoWorkspacePaths, track: VideoTrack) -> &Path {
    match track {
        VideoTrack::Video => &paths.video_path,
        VideoTrack::Audio => &paths.audio_path,
    }
}

fn validate_paths(paths: &VideoWorkspacePaths) -> Result<(), AppError> {
    let contained = [
        &paths.video_path,
        &paths.audio_path,
        &paths.video_checkpoint_path,
        &paths.audio_checkpoint_path,
    ]
    .iter()
    .all(|path| path.starts_with(&paths.task_root));
    if paths.task_root == paths.temporary_root
        || !paths.task_root.starts_with(&paths.temporary_root)
        || !contained
        || !paths.processed_path.starts_with(&paths.output_directory)
        || !paths.final_path.starts_with(&paths.output_directory)
    {
        return Err(invalid_workspace());
    }
    Ok(())
}

fn collision_path(original: &Path, index: u32) -> Result<PathBuf, AppError> {
    let parent = original.parent().ok_or_else(invalid_workspace)?;
    let stem = original
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(invalid_workspace)?;
    let extension = original.extension().and_then(|value| value.to_str());
    Ok(parent.join(match extension {
        Some(extension) => format!("{stem} ({index}).{extension}"),
        None => format!("{stem} ({index})"),
    }))
}

impl VideoWorkspace for FsVideoWorkspace {
    fn prepare(&self, request: &VideoWorkspaceRequest) -> Result<VideoWorkspacePaths, AppError> {
        validate_file_name(&request.final_file_name)?;
        let temporary_root = canonical_directory(&request.temporary_root)?;
        let output_directory = canonical_directory(&request.output_directory)?;
        let task_root = temporary_root.join(task_hash(&request.task_id));
        fs::create_dir_all(&task_root).map_err(io_error)?;
        let task_root = task_root.canonicalize().map_err(io_error)?;
        let extension = Path::new(&request.final_file_name)
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(invalid_workspace)?;
        let processed_name = format!(
            ".bilicatch-{}-{}.processing.{extension}",
            task_hash(&request.task_id),
            task_hash(&request.attempt_id)
        );
        let paths = VideoWorkspacePaths {
            temporary_root,
            output_directory: output_directory.clone(),
            task_root: task_root.clone(),
            video_path: task_root.join("video.partial"),
            audio_path: task_root.join("audio.partial"),
            video_checkpoint_path: task_root.join("video.checkpoint.json"),
            audio_checkpoint_path: task_root.join("audio.checkpoint.json"),
            processed_path: output_directory.join(processed_name),
            final_path: output_directory.join(&request.final_file_name),
        };
        validate_paths(&paths)?;
        Ok(paths)
    }

    fn save_checkpoint(
        &self,
        paths: &VideoWorkspacePaths,
        checkpoint: &VideoCheckpointV1,
    ) -> Result<(), AppError> {
        validate_paths(paths)?;
        if checkpoint.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(invalid_workspace());
        }
        let target = checkpoint_path(paths, checkpoint.track_kind);
        let next = target.with_extension("json.next");
        fs::write(
            &next,
            serde_json::to_vec(checkpoint).map_err(|_| invalid_workspace())?,
        )
        .map_err(io_error)?;
        if target.exists() {
            fs::remove_file(target).map_err(io_error)?;
        }
        fs::rename(next, target).map_err(io_error)
    }

    fn load_compatible_checkpoint(
        &self,
        paths: &VideoWorkspacePaths,
        track: VideoTrack,
        task_id: &str,
        source_identity_hash: &str,
        total_bytes: Option<u64>,
        etag: Option<&str>,
    ) -> Result<Option<VideoCheckpointV1>, AppError> {
        validate_paths(paths)?;
        let checkpoint_path = checkpoint_path(paths, track);
        let partial_path = partial_path(paths, track);
        if !checkpoint_path.exists() {
            if partial_path.exists() {
                fs::remove_file(partial_path).map_err(io_error)?;
            }
            return Ok(None);
        }
        let loaded = fs::read(checkpoint_path)
            .map_err(io_error)
            .and_then(|bytes| {
                serde_json::from_slice::<VideoCheckpointV1>(&bytes).map_err(|_| invalid_workspace())
            });
        let compatible = loaded.ok().filter(|checkpoint| {
            checkpoint.schema_version == CHECKPOINT_SCHEMA_VERSION
                && checkpoint.track_kind == track
                && checkpoint.task_id == task_id
                && checkpoint.source_identity_hash == source_identity_hash
                && fs::metadata(partial_path).map(|item| item.len()).ok()
                    == Some(checkpoint.completed_bytes)
                && checkpoint
                    .total_bytes
                    .is_none_or(|total| checkpoint.completed_bytes <= total)
                && total_bytes.is_none_or(|total| checkpoint.total_bytes == Some(total))
                && etag.is_none_or(|value| checkpoint.etag.as_deref() == Some(value))
        });
        if compatible.is_none() {
            self.discard_track(paths, track)?;
        }
        Ok(compatible)
    }

    fn discard_track(
        &self,
        paths: &VideoWorkspacePaths,
        track: VideoTrack,
    ) -> Result<(), AppError> {
        validate_paths(paths)?;
        for path in [partial_path(paths, track), checkpoint_path(paths, track)] {
            if path.exists() {
                fs::remove_file(path).map_err(io_error)?;
            }
        }
        Ok(())
    }

    fn finalize_video_only(&self, paths: &VideoWorkspacePaths) -> Result<PathBuf, AppError> {
        self.finalize_source(paths, &paths.video_path)
    }

    fn finalize_processed(&self, paths: &VideoWorkspacePaths) -> Result<PathBuf, AppError> {
        self.finalize_source(paths, &paths.processed_path)
    }

    fn discard_processed(&self, paths: &VideoWorkspacePaths) -> Result<(), AppError> {
        validate_paths(paths)?;
        if paths.processed_path.exists() {
            fs::remove_file(&paths.processed_path).map_err(io_error)?;
        }
        Ok(())
    }

    fn cleanup(&self, paths: &VideoWorkspacePaths) -> Result<(), AppError> {
        validate_paths(paths)?;
        if paths.processed_path.exists() {
            fs::remove_file(&paths.processed_path).map_err(io_error)?;
        }
        if paths.task_root.exists() {
            fs::remove_dir_all(&paths.task_root).map_err(io_error)?;
        }
        Ok(())
    }
}

impl FsVideoWorkspace {
    fn finalize_source(
        &self,
        paths: &VideoWorkspacePaths,
        source_path: &Path,
    ) -> Result<PathBuf, AppError> {
        validate_paths(paths)?;
        if !fs::metadata(source_path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
        {
            return Err(AppError::internal("The video output is unavailable"));
        }
        for index in 0..10_000 {
            let candidate = if index == 0 {
                paths.final_path.clone()
            } else {
                collision_path(&paths.final_path, index)?
            };
            match fs::hard_link(source_path, &candidate) {
                Ok(()) => {
                    fs::remove_file(source_path).map_err(io_error)?;
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(io_error(error)),
            }
        }
        Err(AppError::internal("Unable to reserve a video output name"))
    }
}
