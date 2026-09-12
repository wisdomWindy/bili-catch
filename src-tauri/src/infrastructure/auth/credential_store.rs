use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    models::{AppError, AppErrorCode},
    services::auth::ports::{CredentialStorePort, StoredCredential},
};

const SERVICE: &str = "com.bilicatch.app";
const USER: &str = "bilibili-session";
const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendError {
    Unavailable,
    Failure,
}

trait SecretBackend: Send + Sync {
    fn get_secret(&self) -> Result<Option<Vec<u8>>, BackendError>;
    fn set_secret(&self, secret: &[u8]) -> Result<(), BackendError>;
    fn delete_secret(&self) -> Result<(), BackendError>;
}

struct KeyringSecretBackend;

impl KeyringSecretBackend {
    fn entry() -> Result<keyring::v1::Entry, BackendError> {
        keyring::v1::Entry::new(SERVICE, USER).map_err(map_keyring_error)
    }
}

impl SecretBackend for KeyringSecretBackend {
    fn get_secret(&self) -> Result<Option<Vec<u8>>, BackendError> {
        match Self::entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn set_secret(&self, secret: &[u8]) -> Result<(), BackendError> {
        Self::entry()?.set_secret(secret).map_err(map_keyring_error)
    }

    fn delete_secret(&self) -> Result<(), BackendError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::v1::Error::NoEntry) => Ok(()),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

fn map_keyring_error(error: keyring::v1::Error) -> BackendError {
    match error {
        keyring::v1::Error::NoStorageAccess(_) | keyring::v1::Error::NoDefaultStore => {
            BackendError::Unavailable
        }
        _ => BackendError::Failure,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentialWireRef<'a> {
    schema_version: u8,
    cookie_header: &'a str,
    refresh_token: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentialWire {
    schema_version: u8,
    cookie_header: String,
    refresh_token: Option<String>,
}

fn encode_credential(credential: &StoredCredential) -> Result<Zeroizing<Vec<u8>>, AppError> {
    serde_json::to_vec(&StoredCredentialWireRef {
        schema_version: SCHEMA_VERSION,
        cookie_header: credential.expose_cookie_header(),
        refresh_token: credential.expose_refresh_token(),
    })
    .map(Zeroizing::new)
    .map_err(|_| AppError::internal("Unable to encode the secure credential"))
}

fn decode_credential(secret: Vec<u8>) -> Result<StoredCredential, AppError> {
    let secret = Zeroizing::new(secret);
    let wire: StoredCredentialWire = serde_json::from_slice(&secret)
        .map_err(|_| AppError::internal("The secure credential is corrupt"))?;
    if wire.schema_version != SCHEMA_VERSION || wire.cookie_header.trim().is_empty() {
        return Err(AppError::internal("The secure credential is corrupt"));
    }
    StoredCredential::try_new(wire.cookie_header, wire.refresh_token)
}

fn map_backend_error(error: BackendError) -> AppError {
    match error {
        BackendError::Unavailable => AppError::new(
            AppErrorCode::E007,
            "Secure credential storage is unavailable",
        ),
        BackendError::Failure => AppError::internal("Secure credential storage failed"),
    }
}

pub(crate) struct SystemCredentialStore {
    backend: Arc<dyn SecretBackend>,
}

impl SystemCredentialStore {
    pub(crate) fn new() -> Self {
        Self {
            backend: Arc::new(KeyringSecretBackend),
        }
    }

    #[cfg(test)]
    fn with_backend(backend: Arc<dyn SecretBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl CredentialStorePort for SystemCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredential>, AppError> {
        let backend = Arc::clone(&self.backend);
        let secret = tokio::task::spawn_blocking(move || backend.get_secret())
            .await
            .map_err(|_| AppError::internal("Secure credential storage was interrupted"))?
            .map_err(map_backend_error)?;
        secret.map(decode_credential).transpose()
    }

    async fn save(&self, credential: &StoredCredential) -> Result<(), AppError> {
        let secret = encode_credential(credential)?;
        let backend = Arc::clone(&self.backend);
        tokio::task::spawn_blocking(move || backend.set_secret(&secret))
            .await
            .map_err(|_| AppError::internal("Secure credential storage was interrupted"))?
            .map_err(map_backend_error)
    }

    async fn delete(&self) -> Result<(), AppError> {
        let backend = Arc::clone(&self.backend);
        tokio::task::spawn_blocking(move || backend.delete_secret())
            .await
            .map_err(|_| AppError::internal("Secure credential storage was interrupted"))?
            .map_err(map_backend_error)
    }
}

#[cfg(test)]
#[derive(Default)]
struct FakeSecretBackend {
    state: std::sync::Mutex<FakeBackendState>,
}

#[cfg(test)]
#[derive(Default)]
struct FakeBackendState {
    secret: Option<Vec<u8>>,
    get_error: Option<BackendError>,
    set_error: Option<BackendError>,
    delete_error: Option<BackendError>,
}

#[cfg(test)]
impl FakeSecretBackend {
    fn with_secret(secret: Vec<u8>) -> Self {
        Self {
            state: std::sync::Mutex::new(FakeBackendState {
                secret: Some(secret),
                ..FakeBackendState::default()
            }),
        }
    }

    fn failing(operation: &str, error: BackendError) -> Self {
        let mut state = FakeBackendState::default();
        match operation {
            "get" => state.get_error = Some(error),
            "set" => state.set_error = Some(error),
            "delete" => state.delete_error = Some(error),
            _ => unreachable!(),
        }
        Self {
            state: std::sync::Mutex::new(state),
        }
    }
}

#[cfg(test)]
impl SecretBackend for FakeSecretBackend {
    fn get_secret(&self) -> Result<Option<Vec<u8>>, BackendError> {
        let state = self.state.lock().expect("fake backend lock poisoned");
        state
            .get_error
            .map_or_else(|| Ok(state.secret.clone()), Err)
    }

    fn set_secret(&self, secret: &[u8]) -> Result<(), BackendError> {
        let mut state = self.state.lock().expect("fake backend lock poisoned");
        if let Some(error) = state.set_error {
            return Err(error);
        }
        state.secret = Some(secret.to_vec());
        Ok(())
    }

    fn delete_secret(&self) -> Result<(), BackendError> {
        let mut state = self.state.lock().expect("fake backend lock poisoned");
        if let Some(error) = state.delete_error {
            return Err(error);
        }
        state.secret = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::services::auth::ports::{CredentialStorePort, StoredCredential};

    use super::{BackendError, FakeSecretBackend, SystemCredentialStore};

    #[test]
    fn round_trips_a_versioned_secret_and_treats_missing_as_anonymous() {
        tauri::async_runtime::block_on(async {
            let backend = Arc::new(FakeSecretBackend::default());
            let store = SystemCredentialStore::with_backend(backend.clone());
            assert!(store.load().await.unwrap().is_none());

            store
                .save(
                    &StoredCredential::try_new(
                        "DedeUserID=9001; SESSDATA=fixture-session".into(),
                        Some("fixture-refresh".into()),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let loaded = store
                .load()
                .await
                .unwrap()
                .expect("credential should exist");
            assert_eq!(
                loaded.expose_cookie_header(),
                "DedeUserID=9001; SESSDATA=fixture-session"
            );
            assert_eq!(loaded.expose_refresh_token(), Some("fixture-refresh"));

            store.delete().await.unwrap();
            assert!(store.load().await.unwrap().is_none());
        });
    }

    #[test]
    fn rejects_a_corrupt_blob_without_echoing_secret_bytes() {
        tauri::async_runtime::block_on(async {
            let backend = Arc::new(FakeSecretBackend::with_secret(
                br#"{"schemaVersion":9,"cookieHeader":"must-not-leak"}"#.to_vec(),
            ));
            let store = SystemCredentialStore::with_backend(backend);
            let error = store.load().await.err().expect("unknown schema must fail");
            let serialized = serde_json::to_string(&error).unwrap();
            assert!(!serialized.contains("must-not-leak"));
        });
    }

    #[test]
    fn rejects_a_versioned_blob_missing_required_cookies() {
        tauri::async_runtime::block_on(async {
            let backend = Arc::new(FakeSecretBackend::with_secret(
                br#"{"schemaVersion":1,"cookieHeader":"SESSDATA=fixture-session","refreshToken":null}"#
                    .to_vec(),
            ));
            let store = SystemCredentialStore::with_backend(backend);
            assert!(store.load().await.is_err());
        });
    }

    #[test]
    fn maps_get_set_and_delete_failures_without_plaintext_fallback() {
        tauri::async_runtime::block_on(async {
            for operation in ["get", "set", "delete"] {
                let backend = Arc::new(FakeSecretBackend::failing(
                    operation,
                    BackendError::Unavailable,
                ));
                let store = SystemCredentialStore::with_backend(backend);
                let result = match operation {
                    "get" => store.load().await.map(|_| ()),
                    "set" => {
                        store
                            .save(
                                &StoredCredential::try_new(
                                    "DedeUserID=9001; SESSDATA=fixture-session".into(),
                                    None,
                                )
                                .unwrap(),
                            )
                            .await
                    }
                    "delete" => store.delete().await,
                    _ => unreachable!(),
                };
                let error = result.expect_err("platform failure must be visible");
                assert_eq!(serde_json::to_value(error).unwrap()["code"], "E007");
            }
        });
    }

    #[test]
    fn production_adapter_can_be_constructed_without_accessing_a_sample_store() {
        let _store = SystemCredentialStore::new();
    }
}
