use std::{fs, path::PathBuf};

use crate::{
    models::{AppError, AppErrorCode},
    services::transcription::{TranscriptionDocument, TranscriptionStore},
};

#[derive(Debug, Clone)]
pub struct JsonTranscriptionStore {
    path: PathBuf,
}

impl JsonTranscriptionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn next_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.next", self.path.display()))
    }
}

impl TranscriptionStore for JsonTranscriptionStore {
    fn load(&self) -> Result<Option<TranscriptionDocument>, AppError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&self.path).map_err(|_| store_error("TRANSCRIPTION_STORE_IO"))?;
        let document = serde_json::from_slice::<TranscriptionDocument>(&bytes)
            .map_err(|_| store_error("TRANSCRIPTION_STORE_CORRUPT"))?;
        Ok(Some(document))
    }

    fn save(&self, document: &TranscriptionDocument) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| store_error("TRANSCRIPTION_STORE_IO"))?;
        }
        let bytes = serde_json::to_vec_pretty(document)
            .map_err(|_| store_error("TRANSCRIPTION_STORE_SERIALIZE"))?;
        let next_path = self.next_path();
        fs::write(&next_path, bytes).map_err(|_| store_error("TRANSCRIPTION_STORE_IO"))?;
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|_| store_error("TRANSCRIPTION_STORE_IO"))?;
        }
        fs::rename(&next_path, &self.path).map_err(|_| store_error("TRANSCRIPTION_STORE_IO"))
    }
}

fn store_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: "Transcription storage is unavailable".into(),
        details: Some(details.into()),
    }
}
