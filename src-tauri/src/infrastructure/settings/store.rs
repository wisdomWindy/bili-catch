use std::sync::Arc;

use serde_json::Value;
use tauri::{Manager, Runtime};
use tauri_plugin_store::{resolve_store_path, Store, StoreBuilder};

use crate::{
    models::{AppError, AppErrorCode, SettingsDocument},
    services::settings::SettingsStorePort,
};

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "document";

trait SettingsCache: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&self, key: &str, value: Value);
    fn delete(&self, key: &str);
    fn save(&self) -> Result<(), String>;
}

struct TransactionalSettingsStore<C> {
    cache: C,
}

impl<C> TransactionalSettingsStore<C> {
    fn new(cache: C) -> Self {
        Self { cache }
    }
}

impl<C: SettingsCache> SettingsStorePort for TransactionalSettingsStore<C> {
    fn load_raw(&self) -> Result<Option<Value>, AppError> {
        Ok(self.cache.get(SETTINGS_KEY))
    }

    fn save_document(&self, document: &SettingsDocument) -> Result<(), AppError> {
        let previous = self.cache.get(SETTINGS_KEY);
        let candidate = serde_json::to_value(document).map_err(|_| settings_store_error())?;
        self.cache.set(SETTINGS_KEY, candidate);

        if self.cache.save().is_err() {
            match previous {
                Some(previous) => self.cache.set(SETTINGS_KEY, previous),
                None => self.cache.delete(SETTINGS_KEY),
            }
            return Err(settings_store_error());
        }
        Ok(())
    }
}

struct PluginSettingsCache<R: Runtime> {
    store: Arc<Store<R>>,
}

impl<R: Runtime> SettingsCache for PluginSettingsCache<R> {
    fn get(&self, key: &str) -> Option<Value> {
        self.store.get(key)
    }

    fn set(&self, key: &str, value: Value) {
        self.store.set(key, value);
    }

    fn delete(&self, key: &str) {
        self.store.delete(key);
    }

    fn save(&self) -> Result<(), String> {
        self.store.save().map_err(|error| error.to_string())
    }
}

pub struct TauriSettingsStore<R: Runtime> {
    inner: TransactionalSettingsStore<PluginSettingsCache<R>>,
}

impl<R: Runtime> TauriSettingsStore<R> {
    pub fn build<M: Manager<R>>(manager: &M) -> Result<Self, AppError> {
        let resolved_path = resolve_store_path(manager.app_handle(), SETTINGS_FILE)
            .map_err(|_| settings_store_error())?;
        let existed = resolved_path.exists();
        let store = StoreBuilder::new(manager, SETTINGS_FILE)
            .disable_auto_save()
            .build()
            .map_err(|_| settings_store_error())?;

        // StoreBuilder intentionally ignores initial load errors. Re-read existing
        // files so corrupt data is surfaced instead of being replaced by defaults.
        if existed {
            store
                .reload_ignore_defaults()
                .map_err(|_| settings_store_error())?;
        }

        Ok(Self {
            inner: TransactionalSettingsStore::new(PluginSettingsCache { store }),
        })
    }
}

impl<R: Runtime> SettingsStorePort for TauriSettingsStore<R> {
    fn load_raw(&self) -> Result<Option<Value>, AppError> {
        self.inner.load_raw()
    }

    fn save_document(&self, document: &SettingsDocument) -> Result<(), AppError> {
        self.inner.save_document(document)
    }
}

fn settings_store_error() -> AppError {
    AppError {
        code: AppErrorCode::Internal,
        message: "Settings storage is unavailable".into(),
        details: Some("SETTINGS_STORE_UNAVAILABLE".into()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::{json, Value};

    use super::{SettingsCache, TransactionalSettingsStore};
    use crate::{
        models::{
            AudioFormat, CloseBehavior, SettingsDocument, SettingsValues, ThemePreference,
            VideoQualityId,
        },
        services::settings::SettingsStorePort,
    };

    struct FailingCache {
        document: Mutex<Option<Value>>,
    }

    impl SettingsCache for FailingCache {
        fn get(&self, _key: &str) -> Option<Value> {
            self.document.lock().expect("cache lock").clone()
        }

        fn set(&self, _key: &str, value: Value) {
            *self.document.lock().expect("cache lock") = Some(value);
        }

        fn delete(&self, _key: &str) {
            *self.document.lock().expect("cache lock") = None;
        }

        fn save(&self) -> Result<(), String> {
            Err("injected disk failure".into())
        }
    }

    fn document(revision: u64, theme: ThemePreference) -> SettingsDocument {
        SettingsDocument {
            schema_version: 1,
            revision,
            values: SettingsValues {
                download_directory: "C:/Downloads/BiliCatch".into(),
                temporary_directory: "C:/Temp".into(),
                max_concurrent_tasks: 3,
                connections_per_task: 8,
                default_video_quality: VideoQualityId::P1080,
                default_audio_format: AudioFormat::Mp3,
                theme,
                locale: "zh-CN".into(),
                notify_on_complete: true,
                close_behavior: CloseBehavior::MinimizeToTray,
                auto_check_updates: true,
            },
        }
    }

    #[test]
    fn failed_disk_save_restores_the_previous_plugin_cache_value() {
        let original = document(0, ThemePreference::System);
        let candidate = document(1, ThemePreference::Dark);
        let cache = FailingCache {
            document: Mutex::new(Some(json!(original))),
        };
        let store = TransactionalSettingsStore::new(cache);

        let error = store
            .save_document(&candidate)
            .expect_err("injected save should fail");

        assert_eq!(error.details.as_deref(), Some("SETTINGS_STORE_UNAVAILABLE"));
        assert_eq!(
            store.load_raw().expect("cache should load"),
            Some(json!(original))
        );
    }

    #[test]
    fn failed_first_save_removes_the_unsaved_plugin_cache_value() {
        let cache = FailingCache {
            document: Mutex::new(None),
        };
        let store = TransactionalSettingsStore::new(cache);

        store
            .save_document(&document(0, ThemePreference::System))
            .expect_err("injected save should fail");

        assert_eq!(store.load_raw().expect("cache should load"), None);
    }
}
