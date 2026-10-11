use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::{
    AppError, AppErrorCode, AsrModelId, AudioFormat, CloseBehavior, GpuMode, SettingsDocument,
    SettingsPatch, SettingsSnapshot, SettingsValues, ThemePreference, TranscriptFormat,
    TranscriptionLanguage, VideoQualityId,
};

const SETTINGS_SCHEMA_VERSION: u16 = 2;
const SETTINGS_INVALID_VALUE: &str = "SETTINGS_INVALID_VALUE";
const SETTINGS_PATH_INVALID: &str = "SETTINGS_PATH_INVALID";
const SETTINGS_STORE_UNAVAILABLE: &str = "SETTINGS_STORE_UNAVAILABLE";

pub trait SettingsStorePort: Send + Sync {
    fn load_raw(&self) -> Result<Option<Value>, AppError>;
    fn save_document(&self, document: &SettingsDocument) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct SettingsDefaults {
    pub download_directory: String,
    pub temporary_directory: String,
    pub asr_model_directory: String,
    legacy_download_directory: Option<String>,
    legacy_temporary_directory: Option<String>,
    legacy_asr_model_directory: Option<String>,
}

impl SettingsDefaults {
    pub fn from_install_directory(install_directory: impl Into<PathBuf>) -> Result<Self, AppError> {
        let install_directory = normalize_path(install_directory.into());
        let mut defaults = Self::from_application_paths(&install_directory, &install_directory)?;
        if let Some(previous_root) = install_directory.parent() {
            defaults.legacy_download_directory = Some(
                normalize_path(previous_root.join("download"))
                    .to_string_lossy()
                    .into_owned(),
            );
            defaults.legacy_temporary_directory = Some(
                normalize_path(previous_root.join("temp"))
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        Ok(defaults)
    }

    pub fn from_application_paths(
        install_directory: impl Into<PathBuf>,
        app_data_directory: impl Into<PathBuf>,
    ) -> Result<Self, AppError> {
        let install_directory = normalize_path(install_directory.into());
        let app_data_directory = normalize_path(app_data_directory.into());
        let previous_model_directory = app_data_directory.join("models").join("whisper");
        let mut defaults = Self::from_directories(
            install_directory.join("download"),
            install_directory.join("temp"),
            install_directory.join("models").join("whisper"),
        )?;
        if !paths_equal(
            &defaults.asr_model_directory,
            &previous_model_directory.to_string_lossy(),
        ) {
            defaults.legacy_asr_model_directory =
                Some(previous_model_directory.to_string_lossy().into_owned());
        }
        Ok(defaults)
    }

    pub fn from_system_paths(
        download_root: impl Into<PathBuf>,
        temporary_directory: impl Into<PathBuf>,
    ) -> Result<Self, AppError> {
        let download_root = download_root.into();
        let app_download_root = download_root.join("BiliCatch");
        Self::from_directories(
            app_download_root.clone(),
            temporary_directory.into(),
            app_download_root.join("models").join("whisper"),
        )
    }

    fn from_directories(
        download_directory: PathBuf,
        temporary_directory: PathBuf,
        asr_model_directory: PathBuf,
    ) -> Result<Self, AppError> {
        let download_directory = normalize_path(download_directory);
        let temporary_directory = normalize_path(temporary_directory);
        let asr_model_directory = normalize_path(asr_model_directory);
        fs::create_dir_all(&download_directory).map_err(|_| store_error())?;
        fs::create_dir_all(&temporary_directory).map_err(|_| store_error())?;
        fs::create_dir_all(&asr_model_directory).map_err(|_| store_error())?;
        validate_directory(&download_directory)?;
        validate_directory(&temporary_directory)?;
        validate_directory(&asr_model_directory)?;

        Ok(Self {
            download_directory: download_directory.to_string_lossy().into_owned(),
            temporary_directory: temporary_directory.to_string_lossy().into_owned(),
            asr_model_directory: asr_model_directory.to_string_lossy().into_owned(),
            legacy_download_directory: None,
            legacy_temporary_directory: None,
            legacy_asr_model_directory: None,
        })
    }

    fn values(&self) -> SettingsValues {
        SettingsValues {
            download_directory: self.download_directory.clone(),
            temporary_directory: self.temporary_directory.clone(),
            max_concurrent_tasks: 3,
            connections_per_task: 8,
            default_video_quality: VideoQualityId::P1080,
            default_audio_format: AudioFormat::Mp3,
            theme: ThemePreference::System,
            locale: "zh-CN".into(),
            notify_on_complete: true,
            completion_sound: true,
            close_behavior: CloseBehavior::MinimizeToTray,
            auto_check_updates: true,
            default_asr_model: AsrModelId::Base,
            default_transcription_language: TranscriptionLanguage::Zh,
            default_transcript_format: TranscriptFormat::Srt,
            transcription_timestamps: true,
            transcription_punctuation: true,
            prefer_platform_subtitles: true,
            auto_transcribe_downloads: false,
            max_concurrent_transcriptions: 1,
            asr_model_directory: self.asr_model_directory.clone(),
            asr_gpu_mode: GpuMode::Auto,
            asr_model_source: String::new(),
        }
    }
}

pub struct SettingsManager {
    store: Arc<dyn SettingsStorePort>,
    document: Mutex<SettingsDocument>,
}

impl std::fmt::Debug for SettingsManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SettingsManager")
            .finish_non_exhaustive()
    }
}

impl SettingsManager {
    pub fn load(
        store: Arc<dyn SettingsStorePort>,
        defaults: SettingsDefaults,
    ) -> Result<Self, AppError> {
        let raw = store.load_raw().map_err(|_| store_error())?;
        let (document, should_persist) = match raw {
            None => (
                SettingsDocument {
                    schema_version: SETTINGS_SCHEMA_VERSION,
                    revision: 0,
                    values: defaults.values(),
                },
                true,
            ),
            Some(raw) => migrate_document(&raw, &defaults)?,
        };

        if should_persist {
            store.save_document(&document).map_err(|_| store_error())?;
        }

        Ok(Self {
            store,
            document: Mutex::new(document),
        })
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        self.document
            .lock()
            .expect("settings document lock poisoned")
            .clone()
    }

