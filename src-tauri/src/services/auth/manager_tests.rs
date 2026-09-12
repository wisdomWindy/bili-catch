use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use async_trait::async_trait;
use secrecy::SecretString;
use time::OffsetDateTime;

use crate::models::{AppError, AuthAccount, AuthStateEvent, AuthStatus};

use super::{
    context::AuthContextProvider,
    manager::AuthManager,
    ports::{
        AuthClock, AuthEventSink, AuthSleeper, CredentialStorePort, GeneratedQr, QrAuthPort,
        QrPollResult, SessionValidation, StoredCredential,
    },
};

struct FakeClock {
    millis: AtomicU64,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            millis: AtomicU64::new(0),
        }
    }
}

impl AuthClock for FakeClock {
    fn monotonic_elapsed(&self) -> Duration {
        Duration::from_millis(self.millis.load(Ordering::SeqCst))
    }

    fn utc_now(&self) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::from_millis(self.millis.load(Ordering::SeqCst))
    }
}

struct AdvancingSleeper {
    clock: Arc<FakeClock>,
}

#[async_trait]
impl AuthSleeper for AdvancingSleeper {
    async fn sleep(&self, duration: Duration) {
        self.clock
            .millis
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
        tokio::task::yield_now().await;
    }
}

struct FakeQrPort {
    polls: Mutex<VecDeque<QrPollResult>>,
    validation: Mutex<Option<SessionValidation>>,
    clock: Arc<FakeClock>,
    poll_times: Mutex<Vec<u64>>,
    fail_validation: AtomicBool,
    force_invalid: AtomicBool,
    block_validation: AtomicBool,
    validation_entered: AtomicBool,
    validation_release: tokio::sync::Notify,
}

impl FakeQrPort {
    fn new(
        clock: Arc<FakeClock>,
        polls: impl IntoIterator<Item = QrPollResult>,
        validation: SessionValidation,
    ) -> Self {
        Self {
            polls: Mutex::new(polls.into_iter().collect()),
            validation: Mutex::new(Some(validation)),
            clock,
            poll_times: Mutex::new(Vec::new()),
            fail_validation: AtomicBool::new(false),
            force_invalid: AtomicBool::new(false),
            block_validation: AtomicBool::new(false),
            validation_entered: AtomicBool::new(false),
            validation_release: tokio::sync::Notify::new(),
        }
    }
}

#[async_trait]
impl QrAuthPort for FakeQrPort {
    async fn generate_qr(&self) -> Result<GeneratedQr, AppError> {
        Ok(GeneratedQr {
            qr_content: "https://passport.bilibili.com/fixture-qr".into(),
            qrcode_key: SecretString::from("fixture-key"),
        })
    }

    async fn poll_qr(&self, _qrcode_key: &SecretString) -> Result<QrPollResult, AppError> {
        self.poll_times
            .lock()
            .unwrap()
            .push(self.clock.millis.load(Ordering::SeqCst));
        Ok(self
            .polls
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(QrPollResult::WaitingScan))
    }

    async fn validate_session(
        &self,
        _credential: &StoredCredential,
    ) -> Result<SessionValidation, AppError> {
        if self.fail_validation.load(Ordering::SeqCst) {
            return Err(AppError::internal("fixture validation failure"));
        }
        if self.force_invalid.load(Ordering::SeqCst) {
            return Ok(SessionValidation::Invalid);
        }
        if self.block_validation.load(Ordering::SeqCst) {
            self.validation_entered.store(true, Ordering::SeqCst);
            self.validation_release.notified().await;
        }
        Ok(self
            .validation
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(valid_account))
    }
}

