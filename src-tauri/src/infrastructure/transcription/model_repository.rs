use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use sha2::{Digest, Sha256};
use url::Url;

use crate::models::{AppError, AppErrorCode, AsrModel, AsrModelId, ModelDownloadState};

const DEFAULT_MODEL_MIRROR: &str = "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelManifestEntry {
    pub id: AsrModelId,
    pub display_name: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub official_url: String,
    pub recommended_for: String,
    pub capabilities: Vec<String>,
}

pub fn official_model_manifest() -> Vec<ModelManifestEntry> {
    const BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
    vec![
        manifest_entry(
            AsrModelId::Tiny,
            "Tiny",
            "ggml-tiny.bin",
            77_691_713,
            "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
            BASE,
            "Fast drafts on low-resource devices",
        ),
        manifest_entry(
            AsrModelId::Base,
            "Base",
            "ggml-base.bin",
            147_951_465,
            "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
            BASE,
            "Recommended default for everyday transcription",
        ),
        manifest_entry(
            AsrModelId::Small,
            "Small",
            "ggml-small.bin",
            487_601_967,
            "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
            BASE,
            "Higher accuracy with moderate resource use",
        ),
        manifest_entry(
            AsrModelId::Medium,
            "Medium",
            "ggml-medium.bin",
            1_533_763_059,
            "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
            BASE,
            "High accuracy on capable desktop systems",
        ),
        manifest_entry(
            AsrModelId::LargeV3,
            "Large v3",
            "ggml-large-v3.bin",
            3_095_033_483,
            "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
            BASE,
            "Best accuracy with high memory and CPU requirements",
        ),
    ]
}

fn manifest_entry(
    id: AsrModelId,
    display_name: &str,
    file_name: &str,
    size_bytes: u64,
    sha256: &str,
    base_url: &str,
    recommended_for: &str,
) -> ModelManifestEntry {
    ModelManifestEntry {
        id,
        display_name: display_name.into(),
        file_name: file_name.into(),
        size_bytes,
        sha256: sha256.into(),
        official_url: format!("{base_url}/{file_name}"),
        recommended_for: recommended_for.into(),
        capabilities: vec!["multilingual".into(), "timestamps".into()],
    }
}

#[derive(Debug, Clone)]
pub struct FileModelRepository {
    model_directory: PathBuf,
    manifest: Vec<ModelManifestEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelDownloadCancellation {
    cancelled: Arc<AtomicBool>,
}

impl ModelDownloadCancellation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub struct ModelDownloadWriter {
    repository: FileModelRepository,
    model_id: AsrModelId,
    default_model: AsrModelId,
    expected_size: u64,
    part_path: PathBuf,
    file: Option<File>,
    downloaded_bytes: u64,
    cancellation: ModelDownloadCancellation,
    committed: bool,
}

impl ModelDownloadWriter {
    pub fn append(&mut self, chunk: &[u8]) -> Result<u64, AppError> {
        if self.cancellation.is_cancelled() {
            return Err(download_error("MODEL_DOWNLOAD_CANCELLED"));
        }
        let next_size = self
            .downloaded_bytes
            .checked_add(chunk.len() as u64)
            .filter(|size| *size <= self.expected_size)
            .ok_or_else(|| download_error("MODEL_SIZE_MISMATCH"))?;
        self.file
            .as_mut()
            .ok_or_else(|| download_error("MODEL_DOWNLOAD_CLOSED"))?
            .write_all(chunk)
            .map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        self.downloaded_bytes = next_size;
        Ok(next_size)
    }

    pub fn finish(mut self) -> Result<AsrModel, AppError> {
        if self.cancellation.is_cancelled() {
            return Err(download_error("MODEL_DOWNLOAD_CANCELLED"));
        }
        if self.downloaded_bytes != self.expected_size {
            return Err(download_error("MODEL_SIZE_MISMATCH"));
        }
        if let Some(file) = self.file.take() {
            file.sync_all()
                .map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        }
        let model =
            self.repository
                .publish_part(self.model_id, &self.part_path, self.default_model)?;
        self.committed = true;
        Ok(model)
    }
}

impl Drop for ModelDownloadWriter {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.part_path);
        }
    }
}