    pub fn scheduler_limits(&self) -> (u8, u8) {
        let document = self
            .document
            .lock()
            .expect("settings document lock poisoned");
        (
            document.values.max_concurrent_tasks,
            document.values.connections_per_task,
        )
    }

    pub fn update(&self, patch: SettingsPatch) -> Result<SettingsSnapshot, AppError> {
        let mut current = self.document.lock().map_err(|_| store_error())?;
        let mut candidate = current.clone();
        apply_patch(&mut candidate.values, patch);

        if candidate.values == current.values {
            return Ok(current.clone());
        }

        validate_values(&candidate.values)?;
        candidate.revision = current
            .revision
            .checked_add(1)
            .ok_or_else(invalid_value_error)?;
        self.store
            .save_document(&candidate)
            .map_err(|_| store_error())?;
        *current = candidate.clone();
        Ok(candidate)
    }
}

fn migrate_document(
    raw: &Value,
    defaults: &SettingsDefaults,
) -> Result<(SettingsDocument, bool), AppError> {
    let object = raw.as_object().ok_or_else(store_error)?;
    let schema_version = object
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(SETTINGS_SCHEMA_VERSION as u64);
    if !matches!(schema_version, 1 | 2) {
        return Err(store_error());
    }

    if schema_version == SETTINGS_SCHEMA_VERSION as u64 {
        let mut document: SettingsDocument =
            serde_json::from_value(raw.clone()).map_err(|_| store_error())?;
        let model_directory = migrated_persisted_directory(
            Some(&Value::String(document.values.asr_model_directory.clone())),
            &defaults.asr_model_directory,
            defaults.legacy_asr_model_directory.as_deref(),
        );
        let should_persist = model_directory != document.values.asr_model_directory;
        document.values.asr_model_directory = model_directory;
        validate_values(&document.values)?;
        return Ok((document, should_persist));
    }

    let values = object
        .get("values")
        .and_then(Value::as_object)
        .ok_or_else(store_error)?;
    let fallback = defaults.values();
    let document = SettingsDocument {
        schema_version: SETTINGS_SCHEMA_VERSION,
        revision: object.get("revision").and_then(Value::as_u64).unwrap_or(0),
        values: SettingsValues {
            download_directory: migrated_persisted_directory(
                values.get("downloadDirectory"),
                &fallback.download_directory,
                defaults.legacy_download_directory.as_deref(),
            ),
            temporary_directory: migrated_persisted_directory(
                values.get("temporaryDirectory"),
                &fallback.temporary_directory,
                defaults.legacy_temporary_directory.as_deref(),
            ),
            max_concurrent_tasks: bounded_u8(
                values.get("maxConcurrentTasks"),
                1,
                10,
                fallback.max_concurrent_tasks,
            ),
            connections_per_task: bounded_u8(
                values.get("connectionsPerTask"),
                1,
                32,
                fallback.connections_per_task,
            ),
            default_video_quality: parse_or(
                values.get("defaultVideoQuality"),
                fallback.default_video_quality,
            ),
            default_audio_format: parse_or(
                values.get("defaultAudioFormat"),
                fallback.default_audio_format,
            ),
            theme: parse_or(values.get("theme"), fallback.theme),
            locale: values
                .get("locale")
                .and_then(Value::as_str)
                .filter(|locale| matches!(*locale, "zh-CN" | "en-US"))
                .unwrap_or(&fallback.locale)
                .to_owned(),
            notify_on_complete: values
                .get("notifyOnComplete")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.notify_on_complete),
            completion_sound: values
                .get("completionSound")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.completion_sound),
            close_behavior: parse_or(values.get("closeBehavior"), fallback.close_behavior),
            auto_check_updates: values
                .get("autoCheckUpdates")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.auto_check_updates),
            default_asr_model: parse_or(values.get("defaultAsrModel"), fallback.default_asr_model),
            default_transcription_language: parse_or(
                values.get("defaultTranscriptionLanguage"),
                fallback.default_transcription_language,
            ),
            default_transcript_format: parse_or(
                values.get("defaultTranscriptFormat"),
                fallback.default_transcript_format,
            ),
            transcription_timestamps: values
                .get("transcriptionTimestamps")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.transcription_timestamps),
            transcription_punctuation: values
                .get("transcriptionPunctuation")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.transcription_punctuation),
            prefer_platform_subtitles: values
                .get("preferPlatformSubtitles")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.prefer_platform_subtitles),
            auto_transcribe_downloads: values
                .get("autoTranscribeDownloads")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.auto_transcribe_downloads),
            max_concurrent_transcriptions: bounded_u8(
                values.get("maxConcurrentTranscriptions"),
                1,
                3,
                fallback.max_concurrent_transcriptions,
            ),
            asr_model_directory: migrated_persisted_directory(
                values.get("asrModelDirectory"),
                &fallback.asr_model_directory,
                defaults.legacy_asr_model_directory.as_deref(),
            ),
            asr_gpu_mode: parse_or(values.get("asrGpuMode"), fallback.asr_gpu_mode),
            asr_model_source: normalized_model_source(
                values.get("asrModelSource").and_then(Value::as_str),
            )
            .unwrap_or_else(|| fallback.asr_model_source.clone()),
        },
    };
    let normalized = serde_json::to_value(&document).map_err(|_| store_error())?;
    Ok((document, normalized != *raw))
}

