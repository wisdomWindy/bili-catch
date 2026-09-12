use std::sync::Arc;

use tauri::State;

use crate::{
    models::{AppError, SettingsSnapshot, UpdateSettingRequest},
    services::settings::SettingsManager,
};

#[tauri::command]
pub fn get_settings_snapshot(manager: State<'_, Arc<SettingsManager>>) -> SettingsSnapshot {
    manager.snapshot()
}

#[tauri::command]
pub fn update_setting(
    request: UpdateSettingRequest,
    manager: State<'_, Arc<SettingsManager>>,
) -> Result<SettingsSnapshot, AppError> {
    manager.update(request.patch)
}
