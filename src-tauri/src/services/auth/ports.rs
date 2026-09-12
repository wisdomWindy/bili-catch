use std::time::{Duration, Instant};

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};

use crate::models::{AppError, AuthAccount, AuthStateEvent};

const ALLOWED_COOKIE_NAMES: [&str; 5] = [
    "DedeUserID",
    "DedeUserID__ckMd5",
    "SESSDATA",
    "bili_jct",
    "sid",
];

#[derive(Clone)]
pub(crate) struct StoredCredential {
    cookie_header: SecretString,
    refresh_token: Option<SecretString>,
}

impl StoredCredential {
    pub(crate) fn try_new(
        cookie_header: String,
        refresh_token: Option<String>,
    ) -> Result<Self, AppError> {
        let mut has_sessdata = false;
        let mut has_user_id = false;
        for item in cookie_header.split(';') {
            let Some((name, value)) = item.trim().split_once('=') else {
                return Err(AppError::internal("The login credential is incomplete"));
            };
            if value.trim().is_empty() || !ALLOWED_COOKIE_NAMES.contains(&name.trim()) {
                return Err(AppError::internal("The login credential is incomplete"));
            }
            has_sessdata |= name.trim() == "SESSDATA";
            has_user_id |= name.trim() == "DedeUserID";
        }
        if !has_sessdata || !has_user_id {
            return Err(AppError::internal("The login credential is incomplete"));
        }
        Ok(Self {
            cookie_header: cookie_header.into(),
            refresh_token: refresh_token.map(Into::into),
        })
    }

    pub(crate) fn expose_cookie_header(&self) -> &str {
        self.cookie_header.expose_secret()
    }

    pub(crate) fn expose_refresh_token(&self) -> Option<&str> {
        self.refresh_token
            .as_ref()
            .map(|token| token.expose_secret())
    }
}

pub(crate) struct GeneratedQr {
    pub(crate) qr_content: String,
    pub(crate) qrcode_key: SecretString,
}

pub(crate) enum QrPollResult {
    WaitingScan,
    WaitingConfirm,
    Confirmed(StoredCredential),
    Expired,
}

pub(crate) enum SessionValidation {
    Valid(AuthAccount),
    Invalid,
}

#[async_trait]
pub(crate) trait QrAuthPort: Send + Sync {
    async fn generate_qr(&self) -> Result<GeneratedQr, AppError>;
    async fn poll_qr(&self, qrcode_key: &SecretString) -> Result<QrPollResult, AppError>;
    async fn validate_session(
        &self,
        credential: &StoredCredential,
    ) -> Result<SessionValidation, AppError>;
}

#[async_trait]
pub(crate) trait CredentialStorePort: Send + Sync {
    async fn load(&self) -> Result<Option<StoredCredential>, AppError>;
    async fn save(&self, credential: &StoredCredential) -> Result<(), AppError>;
    async fn delete(&self) -> Result<(), AppError>;
}

pub(crate) trait AuthClock: Send + Sync {
    fn monotonic_elapsed(&self) -> Duration;
    fn utc_now(&self) -> time::OffsetDateTime;
}

pub(crate) struct SystemAuthClock {
    origin: Instant,
}

impl Default for SystemAuthClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl AuthClock for SystemAuthClock {
    fn monotonic_elapsed(&self) -> Duration {
        self.origin.elapsed()
    }

    fn utc_now(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::now_utc()
    }
}

#[async_trait]
pub(crate) trait AuthSleeper: Send + Sync {
    async fn sleep(&self, duration: Duration);
}

pub(crate) struct TokioAuthSleeper;

#[async_trait]
impl AuthSleeper for TokioAuthSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

pub(crate) trait AuthEventSink: Send + Sync {
    fn emit(&self, event: AuthStateEvent) -> Result<(), AppError>;
}
