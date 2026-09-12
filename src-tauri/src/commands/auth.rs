use std::sync::Arc;

use tauri::State;

use crate::{
    models::{AppError, AuthSnapshot},
    services::auth::AuthManager,
};

#[tauri::command]
pub(crate) fn get_auth_snapshot(
    manager: State<'_, Arc<AuthManager>>,
) -> Result<AuthSnapshot, AppError> {
    Ok(manager.snapshot())
}

#[tauri::command]
pub(crate) async fn start_qr_login(
    manager: State<'_, Arc<AuthManager>>,
) -> Result<AuthSnapshot, AppError> {
    manager.inner().start_login().await
}

#[tauri::command]
pub(crate) fn cancel_qr_login(
    manager: State<'_, Arc<AuthManager>>,
) -> Result<AuthSnapshot, AppError> {
    manager.cancel_login()
}

#[tauri::command]
pub(crate) async fn logout(manager: State<'_, Arc<AuthManager>>) -> Result<AuthSnapshot, AppError> {
    manager.logout().await
}