fn parse_or<T: DeserializeOwned>(value: Option<&Value>, fallback: T) -> T {
    value
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or(fallback)
}

fn bounded_u8(value: Option<&Value>, min: u8, max: u8, fallback: u8) -> u8 {
    value
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| (*value >= min) && (*value <= max))
        .unwrap_or(fallback)
}

fn valid_persisted_directory(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .filter(|value| validate_directory(Path::new(value)).is_ok())
        .unwrap_or(fallback)
        .to_owned()
}

fn migrated_persisted_directory(
    value: Option<&Value>,
    fallback: &str,
    legacy_default: Option<&str>,
) -> String {
    let Some(value) = value.and_then(Value::as_str) else {
        return fallback.to_owned();
    };
    if legacy_default.is_some_and(|legacy| paths_equal(value, legacy)) {
        return fallback.to_owned();
    }
    valid_persisted_directory(Some(&Value::String(value.to_owned())), fallback)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let value = path.to_string_lossy();
    let value = value
        .strip_prefix(r"\\?\")
        .or_else(|| value.strip_prefix("//?/"))
        .unwrap_or(&value);
    PathBuf::from(value)
}

fn paths_equal(left: &str, right: &str) -> bool {
    let left = normalize_path(PathBuf::from(left));
    let right = normalize_path(PathBuf::from(right));
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn apply_patch(values: &mut SettingsValues, patch: SettingsPatch) {
    match patch {
        SettingsPatch::DownloadDirectory(value) => values.download_directory = value,
        SettingsPatch::TemporaryDirectory(value) => values.temporary_directory = value,
        SettingsPatch::MaxConcurrentTasks(value) => values.max_concurrent_tasks = value,
        SettingsPatch::ConnectionsPerTask(value) => values.connections_per_task = value,
        SettingsPatch::DefaultVideoQuality(value) => values.default_video_quality = value,
        SettingsPatch::DefaultAudioFormat(value) => values.default_audio_format = value,
        SettingsPatch::Theme(value) => values.theme = value,
        SettingsPatch::Locale(value) => values.locale = value,
        SettingsPatch::NotifyOnComplete(value) => values.notify_on_complete = value,
        SettingsPatch::CompletionSound(value) => values.completion_sound = value,
        SettingsPatch::CloseBehavior(value) => values.close_behavior = value,
        SettingsPatch::AutoCheckUpdates(value) => values.auto_check_updates = value,
        SettingsPatch::DefaultAsrModel(value) => values.default_asr_model = value,
        SettingsPatch::DefaultTranscriptionLanguage(value) => {
            values.default_transcription_language = value
        }
        SettingsPatch::DefaultTranscriptFormat(value) => values.default_transcript_format = value,
        SettingsPatch::TranscriptionTimestamps(value) => values.transcription_timestamps = value,
        SettingsPatch::TranscriptionPunctuation(value) => values.transcription_punctuation = value,
        SettingsPatch::PreferPlatformSubtitles(value) => values.prefer_platform_subtitles = value,
        SettingsPatch::AutoTranscribeDownloads(value) => values.auto_transcribe_downloads = value,
        SettingsPatch::MaxConcurrentTranscriptions(value) => {
            values.max_concurrent_transcriptions = value
        }
        SettingsPatch::AsrModelDirectory(value) => values.asr_model_directory = value.trim().into(),
        SettingsPatch::AsrGpuMode(value) => values.asr_gpu_mode = value,
        SettingsPatch::AsrModelSource(value) => values.asr_model_source = value.trim().into(),
    }
}

fn validate_values(values: &SettingsValues) -> Result<(), AppError> {
    if !(1..=10).contains(&values.max_concurrent_tasks)
        || !(1..=32).contains(&values.connections_per_task)
        || !(1..=3).contains(&values.max_concurrent_transcriptions)
        || !matches!(values.locale.as_str(), "zh-CN" | "en-US")
        || normalized_model_source(Some(&values.asr_model_source)).as_deref()
            != Some(values.asr_model_source.as_str())
    {
        return Err(invalid_value_error());
    }
    validate_directory(Path::new(&values.download_directory))?;
    validate_directory(Path::new(&values.temporary_directory))?;
    validate_directory(Path::new(&values.asr_model_directory))?;
    Ok(())
}

fn normalized_model_source(value: Option<&str>) -> Option<String> {
    let value = value.unwrap_or_default().trim();
    if value.is_empty() {
        return Some(String::new());
    }
    let url = url::Url::parse(value).ok()?;
    if url.scheme() != "https" || url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    Some(value.to_owned())
}

fn validate_directory(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() || !path.is_dir() {
        return Err(path_error());
    }
    Ok(())
}

fn error_with_details(message: &str, details: &str) -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: message.into(),
        details: Some(details.into()),
    }
}

fn invalid_value_error() -> AppError {
    error_with_details("The setting value is invalid", SETTINGS_INVALID_VALUE)
}

fn path_error() -> AppError {
    error_with_details("The selected directory is invalid", SETTINGS_PATH_INVALID)
}

fn store_error() -> AppError {
    error_with_details(
        "Settings storage is unavailable",
        SETTINGS_STORE_UNAVAILABLE,
    )
}
