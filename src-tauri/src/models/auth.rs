use serde::Serialize;

use super::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    Restoring,
    Anonymous,
    Requesting,
    WaitingScan,
    WaitingConfirm,
    Authenticated,
    Expired,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthAccount {
    pub mid: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSnapshot {
    pub revision: u64,
    pub status: AuthStatus,
    pub qr_content: Option<String>,
    pub expires_at: Option<String>,
    pub account: Option<AuthAccount>,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStateEvent {
    pub snapshot: AuthSnapshot,
}
