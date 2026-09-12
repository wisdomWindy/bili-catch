use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    models::{AppError, AppInfo, HealthStatus},
    services::{self, ParserService},
};

pub(crate) mod auth;
pub(crate) mod settings;
pub(crate) mod tasks;

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo, AppError> {
    let package = app.package_info();
    services::app_info(&package.name, &package.version.to_string())
}

#[tauri::command]
pub fn health_check() -> Result<HealthStatus, AppError> {
    services::health_status()
}

#[tauri::command]
pub async fn parse_video(
    input: String,
    parser: State<'_, Arc<ParserService>>,
) -> Result<crate::models::ParseVideoResult, AppError> {
    parser.parse(&input).await
}
