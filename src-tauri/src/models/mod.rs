mod app_error;
mod audio;
mod auth;
mod parse;
mod settings;
mod task;

pub use app_error::{AppError, AppErrorCode};
pub use audio::AudioOutputProfile;
pub use auth::{AuthAccount, AuthSnapshot, AuthStateEvent, AuthStatus};
pub use parse::{
    AudioCapability, AudioFormat, MediaOption, ParseVideoResult, VideoCodec, VideoPart,
    VideoVariant,
};
pub use settings::{
    CloseBehavior, SettingsDocument, SettingsPatch, SettingsSnapshot, SettingsValues,
    ThemePreference, UpdateSettingRequest, VideoQualityId,
};
pub use task::{
    BatchTaskFailure, BatchTaskResult, ClearTasksResult, ControlDownloadTaskRequest,
    CreateDownloadTasksRequest, CreateTasksResult, DownloadMode, DownloadTask, DownloadTaskDraft,
    OpenDownloadTaskRequest, OpenTarget, TaskAction, TaskControlRequest, TaskListSnapshot,
    TaskProgressEvent, TaskRemovedEvent, TaskStatus,
};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub status: &'static str,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::{AppInfo, HealthStatus};
    use serde_json::json;

    #[test]
    fn app_info_serializes_to_the_frontend_contract() {
        let value = serde_json::to_value(AppInfo {
            name: "BiliCatch".into(),
            version: "0.1.0".into(),
        })
        .expect("AppInfo should serialize");

        assert_eq!(value, json!({ "name": "BiliCatch", "version": "0.1.0" }));
    }

    #[test]
    fn health_status_serializes_to_the_frontend_contract() {
        let value = serde_json::to_value(HealthStatus {
            status: "ok",
            timestamp: "2026-09-10T00:00:00Z".into(),
        })
        .expect("HealthStatus should serialize");

        assert_eq!(
            value,
            json!({ "status": "ok", "timestamp": "2026-09-10T00:00:00Z" })
        );
    }
}