#[derive(Clone)]
pub struct ReqwestModelDownloader {
    client: reqwest::Client,
    allow_insecure_loopback: bool,
}

impl std::fmt::Debug for ReqwestModelDownloader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReqwestModelDownloader")
            .field("allow_insecure_loopback", &self.allow_insecure_loopback)
            .finish_non_exhaustive()
    }
}

impl ReqwestModelDownloader {
    pub fn new() -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.url().scheme() == "https" {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|_| download_error("MODEL_HTTP_CLIENT_UNAVAILABLE"))?;
        Ok(Self {
            client,
            allow_insecure_loopback: false,
        })
    }

    #[cfg(test)]
    fn new_insecure_loopback_for_test() -> Self {
        Self {
            client: reqwest::Client::new(),
            allow_insecure_loopback: true,
        }
    }

    pub async fn download<F>(
        &self,
        repository: &FileModelRepository,
        model_id: AsrModelId,
        default_model: AsrModelId,
        custom_source: &str,
        cancellation: ModelDownloadCancellation,
        on_progress: F,
    ) -> Result<AsrModel, AppError>
    where
        F: FnMut(u64, u64),
    {
        let urls = repository.download_urls(model_id, custom_source)?;
        self.download_from_urls(
            repository,
            model_id,
            default_model,
            urls,
            cancellation,
            on_progress,
        )
        .await
    }

    async fn download_from_urls<F>(
        &self,
        repository: &FileModelRepository,
        model_id: AsrModelId,
        default_model: AsrModelId,
        urls: Vec<Url>,
        cancellation: ModelDownloadCancellation,
        mut on_progress: F,
    ) -> Result<AsrModel, AppError>
    where
        F: FnMut(u64, u64),
    {
        let candidate_count = urls.len();
        for (index, url) in urls.into_iter().enumerate() {
            let result = self
                .download_from_url(
                    repository,
                    model_id,
                    default_model,
                    url,
                    cancellation.clone(),
                    &mut on_progress,
                )
                .await;
            match result {
                Ok(model) => return Ok(model),
                Err(error)
                    if index + 1 < candidate_count
                        && !cancellation.is_cancelled()
                        && retryable_source_error(&error) => {}
                Err(error) => return Err(error),
            }
        }
        Err(download_error("MODEL_REQUEST_FAILED"))
    }

    async fn download_from_url<F>(
        &self,
        repository: &FileModelRepository,
        model_id: AsrModelId,
        default_model: AsrModelId,
        url: Url,
        cancellation: ModelDownloadCancellation,
        on_progress: &mut F,
    ) -> Result<AsrModel, AppError>
    where
        F: FnMut(u64, u64),
    {
        self.validate_url(&url)?;
        if cancellation.is_cancelled() {
            return Err(download_error("MODEL_DOWNLOAD_CANCELLED"));
        }
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| download_error("MODEL_REQUEST_FAILED"))?;
        self.validate_url(response.url())?;
        if !response.status().is_success() {
            return Err(download_error("MODEL_REQUEST_FAILED"));
        }
        let expected_size = repository.expected_size(model_id)?;
        if response
            .content_length()
            .is_some_and(|content_length| content_length != expected_size)
        {
            return Err(download_error("MODEL_SIZE_MISMATCH"));
        }
        let mut writer = repository.begin_download(model_id, default_model, cancellation)?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| download_error("MODEL_STREAM_FAILED"))?
        {
            let downloaded = writer.append(&chunk)?;
            on_progress(downloaded, expected_size);
        }
        writer.finish()
    }

    fn validate_url(&self, url: &Url) -> Result<(), AppError> {
        let allowed_loopback = self.allow_insecure_loopback
            && url.scheme() == "http"
            && url
                .host_str()
                .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"));
        if url.scheme() == "https" || allowed_loopback {
            Ok(())
        } else {
            Err(download_error("MODEL_REDIRECT_INSECURE"))
        }
    }
}

