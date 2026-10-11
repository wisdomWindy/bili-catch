use std::path::Path;

use sha2::{Digest, Sha256};

use crate::models::{AsrModelId, TranscriptionLanguage};

pub fn source_fingerprint(
    path: &Path,
    size_bytes: u64,
    modified_millis: u128,
    model_id: AsrModelId,
    language: TranscriptionLanguage,
    punctuation: bool,
) -> String {
    let normalized_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let mut digest = Sha256::new();
    for value in [
        normalized_path,
        size_bytes.to_string(),
        modified_millis.to_string(),
        format!("{model_id:?}"),
        format!("{language:?}"),
        punctuation.to_string(),
    ] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}
