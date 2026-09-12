use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use time::format_description::well_known::Rfc3339;

use crate::models::{AppError, AuthAccount, AuthSnapshot, AuthStateEvent, AuthStatus};

use super::ports::{
    AuthClock, AuthEventSink, AuthSleeper, CredentialStorePort, QrAuthPort, StoredCredential,
};

const QR_LIFETIME: Duration = Duration::from_secs(180);

pub(super) struct AuthManagerState {
    pub(super) generation: u64,
    pub(super) snapshot: AuthSnapshot,
    pub(super) credential: Option<StoredCredential>,
    pub(super) poll_task: Option<tokio::task::JoinHandle<()>>,
}

pub(crate) struct AuthManager {
    pub(super) qr: Arc<dyn QrAuthPort>,
    pub(super) store: Arc<dyn CredentialStorePort>,
    pub(super) clock: Arc<dyn AuthClock>,
    pub(super) sleeper: Arc<dyn AuthSleeper>,
    pub(super) events: Arc<dyn AuthEventSink>,
    pub(super) credential_gate: tokio::sync::Semaphore,
    pub(super) state: Mutex<AuthManagerState>,
}

impl AuthManager {
    pub(crate) fn new(
        qr: Arc<dyn QrAuthPort>,
        store: Arc<dyn CredentialStorePort>,
        clock: Arc<dyn AuthClock>,
        sleeper: Arc<dyn AuthSleeper>,
        events: Arc<dyn AuthEventSink>,
    ) -> Self {
        Self {
            qr,
            store,
            clock,
            sleeper,
            events,
            credential_gate: tokio::sync::Semaphore::new(1),
            state: Mutex::new(AuthManagerState {
                generation: 0,
                snapshot: AuthSnapshot {
                    revision: 0,
                    status: AuthStatus::Restoring,
                    qr_content: None,
                    expires_at: None,
                    account: None,
                    error: None,
                },
                credential: None,
                poll_task: None,
            }),
        }
    }

    pub(crate) fn snapshot(&self) -> AuthSnapshot {
        self.state
            .lock()
            .expect("auth manager lock poisoned")
            .snapshot
            .clone()
    }

    pub(crate) async fn start_login(self: &Arc<Self>) -> Result<AuthSnapshot, AppError> {
        let (generation, previous_task, requesting) = self.begin_generation(AuthStatus::Requesting);
        if let Some(task) = previous_task {
            task.abort();
        }
        self.emit(&requesting);

        let generated = match self.qr.generate_qr().await {
            Ok(generated) => generated,
            Err(error) => {
                self.commit_error(generation, error.clone());
                return Err(error);
            }
        };
        if !self.is_current(generation) {
            return Ok(self.snapshot());
        }

        let deadline = self.clock.monotonic_elapsed() + QR_LIFETIME;
        let expires_at = match (self.clock.utc_now() + QR_LIFETIME).format(&Rfc3339) {
            Ok(value) => value,
            Err(_) => {
                let error = AppError::internal("System clock is unavailable");
                self.commit_error(generation, error.clone());
                return Err(error);
            }
        };
        let Some(waiting) = self.commit_if_current(generation, |state| {
            set_snapshot_status(&mut state.snapshot, AuthStatus::WaitingScan);
            state.snapshot.qr_content = Some(generated.qr_content);
            state.snapshot.expires_at = Some(expires_at);
            state.credential = None;
        }) else {
            return Ok(self.snapshot());
        };
        self.emit(&waiting);

        let manager = Arc::clone(self);
        let task = tokio::spawn(async move {
            manager
                .poll_until_terminal(generation, generated.qrcode_key, deadline)
                .await;
        });
        let mut state = self.state.lock().expect("auth manager lock poisoned");
        if state.generation == generation {
            state.poll_task = Some(task);
        } else {
            task.abort();
        }
        Ok(waiting)
    }

    pub(crate) fn cancel_login(&self) -> Result<AuthSnapshot, AppError> {
        let (task, snapshot) = {
            let mut state = self.state.lock().expect("auth manager lock poisoned");
            if !matches!(
                state.snapshot.status,
                AuthStatus::Requesting | AuthStatus::WaitingScan | AuthStatus::WaitingConfirm
            ) {
                return Ok(state.snapshot.clone());
            }
            state.generation = state.generation.saturating_add(1);
            let task = state.poll_task.take();
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            set_snapshot_status(&mut state.snapshot, AuthStatus::Cancelled);
            state.credential = None;
            (task, state.snapshot.clone())
        };
        if let Some(task) = task {
            task.abort();
        }
        self.emit(&snapshot);
        Ok(snapshot)
    }