impl FileModelRepository {
    pub fn new(model_directory: impl Into<PathBuf>) -> Self {
        Self::with_manifest(model_directory, official_model_manifest())
    }

    pub fn with_manifest(
        model_directory: impl Into<PathBuf>,
        manifest: Vec<ModelManifestEntry>,
    ) -> Self {
        Self {
            model_directory: model_directory.into(),
            manifest,
        }
    }

    pub fn begin_download(
        &self,
        model_id: AsrModelId,
        default_model: AsrModelId,
        cancellation: ModelDownloadCancellation,
    ) -> Result<ModelDownloadWriter, AppError> {
        let entry = self.entry(model_id)?;
        fs::create_dir_all(&self.model_directory)
            .map_err(|_| download_error("MODEL_STORAGE_UNAVAILABLE"))?;
        let part_path = self.part_path(entry);
        remove_if_exists(&part_path)?;
        let file =
            File::create(&part_path).map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        Ok(ModelDownloadWriter {
            repository: self.clone(),
            model_id,
            default_model,
            expected_size: entry.size_bytes,
            part_path,
            file: Some(file),
            downloaded_bytes: 0,
            cancellation,
            committed: false,
        })
    }

    pub fn list(&self, default_model: AsrModelId) -> Result<Vec<AsrModel>, AppError> {
        fs::create_dir_all(&self.model_directory).map_err(model_io_error)?;
        self.manifest
            .iter()
            .map(|entry| self.model_state(entry, default_model))
            .collect()
    }

    pub fn verify(
        &self,
        model_id: AsrModelId,
        default_model: AsrModelId,
    ) -> Result<AsrModel, AppError> {
        let entry = self.entry(model_id)?;
        let final_path = self.final_path(entry);
        let part_path = self.part_path(entry);
        if !file_matches(&final_path, entry)? {
            remove_if_exists(&final_path)?;
            remove_if_exists(&part_path)?;
            return Err(integrity_error("MODEL_CHECKSUM_MISMATCH"));
        }
        self.model_state(entry, default_model)
    }

    pub fn publish_part(
        &self,
        model_id: AsrModelId,
        part_path: &Path,
        default_model: AsrModelId,
    ) -> Result<AsrModel, AppError> {
        let entry = self.entry(model_id)?;
        let expected_part = self.part_path(entry);
        if part_path != expected_part || !file_matches(part_path, entry)? {
            remove_if_exists(part_path)?;
            return Err(integrity_error("MODEL_CHECKSUM_MISMATCH"));
        }
        let final_path = self.final_path(entry);
        if final_path.exists() {
            return Err(integrity_error("MODEL_ALREADY_EXISTS"));
        }
        fs::rename(part_path, &final_path).map_err(model_io_error)?;
        self.model_state(entry, default_model)
    }

    pub fn delete(&self, model_id: AsrModelId, in_use: bool) -> Result<(), AppError> {
        if in_use {
            return Err(integrity_error("MODEL_IN_USE"));
        }
        let entry = self.entry(model_id)?;
        remove_if_exists(&self.part_path(entry))?;
        remove_if_exists(&self.final_path(entry))
    }

    pub fn download_url(&self, model_id: AsrModelId, custom_source: &str) -> Result<Url, AppError> {
        let entry = self.entry(model_id)?;
        if custom_source.trim().is_empty() {
            return Url::parse(&entry.official_url)
                .map_err(|_| download_error("MODEL_URL_INVALID"));
        }
        let source =
            Url::parse(custom_source.trim()).map_err(|_| download_error("MODEL_SOURCE_INVALID"))?;
        if source.scheme() != "https" || source.query().is_some() || source.fragment().is_some() {
            return Err(download_error("MODEL_SOURCE_INVALID"));
        }
        source
            .join(&entry.file_name)
            .map_err(|_| download_error("MODEL_SOURCE_INVALID"))
    }

