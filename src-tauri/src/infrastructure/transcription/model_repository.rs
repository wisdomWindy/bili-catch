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

use crate::{
    infrastructure::transcription::openasr_sidecar::{
        pack_id_from_manifest, pack_state, InstalledPack, PackState,
    },
    models::{AppError, AppErrorCode, AsrModel, AsrModelId, ModelDownloadState},
};

const DEFAULT_MODEL_MIRROR: &str = "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelManifestEntry {
    pub id: AsrModelId,
    pub display_name: String,
    /// 主文件名，用于展示与旧接口兼容；多文件模型取 `files[0].name`。
    pub file_name: String,
    /// 整个模型包的总字节数。
    pub size_bytes: u64,
    /// 主文件的 sha256，用于旧接口兼容；校验以 `files` 为准。
    pub sha256: String,
    pub official_url: String,
    pub recommended_for: String,
    pub capabilities: Vec<String>,
    /// 多文件模型的安装目录名；单文件模型为 `None`，直接落在模型根目录。
    pub directory: Option<String>,
    pub files: Vec<ModelFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    pub name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub url: String,
    pub mirror_url: Option<String>,
}

impl ModelManifestEntry {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|file| file.size_bytes).sum()
    }

    pub fn is_package(&self) -> bool {
        self.directory.is_some()
    }
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
        ModelManifestEntry {
            id: AsrModelId::FireRedAsr2,
            display_name: "FireRedASR2 CTC (INT8)".into(),
            file_name: "fire-red-asr2-ctc-int8.onnx".into(),
            size_bytes: 775_861_420,
            sha256: "ca3dbabd82170110cc0b343c2890866d449984bc9cd92b9a18371ff80a81bb99"
                .into(),
            official_url: "https://huggingface.co/csukuangfj2/sherpa-onnx-fire-red-asr2-ctc-zh_en-int8-2026-02-25/resolve/main/model.int8.onnx".into(),
            recommended_for: "High-accuracy Chinese, English, dialect, and singing transcription"
                .into(),
            capabilities: vec![
                "zh-en".into(),
                "dialects".into(),
                "singing".into(),
                "timestamps".into(),
                "cpu".into(),
            ],
            directory: None,
            files: vec![ModelFile {
                name: "fire-red-asr2-ctc-int8.onnx".into(),
                size_bytes: 775_861_420,
                sha256: "ca3dbabd82170110cc0b343c2890866d449984bc9cd92b9a18371ff80a81bb99"
                    .into(),
                url: "https://huggingface.co/csukuangfj2/sherpa-onnx-fire-red-asr2-ctc-zh_en-int8-2026-02-25/resolve/main/model.int8.onnx".into(),
                mirror_url: Some(
                    "https://hf-mirror.com/csukuangfj2/sherpa-onnx-fire-red-asr2-ctc-zh_en-int8-2026-02-25/resolve/main/model.int8.onnx".into(),
                ),
            }],
        },
        ModelManifestEntry {
            id: AsrModelId::FireRedAsr2Aed,
            display_name: "FireRedASR2 AED (INT8)".into(),
            file_name: "encoder.int8.onnx".into(),
            size_bytes: 1_234_578_761,
            sha256: "54048d66b6e8f3c80ea7ce95efe794587b0fd81d7271651d0decd3803852ae82"
                .into(),
            official_url: "https://huggingface.co/csukuangfj2/sherpa-onnx-fire-red-asr2-zh_en-int8-2026-02-26/resolve/main/encoder.int8.onnx".into(),
            recommended_for: "Best Chinese accuracy with word-level timestamps and confidence"
                .into(),
            capabilities: vec![
                "zh-en".into(),
                "dialects".into(),
                "singing".into(),
                "timestamps".into(),
                "confidence".into(),
                "cpu".into(),
            ],
            directory: Some("fire-red-asr2-aed".into()),
            files: vec![
                ModelFile {
                    name: "encoder.int8.onnx".into(),
                    size_bytes: 817_286_833,
                    sha256: "54048d66b6e8f3c80ea7ce95efe794587b0fd81d7271651d0decd3803852ae82"
                        .into(),
                    url: "https://huggingface.co/csukuangfj2/sherpa-onnx-fire-red-asr2-zh_en-int8-2026-02-26/resolve/main/encoder.int8.onnx".into(),
                    mirror_url: Some(
                        "https://hf-mirror.com/csukuangfj2/sherpa-onnx-fire-red-asr2-zh_en-int8-2026-02-26/resolve/main/encoder.int8.onnx".into(),
                    ),
                },
                ModelFile {
                    name: "decoder.int8.onnx".into(),
                    size_bytes: 417_291_928,
                    sha256: "b840ce7196ae4a14d05ae84bbf56082b6b61ccec5610fda907dddbcea37354ff"
                        .into(),
                    url: "https://huggingface.co/csukuangfj2/sherpa-onnx-fire-red-asr2-zh_en-int8-2026-02-26/resolve/main/decoder.int8.onnx".into(),
                    mirror_url: Some(
                        "https://hf-mirror.com/csukuangfj2/sherpa-onnx-fire-red-asr2-zh_en-int8-2026-02-26/resolve/main/decoder.int8.onnx".into(),
                    ),
                },
            ],
        },
        ModelManifestEntry {
            id: AsrModelId::FireRedAsr2Llm,
            display_name: "FireRedASR2 LLM 8B (OpenASR)".into(),
            file_name: "openasr:firered2-llm".into(),
            size_bytes: 0,
            sha256: String::new(),
            official_url: String::new(),
            recommended_for: "Highest Chinese accuracy via an external OpenASR runtime (needs 16 GB RAM)"
                .into(),
            capabilities: vec![
                "zh-en".into(),
                "dialects".into(),
                "high-accuracy".into(),
                "external-tool".into(),
            ],
            directory: Some("openasr".into()),
            // 空文件列表表示该模型由外部工具（OpenASR）管理，本仓库不负责下载与校验。
            files: Vec::new(),
        },
        ModelManifestEntry {
            id: AsrModelId::Qwen3Asr,
            display_name: "Qwen3-ASR 0.6B (INT8)".into(),
            file_name: "conv_frontend.onnx".into(),
            size_bytes: 987_015_347,
            sha256: "d22dc4423e0940e49884e903d2ea2f7e5567c14fc1aed97e4e26d6b8f208ef9e"
                .into(),
            official_url: "https://huggingface.co/csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/resolve/main/conv_frontend.onnx".into(),
            recommended_for: "Japanese, Korean, and 50+ languages with built-in punctuation"
                .into(),
            capabilities: vec![
                "multilingual".into(),
                "ja-ko".into(),
                "punctuation".into(),
                "cpu".into(),
            ],
            directory: Some("qwen3-asr".into()),
            files: vec![
                qwen3_file(
                    "conv_frontend.onnx",
                    "conv_frontend.onnx",
                    44_148_281,
                    "d22dc4423e0940e49884e903d2ea2f7e5567c14fc1aed97e4e26d6b8f208ef9e",
                ),
                qwen3_file(
                    "encoder.int8.onnx",
                    "encoder.int8.onnx",
                    182_491_662,
                    "60748d3e6744a57c9c91e1b17424a6c2990567e8adceb0783940c03ed98fa9d9",
                ),
                qwen3_file(
                    "decoder.int8.onnx",
                    "decoder.int8.onnx",
                    755_914_231,
                    "4f6885be5959ae26af3089d38ee7972c5fafbeeb1cf8d5e76eab6d8b61ca5771",
                ),
                qwen3_file(
                    "vocab.json",
                    "tokenizer/vocab.json",
                    2_776_833,
                    "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
                ),
                qwen3_file(
                    "merges.txt",
                    "tokenizer/merges.txt",
                    1_671_853,
                    "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
                ),
                qwen3_file(
                    "tokenizer_config.json",
                    "tokenizer/tokenizer_config.json",
                    12_487,
                    "4942d005604266809309cabc9f4e9cb89ce855d59b14681fdc0e1cc62ea26c4c",
                ),
            ],
        },
    ]
}

