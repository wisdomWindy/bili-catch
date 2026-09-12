use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::{AppError, AppErrorCode};

const CHECKPOINT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRequest {
    pub task_id: String,
    pub attempt_id: String,
    pub temporary_root: PathBuf,
    pub output_directory: PathBuf,
    pub final_file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    pub temporary_root: PathBuf,
    pub output_directory: PathBuf,
    pub task_root: PathBuf,
    pub source_path: PathBuf,
    pub cover_path: PathBuf,
    pub checkpoint_path: PathBuf,
    pub processed_path: PathBuf,
    pub final_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointV1 {
    pub schema_version: u32,
    pub task_id: String,
    pub source_identity_hash: String,
    pub completed_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

pub trait AudioWorkspace: Send + Sync {
    fn prepare(&self, request: &WorkspaceRequest) -> Result<WorkspacePaths, AppError>;
    fn save_checkpoint(
        &self,
        paths: &WorkspacePaths,
        checkpoint: &CheckpointV1,
    ) -> Result<(), AppError>;
    fn load_compatible_checkpoint(
        &self,
        paths: &WorkspacePaths,
        task_id: &str,
        source_identity_hash: &str,
        total_bytes: Option<u64>,
        etag: Option<&str>,
    ) -> Result<Option<CheckpointV1>, AppError>;
    fn discard_download(&self, paths: &WorkspacePaths) -> Result<(), AppError>;
    fn discard_processed(&self, paths: &WorkspacePaths) -> Result<(), AppError>;
    fn finalize(&self, paths: &WorkspacePaths) -> Result<PathBuf, AppError>;
    fn cleanup(&self, paths: &WorkspacePaths) -> Result<(), AppError>;
}

pub struct FsAudioWorkspace;

fn io_error(error: std::io::Error) -> AppError {
    if matches!(error.raw_os_error(), Some(28 | 112)) {
        AppError::new(AppErrorCode::E007, "There is not enough disk space")
    } else {
        AppError::internal("Unable to access the audio workspace")
    }
}

fn invalid_workspace() -> AppError {
    AppError::internal("The audio workspace path is invalid")
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

fn validate_paths(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.task_root == paths.temporary_root
        || !paths.task_root.starts_with(&paths.temporary_root)
        || !paths.source_path.starts_with(&paths.task_root)
        || !paths.cover_path.starts_with(&paths.task_root)
        || !paths.checkpoint_path.starts_with(&paths.task_root)
        || !paths.processed_path.starts_with(&paths.output_directory)
        || !paths.final_path.starts_with(&paths.output_directory)
    {
        return Err(invalid_workspace());
    }
    Ok(())
}

fn discard_checkpoint(paths: &WorkspacePaths) -> Result<(), AppError> {
    for path in [&paths.source_path, &paths.checkpoint_path] {
        if path.exists() {
            fs::remove_file(path).map_err(io_error)?;
        }
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
    let name = match extension {
        Some(extension) => format!("{stem} ({index}).{extension}"),
        None => format!("{stem} ({index})"),
    };
    Ok(parent.join(name))
}

impl AudioWorkspace for FsAudioWorkspace {
    fn prepare(&self, request: &WorkspaceRequest) -> Result<WorkspacePaths, AppError> {
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
        let paths = WorkspacePaths {
            temporary_root,
            output_directory: output_directory.clone(),
            source_path: task_root.join("source.partial"),
            cover_path: task_root.join("cover.image"),
            checkpoint_path: task_root.join("checkpoint.json"),
            processed_path: output_directory.join(processed_name),
            final_path: output_directory.join(&request.final_file_name),
            task_root,
        };
        validate_paths(&paths)?;
        Ok(paths)
    }

    fn save_checkpoint(
        &self,
        paths: &WorkspacePaths,
        checkpoint: &CheckpointV1,
    ) -> Result<(), AppError> {
        validate_paths(paths)?;
        if checkpoint.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(invalid_workspace());
        }
        let bytes = serde_json::to_vec(checkpoint)
            .map_err(|_| AppError::internal("Unable to encode the audio checkpoint"))?;
        let next = paths.task_root.join("checkpoint.json.next");
        fs::write(&next, bytes).map_err(io_error)?;
        if paths.checkpoint_path.exists() {
            fs::remove_file(&paths.checkpoint_path).map_err(io_error)?;
        }
        fs::rename(next, &paths.checkpoint_path).map_err(io_error)
    }

    fn load_compatible_checkpoint(
        &self,
        paths: &WorkspacePaths,
        task_id: &str,
        source_identity_hash: &str,
        total_bytes: Option<u64>,
        etag: Option<&str>,
    ) -> Result<Option<CheckpointV1>, AppError> {
        validate_paths(paths)?;
        if !paths.checkpoint_path.exists() {
            if paths.source_path.exists() {
                fs::remove_file(&paths.source_path).map_err(io_error)?;
            }
            return Ok(None);
        }
        let checkpoint = fs::read(&paths.checkpoint_path)
            .map_err(io_error)
            .and_then(|bytes| {
                serde_json::from_slice::<CheckpointV1>(&bytes)
                    .map_err(|_| AppError::internal("The audio checkpoint is invalid"))
            });
        let compatible = checkpoint.ok().filter(|checkpoint| {
            let source_length = fs::metadata(&paths.source_path).map(|item| item.len()).ok();
            checkpoint.schema_version == CHECKPOINT_SCHEMA_VERSION
                && checkpoint.task_id == task_id
                && checkpoint.source_identity_hash == source_identity_hash
                && source_length == Some(checkpoint.completed_bytes)
                && checkpoint
                    .total_bytes
                    .is_none_or(|total| checkpoint.completed_bytes <= total)
                && total_bytes.is_none_or(|total| checkpoint.total_bytes == Some(total))
                && etag.is_none_or(|value| checkpoint.etag.as_deref() == Some(value))
        });
        if compatible.is_none() {
            discard_checkpoint(paths)?;
        }
        Ok(compatible)
    }

    fn finalize(&self, paths: &WorkspacePaths) -> Result<PathBuf, AppError> {
        validate_paths(paths)?;
        if !fs::metadata(&paths.processed_path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
        {
            return Err(AppError::internal(
                "The processed audio output is unavailable",
            ));
        }
        for index in 0..10_000 {
            let candidate = if index == 0 {
                paths.final_path.clone()
            } else {
                collision_path(&paths.final_path, index)?
            };
            match fs::hard_link(&paths.processed_path, &candidate) {
                Ok(()) => {
                    let _ = fs::remove_file(&paths.processed_path);
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(io_error(error)),
            }
        }
        Err(AppError::internal("Unable to reserve an audio output name"))
    }

    fn discard_download(&self, paths: &WorkspacePaths) -> Result<(), AppError> {
        validate_paths(paths)?;
        discard_checkpoint(paths)
    }

    fn discard_processed(&self, paths: &WorkspacePaths) -> Result<(), AppError> {
        validate_paths(paths)?;
        if paths.processed_path.exists() {
            fs::remove_file(&paths.processed_path).map_err(io_error)?;
        }
        Ok(())
    }

    fn cleanup(&self, paths: &WorkspacePaths) -> Result<(), AppError> {
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