    fn download_urls(
        &self,
        model_id: AsrModelId,
        custom_source: &str,
    ) -> Result<Vec<Url>, AppError> {
        let primary = self.download_url(model_id, custom_source)?;
        if !custom_source.trim().is_empty() {
            return Ok(vec![primary]);
        }
        let entry = self.entry(model_id)?;
        let mirror = Url::parse(DEFAULT_MODEL_MIRROR)
            .and_then(|base| base.join(&entry.file_name))
            .map_err(|_| download_error("MODEL_URL_INVALID"))?;
        Ok(vec![primary, mirror])
    }

    pub fn part_path_for(&self, model_id: AsrModelId) -> Result<PathBuf, AppError> {
        Ok(self.part_path(self.entry(model_id)?))
    }

    fn expected_size(&self, model_id: AsrModelId) -> Result<u64, AppError> {
        Ok(self.entry(model_id)?.size_bytes)
    }

    fn entry(&self, model_id: AsrModelId) -> Result<&ModelManifestEntry, AppError> {
        self.manifest
            .iter()
            .find(|entry| entry.id == model_id)
            .ok_or_else(|| integrity_error("MODEL_UNKNOWN"))
    }

    fn model_state(
        &self,
        entry: &ModelManifestEntry,
        default_model: AsrModelId,
    ) -> Result<AsrModel, AppError> {
        let path = self.final_path(entry);
        let installed = path.is_file();
        let valid = installed && file_matches(&path, entry)?;
        Ok(AsrModel {
            id: entry.id,
            display_name: entry.display_name.clone(),
            file_name: entry.file_name.clone(),
            size_bytes: entry.size_bytes,
            installed,
            valid,
            is_default: entry.id == default_model,
            in_use: false,
            path: installed.then(|| path.to_string_lossy().into_owned()),
            recommended_for: entry.recommended_for.clone(),
            capabilities: entry.capabilities.clone(),
            download_state: if valid {
                ModelDownloadState::Completed
            } else {
                ModelDownloadState::Idle
            },
            downloaded_bytes: if installed {
                fs::metadata(&path).map_err(model_io_error)?.len()
            } else {
                0
            },
            error: None,
            revision: 0,
        })
    }

    fn final_path(&self, entry: &ModelManifestEntry) -> PathBuf {
        self.model_directory.join(&entry.file_name)
    }

    fn part_path(&self, entry: &ModelManifestEntry) -> PathBuf {
        self.model_directory
            .join(format!("{}.part", entry.file_name))
    }
}

fn file_matches(path: &Path, entry: &ModelManifestEntry) -> Result<bool, AppError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(model_io_error(error)),
    };
    if !metadata.is_file() || metadata.len() != entry.size_bytes {
        return Ok(false);
    }
    let file = File::open(path).map_err(model_io_error)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(model_io_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()) == entry.sha256)
}

fn remove_if_exists(path: &Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(model_io_error(error)),
    }
}

fn model_io_error(_error: std::io::Error) -> AppError {
    integrity_error("MODEL_STORAGE_UNAVAILABLE")
}

fn integrity_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::E017,
        message: "The ASR model failed integrity validation".into(),
        details: Some(details.into()),
    }
}

fn download_error(details: &str) -> AppError {
    AppError {
        code: AppErrorCode::E012,
        message: "The ASR model could not be downloaded".into(),
        details: Some(details.into()),
    }
}