async fn wait_for_condition(condition: impl Fn() -> bool) {
    let timeout = std::time::Instant::now() + Duration::from_secs(1);
    while !condition() {
        assert!(
            std::time::Instant::now() < timeout,
            "condition did not become true before timeout"
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

fn valid_account() -> SessionValidation {
    SessionValidation::Valid(AuthAccount {
        mid: Some("9001".into()),
        name: "Fixture account".into(),
        avatar_url: None,
    })
}

fn credential() -> StoredCredential {
    StoredCredential::try_new("DedeUserID=9001; SESSDATA=fixture-session".into(), None).unwrap()
}

#[derive(Default)]
struct FakeStore {
    value: Mutex<Option<StoredCredential>>,
    save_calls: AtomicUsize,
    delete_calls: AtomicUsize,
    fail_save: AtomicBool,
    fail_delete: AtomicBool,
}

#[async_trait]
impl CredentialStorePort for FakeStore {
    async fn load(&self) -> Result<Option<StoredCredential>, AppError> {
        Ok(self.value.lock().unwrap().clone())
    }

    async fn save(&self, credential: &StoredCredential) -> Result<(), AppError> {
        self.save_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(AppError::internal("fixture save failure"));
        }
        *self.value.lock().unwrap() = Some(credential.clone());
        Ok(())
    }

    async fn delete(&self) -> Result<(), AppError> {
        self.delete_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(AppError::internal("fixture delete failure"));
        }
        *self.value.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Default)]
struct FakeSink {
    events: Mutex<Vec<AuthStateEvent>>,
}

impl AuthEventSink for FakeSink {
    fn emit(&self, event: AuthStateEvent) -> Result<(), AppError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

fn manager(
    clock: Arc<FakeClock>,
    qr: Arc<FakeQrPort>,
    store: Arc<FakeStore>,
    sink: Arc<FakeSink>,
) -> Arc<AuthManager> {
    Arc::new(AuthManager::new(
        qr,
        store,
        clock.clone(),
        Arc::new(AdvancingSleeper { clock }),
        sink,
    ))
}

async fn wait_for_status(manager: &AuthManager, expected: AuthStatus) {
    let timeout = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        if manager.snapshot().status == expected {
            return;
        }
        if std::time::Instant::now() >= timeout {
            let snapshot = manager.snapshot();
            panic!(
                "auth status did not reach {expected:?}; current={:?}, revision={}",
                snapshot.status, snapshot.revision
            );
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn polls_on_two_second_boundaries_and_commits_only_after_secure_save() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(
            clock.clone(),
            [
                QrPollResult::WaitingScan,
                QrPollResult::WaitingConfirm,
                QrPollResult::Confirmed(credential()),
            ],
            valid_account(),
        ));
        let store = Arc::new(FakeStore::default());
        let sink = Arc::new(FakeSink::default());
        let manager = manager(clock, qr.clone(), store.clone(), sink.clone());

        manager.start_login().await.unwrap();
        wait_for_status(&manager, AuthStatus::Authenticated).await;

        assert_eq!(*qr.poll_times.lock().unwrap(), vec![2000, 4000, 6000]);
        assert_eq!(store.save_calls.load(Ordering::SeqCst), 1);
        let events = sink.events.lock().unwrap();
        let authenticated = events
            .iter()
            .position(|event| event.snapshot.status == AuthStatus::Authenticated)
            .unwrap();
        assert!(events[..authenticated]
            .iter()
            .any(|event| event.snapshot.status == AuthStatus::WaitingConfirm));
    });
}

#[test]
fn expires_at_the_local_deadline_without_a_ninety_first_poll() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let manager = manager(
            clock.clone(),
            qr.clone(),
            Arc::new(FakeStore::default()),
            Arc::new(FakeSink::default()),
        );

        manager.start_login().await.unwrap();
        wait_for_status(&manager, AuthStatus::Expired).await;

        assert_eq!(clock.millis.load(Ordering::SeqCst), 180_000);
        assert!(qr.poll_times.lock().unwrap().len() <= 90);
    });
}

#[test]
fn save_failure_never_emits_authenticated() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(
            clock.clone(),
            [QrPollResult::Confirmed(credential())],
            valid_account(),
        ));
        let store = Arc::new(FakeStore::default());
        store.fail_save.store(true, Ordering::SeqCst);
        let sink = Arc::new(FakeSink::default());
        let manager = manager(clock, qr, store, sink.clone());

        manager.start_login().await.unwrap();
        wait_for_status(&manager, AuthStatus::Error).await;

        assert!(!sink
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| event.snapshot.status == AuthStatus::Authenticated));
    });
}

#[test]
fn cancel_invalidates_the_active_generation_before_poll_or_save() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(
            clock.clone(),
            [QrPollResult::Confirmed(credential())],
            valid_account(),
        ));
        let store = Arc::new(FakeStore::default());
        let manager = manager(
            clock,
            qr.clone(),
            store.clone(),
            Arc::new(FakeSink::default()),
        );

        manager.start_login().await.unwrap();
        let cancelled = manager.cancel_login().unwrap();

        assert_eq!(cancelled.status, AuthStatus::Cancelled);
        tokio::task::yield_now().await;
        assert!(qr.poll_times.lock().unwrap().is_empty());
        assert_eq!(store.save_calls.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn a_confirmed_result_that_finishes_after_cancel_cannot_save() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(
            clock.clone(),
            [QrPollResult::Confirmed(credential())],
            valid_account(),
        ));
        qr.block_validation.store(true, Ordering::SeqCst);
        let store = Arc::new(FakeStore::default());
        let manager = manager(
            clock,
            qr.clone(),
            store.clone(),
            Arc::new(FakeSink::default()),
        );

        manager.start_login().await.unwrap();
        wait_for_condition(|| qr.validation_entered.load(Ordering::SeqCst)).await;
        manager.cancel_login().unwrap();
        qr.validation_release.notify_waiters();
        tokio::time::sleep(Duration::from_millis(5)).await;

        assert_eq!(manager.snapshot().status, AuthStatus::Cancelled);
        assert_eq!(store.save_calls.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn restore_and_logout_respect_secure_delete_commit_order() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(clock, qr, store.clone(), Arc::new(FakeSink::default()));

        manager.restore_on_startup().await.unwrap();
        assert_eq!(manager.snapshot().status, AuthStatus::Authenticated);

        store.fail_delete.store(true, Ordering::SeqCst);
        assert!(manager.logout().await.is_err());
        assert_eq!(manager.snapshot().status, AuthStatus::Authenticated);
        assert!(manager.snapshot().account.is_some());

        store.fail_delete.store(false, Ordering::SeqCst);
        manager.logout().await.unwrap();
        assert_eq!(manager.snapshot().status, AuthStatus::Anonymous);
        assert!(manager.snapshot().account.is_none());
    });
}

