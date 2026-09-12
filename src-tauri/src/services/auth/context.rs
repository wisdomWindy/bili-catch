use async_trait::async_trait;

use crate::models::{AppError, AuthAccount, AuthSnapshot, AuthStatus};

use super::{
    manager::{set_snapshot_status, AuthManager},
    ports::{SessionValidation, StoredCredential},
};

pub(crate) struct ValidatedAuthContext {
    revision: u64,
    credential: Option<StoredCredential>,
}

impl ValidatedAuthContext {
    pub(crate) fn anonymous(revision: u64) -> Self {
        Self {
            revision,
            credential: None,
        }
    }

    pub(crate) fn authenticated(revision: u64, credential: StoredCredential) -> Self {
        Self {
            revision,
            credential: Some(credential),
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn credential(&self) -> Option<&StoredCredential> {
        self.credential.as_ref()
    }

    pub(crate) fn is_authenticated(&self) -> bool {
        self.credential.is_some()
    }
}

#[async_trait]
pub(crate) trait AuthContextProvider: Send + Sync {
    async fn validated_context(&self) -> Result<ValidatedAuthContext, AppError>;
}

struct AuthContextCandidate {
    generation: u64,
    revision: u64,
    status: AuthStatus,
    account: Option<AuthAccount>,
    credential: Option<StoredCredential>,
}

impl AuthManager {
    pub(crate) async fn restore_on_startup(&self) -> Result<AuthSnapshot, AppError> {
        let (generation, previous_task, restoring) = self.begin_generation(AuthStatus::Restoring);
        if let Some(task) = previous_task {
            task.abort();
        }
        self.emit(&restoring);

        let load_result = {
            let _permit = self
                .credential_gate
                .acquire()
                .await
                .map_err(|_| AppError::internal("Credential operations are unavailable"))?;
            self.store.load().await
        };
        let credential = match load_result {
            Ok(Some(credential)) => credential,
            Ok(None) => return Ok(self.commit_anonymous(generation)),
            Err(error) => {
                self.commit_error(generation, error.clone());
                return Err(error);
            }
        };
        if !self.is_current(generation) {
            return Ok(self.snapshot());
        }

        match self.qr.validate_session(&credential).await {
            Ok(SessionValidation::Valid(account)) => {
                Ok(self.commit_authenticated(generation, credential, account))
            }
            Ok(SessionValidation::Invalid) => {
                let delete_result = {
                    let _permit =
                        self.credential_gate.acquire().await.map_err(|_| {
                            AppError::internal("Credential operations are unavailable")
                        })?;
                    if !self.is_current(generation) {
                        return Ok(self.snapshot());
                    }
                    self.store.delete().await
                };
                match delete_result {
                    Ok(()) => Ok(self.commit_anonymous(generation)),
                    Err(error) => {
                        self.commit_error_with_credential(generation, error.clone(), credential);
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.commit_error_with_credential(generation, error.clone(), credential);
                Err(error)
            }
        }
    }

    pub(crate) async fn logout(&self) -> Result<AuthSnapshot, AppError> {
        let (generation, status) = {
            let state = self.state.lock().expect("auth manager lock poisoned");
            (state.generation, state.snapshot.status)
        };
        if status != AuthStatus::Authenticated {
            return Ok(self.snapshot());
        }
        let delete_result = {
            let _permit = self
                .credential_gate
                .acquire()
                .await
                .map_err(|_| AppError::internal("Credential operations are unavailable"))?;
            self.store.delete().await
        };
        if let Err(error) = delete_result {
            return Err(error);
        }
        Ok(self
            .invalidate_session_if_current(generation)
            .unwrap_or_else(|| self.snapshot()))
    }

    fn context_candidate(&self) -> AuthContextCandidate {
        let state = self.state.lock().expect("auth manager lock poisoned");
        AuthContextCandidate {
            generation: state.generation,
            revision: state.snapshot.revision,
            status: state.snapshot.status,
            account: state.snapshot.account.clone(),
            credential: state.credential.clone(),
        }
    }

    pub(super) fn invalidate_session_if_current(&self, generation: u64) -> Option<AuthSnapshot> {
        let (task, snapshot) = {
            let mut state = self.state.lock().expect("auth manager lock poisoned");
            if state.generation != generation {
                return None;
            }
            state.generation = state.generation.saturating_add(1);
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            set_snapshot_status(&mut state.snapshot, AuthStatus::Anonymous);
            state.credential = None;
            (state.poll_task.take(), state.snapshot.clone())
        };
        if let Some(task) = task {
            task.abort();
        }
        self.emit(&snapshot);
        Some(snapshot)
    }
}

#[async_trait]
impl AuthContextProvider for AuthManager {
    async fn validated_context(&self) -> Result<ValidatedAuthContext, AppError> {
        for _ in 0..2 {
            let candidate = self.context_candidate();
            let Some(credential) = candidate.credential else {
                return Ok(ValidatedAuthContext::anonymous(candidate.revision));
            };

            match self.qr.validate_session(&credential).await? {
                SessionValidation::Valid(account) => {
                    if !self.is_current(candidate.generation) {
                        continue;
                    }
                    if candidate.status == AuthStatus::Authenticated
                        && candidate.account.as_ref() == Some(&account)
                    {
                        return Ok(ValidatedAuthContext::authenticated(
                            candidate.revision,
                            credential,
                        ));
                    }
                    let context_credential = credential.clone();
                    let snapshot =
                        self.commit_authenticated(candidate.generation, credential, account);
                    return Ok(ValidatedAuthContext::authenticated(
                        snapshot.revision,
                        context_credential,
                    ));
                }
                SessionValidation::Invalid => {
                    let _permit =
                        self.credential_gate.acquire().await.map_err(|_| {
                            AppError::internal("Credential operations are unavailable")
                        })?;
                    if !self.is_current(candidate.generation) {
                        continue;
                    }
                    self.store.delete().await?;
                    if let Some(snapshot) = self.invalidate_session_if_current(candidate.generation)
                    {
                        return Ok(ValidatedAuthContext::anonymous(snapshot.revision));
                    }
                }
            }
        }
        Err(AppError::internal(
            "Authentication changed while it was being validated",
        ))
    }
}
