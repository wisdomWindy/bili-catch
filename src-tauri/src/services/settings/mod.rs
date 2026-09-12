use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::{
    AppError, AppErrorCode, AudioFormat, CloseBehavior, SettingsDocument, SettingsPatch,
    SettingsSnapshot, SettingsValues, ThemePreference, VideoQualityId,
};

const SETTINGS_SCHEMA_VERSION: u16 = 1;
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
}

impl SettingsDefaults {
    pub fn from_install_directory(install_directory: impl Into<PathBuf>) -> Result<Self, AppError> {
        let install_directory = install_directory.into();
        Self::from_directories(
            install_directory.join("download"),
            install_directory.join("temp"),
        )
    }

    pub fn from_system_paths(
        download_root: impl Into<PathBuf>,
        temporary_directory: impl Into<PathBuf>,
    ) -> Result<Self, AppError> {
        Self::from_directories(
            download_root.into().join("BiliCatch"),
            temporary_directory.into(),
        )
    }

    fn from_directories(
        download_directory: PathBuf,
        temporary_directory: PathBuf,
    ) -> Result<Self, AppError> {
        fs::create_dir_all(&download_directory).map_err(|_| store_error())?;
        fs::create_dir_all(&temporary_directory).map_err(|_| store_error())?;
        validate_directory(&download_directory)?;
        validate_directory(&temporary_directory)?;

        Ok(Self {
            download_directory: download_directory.to_string_lossy().into_owned(),
            temporary_directory: temporary_directory.to_string_lossy().into_owned(),
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
            close_behavior: CloseBehavior::MinimizeToTray,
            auto_check_updates: true,
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
    if schema_version != SETTINGS_SCHEMA_VERSION as u64 {
        return Err(store_error());
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
            download_directory: valid_persisted_directory(
                values.get("downloadDirectory"),
                &fallback.download_directory,
            ),
            temporary_directory: valid_persisted_directory(
                values.get("temporaryDirectory"),
                &fallback.temporary_directory,
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
            close_behavior: parse_or(values.get("closeBehavior"), fallback.close_behavior),
            auto_check_updates: values
                .get("autoCheckUpdates")
                .and_then(Value::as_bool)
                .unwrap_or(fallback.auto_check_updates),
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
        SettingsPatch::CloseBehavior(value) => values.close_behavior = value,
        SettingsPatch::AutoCheckUpdates(value) => values.auto_check_updates = value,
    }
}

fn validate_values(values: &SettingsValues) -> Result<(), AppError> {
    if !(1..=10).contains(&values.max_concurrent_tasks)
        || !(1..=32).contains(&values.connections_per_task)
        || !matches!(values.locale.as_str(), "zh-CN" | "en-US")
    {
        return Err(invalid_value_error());
    }
    validate_directory(Path::new(&values.download_directory))?;
    validate_directory(Path::new(&values.temporary_directory))?;
    Ok(())
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