#[test]
fn restore_deletes_only_explicitly_invalid_credentials() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(
            clock.clone(),
            [],
            SessionValidation::Invalid,
        ));
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(clock, qr, store.clone(), Arc::new(FakeSink::default()));

        manager.restore_on_startup().await.unwrap();

        assert_eq!(manager.snapshot().status, AuthStatus::Anonymous);
        assert_eq!(store.delete_calls.load(Ordering::SeqCst), 1);
        assert!(store.value.lock().unwrap().is_none());
    });
}

#[test]
fn transient_restore_failure_keeps_the_secure_credential() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        qr.fail_validation.store(true, Ordering::SeqCst);
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(clock, qr, store.clone(), Arc::new(FakeSink::default()));

        assert!(manager.restore_on_startup().await.is_err());

        assert_eq!(manager.snapshot().status, AuthStatus::Error);
        assert_eq!(store.delete_calls.load(Ordering::SeqCst), 0);
        assert!(store.value.lock().unwrap().is_some());
    });
}

#[test]
fn parse_context_deletes_an_explicitly_invalid_session() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(
            clock,
            qr.clone(),
            store.clone(),
            Arc::new(FakeSink::default()),
        );
        manager.restore_on_startup().await.unwrap();
        qr.force_invalid.store(true, Ordering::SeqCst);

        let context = manager.validated_context().await.unwrap();

        assert!(!context.is_authenticated());
        assert_eq!(manager.snapshot().status, AuthStatus::Anonymous);
        assert_eq!(store.delete_calls.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn parse_context_transient_failure_preserves_the_authenticated_session() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(
            clock,
            qr.clone(),
            store.clone(),
            Arc::new(FakeSink::default()),
        );
        manager.restore_on_startup().await.unwrap();
        qr.fail_validation.store(true, Ordering::SeqCst);

        assert!(manager.validated_context().await.is_err());

        assert_eq!(manager.snapshot().status, AuthStatus::Authenticated);
        assert_eq!(store.delete_calls.load(Ordering::SeqCst), 0);
        assert!(store.value.lock().unwrap().is_some());
    });
}

#[test]
fn validation_that_finishes_after_logout_cannot_restore_the_session() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let store = Arc::new(FakeStore::default());
        *store.value.lock().unwrap() = Some(credential());
        let manager = manager(
            clock,
            qr.clone(),
            store.clone(),
            Arc::new(FakeSink::default()),
        );
        manager.restore_on_startup().await.unwrap();
        qr.block_validation.store(true, Ordering::SeqCst);

        let validating_manager = manager.clone();
        let validation =
            tokio::spawn(async move { validating_manager.validated_context().await.unwrap() });
        wait_for_condition(|| qr.validation_entered.load(Ordering::SeqCst)).await;
        manager.logout().await.unwrap();
        qr.validation_release.notify_waiters();
        let context = validation.await.unwrap();

        assert!(!context.is_authenticated());
        assert_eq!(manager.snapshot().status, AuthStatus::Anonymous);
        assert!(store.value.lock().unwrap().is_none());
    });
}

#[test]
fn credential_cleanup_failure_aborts_the_current_poll_generation() {
    tauri::async_runtime::block_on(async {
        let clock = Arc::new(FakeClock::new());
        let qr = Arc::new(FakeQrPort::new(clock.clone(), [], valid_account()));
        let manager = manager(
            clock,
            qr,
            Arc::new(FakeStore::default()),
            Arc::new(FakeSink::default()),
        );
        let (task_guard, task_dropped) = tokio::sync::oneshot::channel::<()>();
        let poll_task = tokio::spawn(async move {
            let _task_guard = task_guard;
            std::future::pending::<()>().await;
        });
        let previous_generation = {
            let mut state = manager.state.lock().unwrap();
            state.poll_task = Some(poll_task);
            state.generation
        };

        manager.commit_error_for_current_generation(AppError::internal("fixture cleanup failure"));

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.status, AuthStatus::Error);
        assert_eq!(
            manager.state.lock().unwrap().generation,
            previous_generation + 1
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), task_dropped)
                .await
                .expect("the current poll task should be aborted")
                .is_err()
        );
    });
}