    pub(super) fn begin_generation(
        &self,
        status: AuthStatus,
    ) -> (u64, Option<tokio::task::JoinHandle<()>>, AuthSnapshot) {
        let mut state = self.state.lock().expect("auth manager lock poisoned");
        state.generation = state.generation.saturating_add(1);
        let task = state.poll_task.take();
        state.snapshot.revision = state.snapshot.revision.saturating_add(1);
        set_snapshot_status(&mut state.snapshot, status);
        state.credential = None;
        (state.generation, task, state.snapshot.clone())
    }

    pub(super) fn is_current(&self, generation: u64) -> bool {
        self.state
            .lock()
            .expect("auth manager lock poisoned")
            .generation
            == generation
    }

    fn commit_if_current(
        &self,
        generation: u64,
        mutate: impl FnOnce(&mut AuthManagerState),
    ) -> Option<AuthSnapshot> {
        let mut state = self.state.lock().expect("auth manager lock poisoned");
        if state.generation != generation {
            return None;
        }
        state.snapshot.revision = state.snapshot.revision.saturating_add(1);
        mutate(&mut state);
        Some(state.snapshot.clone())
    }

    pub(super) fn commit_waiting_status(&self, generation: u64, status: AuthStatus) {
        let snapshot = {
            let mut state = self.state.lock().expect("auth manager lock poisoned");
            if state.generation != generation || state.snapshot.status == status {
                return;
            }
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            state.snapshot.status = status;
            state.snapshot.error = None;
            state.snapshot.clone()
        };
        self.emit(&snapshot);
    }

    pub(super) fn commit_terminal(
        &self,
        generation: u64,
        status: AuthStatus,
        error: Option<AppError>,
    ) -> AuthSnapshot {
        let Some(snapshot) = self.commit_if_current(generation, |state| {
            set_snapshot_status(&mut state.snapshot, status);
            state.snapshot.error = error;
            state.credential = None;
            state.poll_task = None;
        }) else {
            return self.snapshot();
        };
        self.emit(&snapshot);
        snapshot
    }

    pub(super) fn commit_error(&self, generation: u64, error: AppError) -> AuthSnapshot {
        self.commit_terminal(generation, AuthStatus::Error, Some(error))
    }

    pub(super) fn commit_error_with_credential(
        &self,
        generation: u64,
        error: AppError,
        credential: StoredCredential,
    ) -> AuthSnapshot {
        let Some(snapshot) = self.commit_if_current(generation, |state| {
            set_snapshot_status(&mut state.snapshot, AuthStatus::Error);
            state.snapshot.error = Some(error);
            state.credential = Some(credential);
        }) else {
            return self.snapshot();
        };
        self.emit(&snapshot);
        snapshot
    }

    pub(super) fn commit_error_for_current_generation(&self, error: AppError) {
        let (task, snapshot) = {
            let mut state = self.state.lock().expect("auth manager lock poisoned");
            state.generation = state.generation.saturating_add(1);
            state.snapshot.revision = state.snapshot.revision.saturating_add(1);
            set_snapshot_status(&mut state.snapshot, AuthStatus::Error);
            state.snapshot.error = Some(error);
            state.credential = None;
            (state.poll_task.take(), state.snapshot.clone())
        };
        if let Some(task) = task {
            task.abort();
        }
        self.emit(&snapshot);
    }

    pub(super) fn commit_authenticated(
        &self,
        generation: u64,
        credential: StoredCredential,
        account: AuthAccount,
    ) -> AuthSnapshot {
        let Some(snapshot) = self.commit_if_current(generation, |state| {
            set_snapshot_status(&mut state.snapshot, AuthStatus::Authenticated);
            state.snapshot.account = Some(account);
            state.credential = Some(credential);
            state.poll_task = None;
        }) else {
            return self.snapshot();
        };
        self.emit(&snapshot);
        snapshot
    }

    pub(super) fn commit_anonymous(&self, generation: u64) -> AuthSnapshot {
        let Some(snapshot) = self.commit_if_current(generation, |state| {
            set_snapshot_status(&mut state.snapshot, AuthStatus::Anonymous);
            state.credential = None;
            state.poll_task = None;
        }) else {
            return self.snapshot();
        };
        self.emit(&snapshot);
        snapshot
    }

    pub(super) fn emit(&self, snapshot: &AuthSnapshot) {
        let _ = self.events.emit(AuthStateEvent {
            snapshot: snapshot.clone(),
        });
    }
}

pub(super) fn set_snapshot_status(snapshot: &mut AuthSnapshot, status: AuthStatus) {
    snapshot.status = status;
    snapshot.qr_content = None;
    snapshot.expires_at = None;
    snapshot.account = None;
    snapshot.error = None;
}