fn retryable_source_error(error: &AppError) -> bool {
    matches!(
        error.details.as_deref(),
        Some(
            "MODEL_REQUEST_FAILED"
                | "MODEL_STREAM_FAILED"
                | "MODEL_SIZE_MISMATCH"
                | "MODEL_CHECKSUM_MISMATCH"
                | "MODEL_REDIRECT_INSECURE"
        )
    )
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::{
        FileModelRepository, ModelDownloadCancellation, ModelManifestEntry, ReqwestModelDownloader,
    };
    use crate::models::AsrModelId;

    #[tokio::test]
    async fn reqwest_downloader_streams_a_response_through_verified_publish() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("local request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).expect("request headers");
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc";
            stream.write_all(response).expect("response");
        });
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(
            root.path(),
            vec![ModelManifestEntry {
                id: AsrModelId::Tiny,
                display_name: "Tiny".into(),
                file_name: "ggml-tiny.bin".into(),
                size_bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                official_url: format!("http://{address}/ggml-tiny.bin"),
                recommended_for: "test".into(),
                capabilities: vec![],
            }],
        );
        let mut progress = Vec::new();

        let model = ReqwestModelDownloader::new_insecure_loopback_for_test()
            .download(
                &repo,
                AsrModelId::Tiny,
                AsrModelId::Tiny,
                "",
                ModelDownloadCancellation::new(),
                |downloaded, total| progress.push((downloaded, total)),
            )
            .await
            .expect("download should publish");

        server.join().expect("server thread");
        assert!(model.valid);
        assert_eq!(progress.last(), Some(&(3, 3)));
        assert_eq!(
            std::fs::read(root.path().join("ggml-tiny.bin")).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn default_download_sources_keep_official_first_and_add_the_verified_mirror() {
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(
            root.path(),
            vec![ModelManifestEntry {
                id: AsrModelId::Tiny,
                display_name: "Tiny".into(),
                file_name: "ggml-tiny.bin".into(),
                size_bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                official_url:
                    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
                recommended_for: "test".into(),
                capabilities: vec![],
            }],
        );

        let urls = repo
            .download_urls(AsrModelId::Tiny, "")
            .expect("default download sources");

        assert_eq!(
            urls.iter().map(url::Url::as_str).collect::<Vec<_>>(),
            vec![
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
                "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            ]
        );
    }

    #[tokio::test]
    async fn reqwest_downloader_falls_back_after_the_primary_source_fails() {
        let primary = TcpListener::bind("127.0.0.1:0").expect("primary listener");
        let primary_address = primary.local_addr().expect("primary address");
        let primary_server = thread::spawn(move || {
            let (mut stream, _) = primary.accept().expect("primary request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).expect("primary request headers");
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("primary response");
        });
        let fallback = TcpListener::bind("127.0.0.1:0").expect("fallback listener");
        let fallback_address = fallback.local_addr().expect("fallback address");
        let fallback_server = thread::spawn(move || {
            let (mut stream, _) = fallback.accept().expect("fallback request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).expect("fallback request headers");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc")
                .expect("fallback response");
        });
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(
            root.path(),
            vec![ModelManifestEntry {
                id: AsrModelId::Tiny,
                display_name: "Tiny".into(),
                file_name: "ggml-tiny.bin".into(),
                size_bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                official_url: format!("http://{primary_address}/ggml-tiny.bin"),
                recommended_for: "test".into(),
                capabilities: vec![],
            }],
        );

        let model = ReqwestModelDownloader::new_insecure_loopback_for_test()
            .download_from_urls(
                &repo,
                AsrModelId::Tiny,
                AsrModelId::Tiny,
                vec![
                    format!("http://{primary_address}/ggml-tiny.bin")
                        .parse()
                        .expect("primary URL"),
                    format!("http://{fallback_address}/ggml-tiny.bin")
                        .parse()
                        .expect("fallback URL"),
                ],
                ModelDownloadCancellation::new(),
                |_, _| {},
            )
            .await
            .expect("fallback download should publish");

        primary_server.join().expect("primary server thread");
        fallback_server.join().expect("fallback server thread");
        assert!(model.valid);
        assert_eq!(
            std::fs::read(root.path().join("ggml-tiny.bin")).unwrap(),
            b"abc"
        );
    }
}
