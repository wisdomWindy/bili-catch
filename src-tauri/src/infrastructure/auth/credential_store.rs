use std::sync::Mutex;

use async_trait::async_trait;

use crate::{
    models::AppError,
    services::auth::ports::{CredentialStorePort, StoredCredential},
};

/// Process-scoped credential storage. Nothing is written to disk, so a new
/// application process always starts without a saved login.
pub(crate) struct InMemoryCredentialStore {
    credential: Mutex<Option<StoredCredential>>,
}

impl InMemoryCredentialStore {
    pub(crate) fn new() -> Self {
        Self {
            credential: Mutex::new(None),
        }
    }
}

#[async_trait]
impl CredentialStorePort for InMemoryCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredential>, AppError> {
        Ok(self.credential.lock().unwrap().clone())
    }

    async fn save(&self, credential: &StoredCredential) -> Result<(), AppError> {
        *self.credential.lock().unwrap() = Some(credential.clone());
        Ok(())
    }

    async fn delete(&self) -> Result<(), AppError> {
        *self.credential.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::services::auth::ports::{CredentialStorePort, StoredCredential};

    use super::InMemoryCredentialStore;

    fn credential() -> StoredCredential {
        StoredCredential::try_new("DedeUserID=9001; SESSDATA=fixture-session".into(), None).unwrap()
    }

    #[test]
    fn stores_credentials_only_for_the_current_process() {
        tauri::async_runtime::block_on(async {
            let first = InMemoryCredentialStore::new();
            first.save(&credential()).await.unwrap();
            assert!(first.load().await.unwrap().is_some());

            let second = InMemoryCredentialStore::new();
            assert!(second.load().await.unwrap().is_none());

            first.delete().await.unwrap();
            assert!(first.load().await.unwrap().is_none());
        });
    }
}