fn qwen3_file(
    name: &str,
    remote_path: &str,
    size_bytes: u64,
    sha256: &str,
) -> ModelFile {
    const REPO: &str = "csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25";
    ModelFile {
        name: name.into(),
        size_bytes,
        sha256: sha256.into(),
        url: format!("https://huggingface.co/{REPO}/resolve/main/{remote_path}"),
        mirror_url: Some(format!("https://hf-mirror.com/{REPO}/resolve/main/{remote_path}")),
    }
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
    let mirror_url = format!("{DEFAULT_MODEL_MIRROR}{file_name}");
    ModelManifestEntry {
        id,
        display_name: display_name.into(),
        file_name: file_name.into(),
        size_bytes,
        sha256: sha256.into(),
        official_url: format!("{base_url}/{file_name}"),
        recommended_for: recommended_for.into(),
        capabilities: vec!["multilingual".into(), "timestamps".into()],
        directory: None,
        files: vec![ModelFile {
            name: file_name.into(),
            size_bytes,
            sha256: sha256.into(),
            url: format!("{base_url}/{file_name}"),
            mirror_url: Some(mirror_url),
        }],
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
    file: ModelFile,
    part_path: PathBuf,
    final_path: PathBuf,
    handle: Option<File>,
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
            .filter(|size| *size <= self.file.size_bytes)
            .ok_or_else(|| download_error("MODEL_SIZE_MISMATCH"))?;
        self.handle
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
        if self.downloaded_bytes != self.file.size_bytes {
            return Err(download_error("MODEL_SIZE_MISMATCH"));
        }
        if let Some(handle) = self.handle.take() {
            handle
                .sync_all()
                .map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        }
        if !file_matches(&self.part_path, &self.file)? {
            let _ = fs::remove_file(&self.part_path);
            return Err(integrity_error("MODEL_CHECKSUM_MISMATCH"));
        }
        if let Some(parent) = self.final_path.parent() {
            fs::create_dir_all(parent).map_err(|_| download_error("MODEL_STORAGE_UNAVAILABLE"))?;
        }
        fs::rename(&self.part_path, &self.final_path).map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        self.committed = true;
        let entry = self.repository.entry(self.model_id)?.clone();
        self.repository.model_state(&entry, self.default_model)
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
        mut on_progress: F,
    ) -> Result<AsrModel, AppError>
    where
        F: FnMut(u64, u64),
    {
        let entry = repository.entry(model_id)?.clone();
        let total = entry.total_size();
        let mut completed = 0_u64;
        for file in entry.files.iter() {
            let urls = repository.file_urls(model_id, &file.name, custom_source)?;
            self.download_one(
                repository,
                model_id,
                default_model,
                &file.name,
                urls,
                total,
                &mut completed,
                cancellation.clone(),
                &mut on_progress,
            )
            .await?;
        }
        repository.model_state(&entry, default_model)
    }

    #[cfg(test)]
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
        let entry = repository.entry(model_id)?.clone();
        let total = entry.total_size();
        let file_name = entry
            .files
            .first()
            .map(|file| file.name.clone())
            .ok_or_else(|| download_error("MODEL_UNKNOWN"))?;
        let mut completed = 0_u64;
        self.download_one(
            repository,
            model_id,
            default_model,
            &file_name,
            urls,
            total,
            &mut completed,
            cancellation,
            &mut on_progress,
        )
        .await?;
        repository.model_state(&entry, default_model)
    }

    async fn download_one<F>(
        &self,
        repository: &FileModelRepository,
        model_id: AsrModelId,
        default_model: AsrModelId,
        file_name: &str,
        urls: Vec<Url>,
        total: u64,
        completed: &mut u64,
        cancellation: ModelDownloadCancellation,
        on_progress: &mut F,
    ) -> Result<(), AppError>
    where
        F: FnMut(u64, u64),
    {
        let candidate_count = urls.len();
        let base = *completed;
        for (index, url) in urls.into_iter().enumerate() {
            *completed = base;
            let result = self
                .stream_url(
                    repository,
                    model_id,
                    default_model,
                    file_name,
                    url,
                    total,
                    completed,
                    cancellation.clone(),
                    on_progress,
                )
                .await;
            match result {
                Ok(()) => return Ok(()),
                Err(error)
                    if index + 1 < candidate_count
                        && !cancellation.is_cancelled()
                        && retryable_source_error(&error) => {}
                Err(error) => return Err(error),
            }
        }
        Err(download_error("MODEL_REQUEST_FAILED"))
    }

    async fn stream_url<F>(
        &self,
        repository: &FileModelRepository,
        model_id: AsrModelId,
        default_model: AsrModelId,
        file_name: &str,
        url: Url,
        total: u64,
        completed: &mut u64,
        cancellation: ModelDownloadCancellation,
        on_progress: &mut F,
    ) -> Result<(), AppError>
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
        let entry = repository.entry(model_id)?.clone();
        let file = entry
            .files
            .iter()
            .find(|candidate| candidate.name == file_name)
            .cloned()
            .ok_or_else(|| download_error("MODEL_UNKNOWN"))?;
        if response
            .content_length()
            .is_some_and(|content_length| content_length != file.size_bytes)
        {
            return Err(download_error("MODEL_SIZE_MISMATCH"));
        }
        let mut writer =
            repository.begin_file_download(model_id, file_name, default_model, cancellation)?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| download_error("MODEL_STREAM_FAILED"))?
        {
            let downloaded = writer.append(&chunk)?;
            on_progress(*completed + downloaded, total);
        }
        writer.finish()?;
        *completed += file.size_bytes;
        on_progress(*completed, total);
        Ok(())
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
        let file_name = entry
            .files
            .first()
            .map(|file| file.name.clone())
            .ok_or_else(|| integrity_error("MODEL_UNKNOWN"))?;
        self.begin_file_download(model_id, &file_name, default_model, cancellation)
    }

    pub fn begin_file_download(
        &self,
        model_id: AsrModelId,
        file_name: &str,
        default_model: AsrModelId,
        cancellation: ModelDownloadCancellation,
    ) -> Result<ModelDownloadWriter, AppError> {
        let entry = self.entry(model_id)?;
        let file = entry
            .files
            .iter()
            .find(|candidate| candidate.name == file_name)
            .cloned()
            .ok_or_else(|| integrity_error("MODEL_UNKNOWN"))?;
        let directory = self.package_root(entry);
        fs::create_dir_all(&directory).map_err(|_| download_error("MODEL_STORAGE_UNAVAILABLE"))?;
        let part_path = self.part_path(entry, &file.name);
        remove_if_exists(&part_path)?;
        let final_path = self.file_path(entry, &file.name);
        let handle =
            File::create(&part_path).map_err(|_| download_error("MODEL_DOWNLOAD_WRITE_FAILED"))?;
        Ok(ModelDownloadWriter {
            repository: self.clone(),
            model_id,
            default_model,
            file,
            part_path,
            final_path,
            handle: Some(handle),
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

    /// 外部工具（OpenASR）托管的模型：BiliCatch 没有可下载的文件，安装交给外部 CLI。
    pub fn is_external_model(&self, model_id: AsrModelId) -> bool {
        self.entry(model_id).is_ok_and(|entry| entry.files.is_empty())
    }

    pub fn verify(
        &self,
        model_id: AsrModelId,
        default_model: AsrModelId,
    ) -> Result<AsrModel, AppError> {
        let entry = self.entry(model_id)?;
        let mut intact = true;
        for file in entry.files.iter() {
            let path = self.file_path(entry, &file.name);
            if !file_matches(&path, file)? {
                intact = false;
            }
            remove_if_exists(&self.part_path(entry, &file.name))?;
        }
        if !intact {
            self.remove_artifacts(entry)?;
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
        let file = entry
            .files
            .first()
            .cloned()
            .ok_or_else(|| integrity_error("MODEL_UNKNOWN"))?;
        let expected_part = self.part_path(entry, &file.name);
        if part_path != expected_part || !file_matches(part_path, &file)? {
            remove_if_exists(part_path)?;
            return Err(integrity_error("MODEL_CHECKSUM_MISMATCH"));
        }
        let final_path = self.file_path(entry, &file.name);
        if final_path.exists() {
            return Err(integrity_error("MODEL_ALREADY_EXISTS"));
        }
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(model_io_error)?;
        }
        fs::rename(part_path, &final_path).map_err(model_io_error)?;
        self.model_state(entry, default_model)
    }

    pub fn delete(&self, model_id: AsrModelId, in_use: bool) -> Result<(), AppError> {
        if in_use {
            return Err(integrity_error("MODEL_IN_USE"));
        }
        let entry = self.entry(model_id)?;
        self.remove_artifacts(entry)
    }

    pub fn download_url(&self, model_id: AsrModelId, custom_source: &str) -> Result<Url, AppError> {
        let entry = self.entry(model_id)?;
        let file_name = entry
            .files
            .first()
            .map(|file| file.name.clone())
            .ok_or_else(|| download_error("MODEL_URL_INVALID"))?;
        self.file_urls(model_id, &file_name, custom_source)?
            .into_iter()
            .next()
            .ok_or_else(|| download_error("MODEL_URL_INVALID"))
    }

    fn file_urls(
        &self,
        model_id: AsrModelId,
        file_name: &str,
        custom_source: &str,
    ) -> Result<Vec<Url>, AppError> {
        if !custom_source.trim().is_empty() {
            let source = Url::parse(custom_source.trim())
                .map_err(|_| download_error("MODEL_SOURCE_INVALID"))?;
            if source.scheme() != "https"
                || source.query().is_some()
                || source.fragment().is_some()
            {
                return Err(download_error("MODEL_SOURCE_INVALID"));
            }
            let url = source
                .join(file_name)
                .map_err(|_| download_error("MODEL_SOURCE_INVALID"))?;
            return Ok(vec![url]);
        }
        let entry = self.entry(model_id)?;
        let file = entry
            .files
            .iter()
            .find(|candidate| candidate.name == file_name)
            .ok_or_else(|| download_error("MODEL_URL_INVALID"))?;
        let mut urls = vec![Url::parse(&file.url).map_err(|_| download_error("MODEL_URL_INVALID"))?];
        if let Some(mirror) = &file.mirror_url {
            urls.push(Url::parse(mirror).map_err(|_| download_error("MODEL_URL_INVALID"))?);
        }
        Ok(urls)
    }

    pub fn part_path_for(&self, model_id: AsrModelId) -> Result<PathBuf, AppError> {
        let entry = self.entry(model_id)?;
        let file = entry
            .files
            .first()
            .ok_or_else(|| integrity_error("MODEL_UNKNOWN"))?;
        Ok(self.part_path(entry, &file.name))
    }

    fn remove_artifacts(&self, entry: &ModelManifestEntry) -> Result<(), AppError> {
        for file in entry.files.iter() {
            remove_if_exists(&self.file_path(entry, &file.name))?;
            remove_if_exists(&self.part_path(entry, &file.name))?;
        }
        if entry.is_package() {
            let root = self.package_root(entry);
            if root.exists() {
                fs::remove_dir_all(&root).map_err(model_io_error)?;
            }
        }
        Ok(())
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
        // 空文件列表 = 外部工具托管（OpenASR），本仓库不做下载与校验。
        if entry.files.is_empty() {
            let pack =
                pack_id_from_manifest(&entry.file_name).map_or(PackState::Unknown, pack_state);
            return Ok(self.external_model_state(entry, default_model, pack));
        }
        let paths = entry
            .files
            .iter()
            .map(|file| self.file_path(entry, &file.name))
            .collect::<Vec<_>>();
        let installed = !paths.is_empty() && paths.iter().all(|path| path.is_file());
        let mut valid = installed;
        if installed {
            for (file, path) in entry.files.iter().zip(paths.iter()) {
                if !file_matches(path, file)? {
                    valid = false;
                    break;
                }
            }
        }
        let downloaded_bytes = paths
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum();
        let artifact = self.artifact_path(entry);
        Ok(AsrModel {
            id: entry.id,
            display_name: entry.display_name.clone(),
            file_name: entry.file_name.clone(),
            size_bytes: entry.total_size(),
            installed,
            valid,
            is_default: entry.id == default_model,
            in_use: false,
            path: installed.then(|| artifact.to_string_lossy().into_owned()),
            recommended_for: entry.recommended_for.clone(),
            capabilities: entry.capabilities.clone(),
            download_state: if valid {
                ModelDownloadState::Completed
            } else {
                ModelDownloadState::Idle
            },
            downloaded_bytes,
            error: None,
            revision: 0,
        })
    }

    /// OpenASR 托管的模型：真实安装状态以 OpenASR 自己的模型仓库为准。
    ///
    /// 之前这里把所有外部托管模型都写死成“已安装、可用”，于是界面显示绿色对勾，
    /// 用户点开始转写后必然失败。现在只有 `Missing`（确认没装）才会标成未安装；
    /// `Unknown`（探测不了）继续沿用旧假设，避免把已装好的用户挡在门外。
    fn external_model_state(
        &self,
        entry: &ModelManifestEntry,
        default_model: AsrModelId,
        pack: PackState,
    ) -> AsrModel {
        let base = AsrModel {
            id: entry.id,
            display_name: entry.display_name.clone(),
            file_name: entry.file_name.clone(),
            size_bytes: 0,
            installed: true,
            valid: true,
            is_default: entry.id == default_model,
            in_use: false,
            path: Some(self.package_root(entry).to_string_lossy().into_owned()),
            recommended_for: entry.recommended_for.clone(),
            capabilities: entry.capabilities.clone(),
            download_state: ModelDownloadState::Completed,
            downloaded_bytes: 0,
            error: None,
            revision: 0,
        };
        match pack {
            PackState::Unknown => base,
            PackState::Installed(InstalledPack { path, size_bytes }) => AsrModel {
                size_bytes,
                downloaded_bytes: size_bytes,
                path: Some(path.to_string_lossy().into_owned()),
                ..base
            },
            PackState::Missing => AsrModel {
                installed: false,
                valid: false,
                download_state: ModelDownloadState::Idle,
                path: None,
                downloaded_bytes: 0,
                error: Some(AppError {
                    code: AppErrorCode::E011,
                    message: "The OpenASR model pack is not installed".into(),
                    details: Some("OPENASR_MODEL_MISSING".into()),
                }),
                ..base
            },
        }
    }

    fn package_root(&self, entry: &ModelManifestEntry) -> PathBuf {
        match &entry.directory {
            Some(directory) => self.model_directory.join(directory),
            None => self.model_directory.clone(),
        }
    }

    fn file_path(&self, entry: &ModelManifestEntry, file_name: &str) -> PathBuf {
        self.package_root(entry).join(file_name)
    }

    fn artifact_path(&self, entry: &ModelManifestEntry) -> PathBuf {
        if entry.is_package() {
            self.package_root(entry)
        } else {
            self.file_path(entry, &entry.file_name)
        }
    }

    fn part_path(&self, entry: &ModelManifestEntry, file_name: &str) -> PathBuf {
        self.file_path(entry, file_name)
            .with_file_name(format!("{file_name}.part"))
    }
}

fn file_matches(path: &Path, file: &ModelFile) -> Result<bool, AppError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(model_io_error(error)),
    };
    if !metadata.is_file() || metadata.len() != file.size_bytes {
        return Ok(false);
    }
    let handle = File::open(path).map_err(model_io_error)?;
    let mut reader = BufReader::new(handle);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(model_io_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()) == file.sha256)
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
        official_model_manifest, FileModelRepository, InstalledPack, ModelDownloadCancellation,
        ModelFile, ModelManifestEntry, PackState, ReqwestModelDownloader,
    };
    use crate::models::{AppErrorCode, AsrModelId};

    fn external_manifest_entry() -> ModelManifestEntry {
        ModelManifestEntry {
            id: AsrModelId::FireRedAsr2Llm,
            display_name: "FireRedASR2 LLM 8B (OpenASR)".into(),
            file_name: "openasr:firered2-llm".into(),
            size_bytes: 0,
            sha256: String::new(),
            official_url: String::new(),
            recommended_for: "test".into(),
            capabilities: vec![],
            directory: Some("openasr".into()),
            // 空文件列表 = 由外部工具管理
            files: Vec::new(),
        }
    }

    #[test]
    fn external_model_without_its_pack_is_reported_as_not_installed() {
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(root.path(), vec![external_manifest_entry()]);
        let entry = external_manifest_entry();

        let missing =
            repo.external_model_state(&entry, AsrModelId::FireRedAsr2Llm, PackState::Missing);

        assert!(!missing.installed);
        assert!(!missing.valid);
        assert_eq!(missing.path, None);
        let error = missing.error.expect("missing pack should carry an error");
        assert_eq!(error.code, AppErrorCode::E011);
        assert_eq!(error.details.as_deref(), Some("OPENASR_MODEL_MISSING"));
    }

    #[test]
    fn external_model_keeps_the_optimistic_state_when_the_pack_cannot_be_inspected() {
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(root.path(), vec![external_manifest_entry()]);

        let unknown = repo.external_model_state(
            &external_manifest_entry(),
            AsrModelId::FireRedAsr2Llm,
            PackState::Unknown,
        );

        assert!(unknown.installed);
        assert!(unknown.valid);
        assert!(unknown.error.is_none());
    }

    #[test]
    fn external_model_reports_the_installed_pack_path_and_size() {
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(root.path(), vec![external_manifest_entry()]);
        let pack = root.path().join("content");

        let installed = repo.external_model_state(
            &external_manifest_entry(),
            AsrModelId::FireRedAsr2Llm,
            PackState::Installed(InstalledPack {
                path: pack.clone(),
                size_bytes: 5_097_451_168,
            }),
        );

        assert!(installed.installed);
        assert!(installed.valid);
        assert_eq!(installed.size_bytes, 5_097_451_168);
        assert_eq!(installed.downloaded_bytes, 5_097_451_168);
        assert_eq!(
            installed.path.as_deref(),
            Some(pack.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn only_external_entries_are_refused_by_the_downloader() {
        let root = tempfile::tempdir().expect("model directory");
        let external =
            FileModelRepository::with_manifest(root.path(), vec![external_manifest_entry()]);
        assert!(external.is_external_model(AsrModelId::FireRedAsr2Llm));

        let local = FileModelRepository::with_manifest(root.path(), official_model_manifest());
        assert!(!local.is_external_model(AsrModelId::Base));
    }

    #[test]
    fn official_manifest_includes_the_verified_fire_red_asr2_int8_model() {
        let entry = official_model_manifest()
            .into_iter()
            .find(|entry| entry.display_name == "FireRedASR2 CTC (INT8)")
            .expect("FireRedASR2 manifest entry");

        assert_eq!(entry.file_name, "fire-red-asr2-ctc-int8.onnx");
        assert_eq!(entry.size_bytes, 775_861_420);
        assert_eq!(
            entry.sha256,
            "ca3dbabd82170110cc0b343c2890866d449984bc9cd92b9a18371ff80a81bb99"
        );
        assert!(entry.official_url.starts_with("https://huggingface.co/"));
        assert_eq!(
            entry.capabilities,
            vec!["zh-en", "dialects", "singing", "timestamps", "cpu"]
        );
    }

    #[test]
    fn package_models_install_into_a_subdirectory_and_delete_with_it() {
        const SHA: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let root = tempfile::tempdir().expect("model directory");
        let repo = FileModelRepository::with_manifest(
            root.path(),
            vec![ModelManifestEntry {
                id: AsrModelId::Tiny,
                display_name: "Package".into(),
                file_name: "a.onnx".into(),
                size_bytes: 6,
                sha256: SHA.into(),
                official_url: "https://example.invalid/a.onnx".into(),
                recommended_for: "test".into(),
                capabilities: vec![],
                directory: Some("pkg".into()),
                files: vec![
                    ModelFile {
                        name: "a.onnx".into(),
                        size_bytes: 3,
                        sha256: SHA.into(),
                        url: "https://example.invalid/a.onnx".into(),
                        mirror_url: None,
                    },
                    ModelFile {
                        name: "b.onnx".into(),
                        size_bytes: 3,
                        sha256: SHA.into(),
                        url: "https://example.invalid/b.onnx".into(),
                        mirror_url: None,
                    },
                ],
            }],
        );
        let package = root.path().join("pkg");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("a.onnx"), b"abc").unwrap();

        let models = repo.list(AsrModelId::Tiny).expect("scan");
        assert!(!models[0].installed);
        assert_eq!(models[0].size_bytes, 6);

        std::fs::write(package.join("b.onnx"), b"abc").unwrap();
        let models = repo.list(AsrModelId::Tiny).expect("scan");
        assert!(models[0].installed);
        assert!(models[0].valid);
        assert_eq!(
            models[0].path.as_deref(),
            Some(package.to_string_lossy().as_ref())
        );

        repo.delete(AsrModelId::Tiny, false)
            .expect("delete package");
        assert!(!package.exists());
    }

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
                directory: None,
                files: vec![ModelFile {
                    name: "ggml-tiny.bin".into(),
                    size_bytes: 3,
                    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .into(),
                    url: format!("http://{address}/ggml-tiny.bin"),
                    mirror_url: None,
                }],
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
                directory: None,
                files: vec![ModelFile {
                    name: "ggml-tiny.bin".into(),
                    size_bytes: 3,
                    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .into(),
                    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
                        .into(),
                    mirror_url: Some(
                        "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
                            .into(),
                    ),
                }],
            }],
        );

        let urls = repo
            .file_urls(AsrModelId::Tiny, "ggml-tiny.bin", "")
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
                directory: None,
                files: vec![ModelFile {
                    name: "ggml-tiny.bin".into(),
                    size_bytes: 3,
                    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .into(),
                    url: format!("http://{primary_address}/ggml-tiny.bin"),
                    mirror_url: None,
                }],
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
