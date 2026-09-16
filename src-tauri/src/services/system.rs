use serde::{Deserialize, Serialize};

use crate::models::{AppError, AppErrorCode, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available,
    Installing,
    Installed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMachine {
    state: UpdateState,
    available_version: Option<String>,
    platform: Option<String>,
    signature_verified: bool,
}

impl Default for UpdateMachine {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            available_version: None,
            platform: None,
            signature_verified: false,
        }
    }
}

impl UpdateMachine {
    pub fn state(&self) -> UpdateState {
        self.state
    }

    pub fn start_check(&mut self) -> UpdateState {
        if matches!(self.state, UpdateState::Checking | UpdateState::Installing) {
            return self.state;
        }
        self.state = UpdateState::Checking;
        self.signature_verified = false;
        self.state
    }

    pub fn finish_available(
        &mut self,
        version: impl Into<String>,
        platform: impl Into<String>,
        signature_verified: bool,
    ) -> UpdateState {
        self.available_version = Some(version.into());
        self.platform = Some(platform.into());
        self.signature_verified = signature_verified;
        self.state = UpdateState::Available;
        self.state
    }

    pub fn finish_up_to_date(&mut self) -> UpdateState {
        self.available_version = None;
        self.platform = None;
        self.signature_verified = false;
        self.state = UpdateState::UpToDate;
        self.state
    }

    pub fn fail(&mut self) -> UpdateState {
        self.state = UpdateState::Failed;
        self.state
    }

    pub fn begin_install(&mut self, confirmed: bool) -> UpdateState {
        if self.state != UpdateState::Available || !confirmed || !self.signature_verified {
            return self.state;
        }
        self.state = UpdateState::Installing;
        self.state
    }

    pub fn finish_install(&mut self, success: bool) -> UpdateState {
        self.state = if success {
            UpdateState::Installed
        } else {
            UpdateState::Failed
        };
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarManifest {
    pub name: String,
    pub platform: String,
    pub sha256: String,
    pub license_path: String,
}

impl SidecarManifest {
    pub fn validate(&self) -> Result<(), AppError> {
        let hash_valid =
            self.sha256.len() == 64 && self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
        if self.name.trim().is_empty()
            || self.platform.trim().is_empty()
            || !hash_valid
            || self.license_path.trim().is_empty()
        {
            return Err(AppError::new(
                AppErrorCode::E008,
                "The sidecar manifest is invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseArtifact {
    pub platform: String,
    pub target: String,
    pub version: String,
    pub signature_path: String,
}

impl ReleaseArtifact {
    pub fn validate(&self) -> Result<(), AppError> {
        let target_valid = matches!(
            self.target.as_str(),
            "nsis" | "msi" | "dmg" | "app" | "deb" | "rpm" | "appimage"
        );
        if self.platform.trim().is_empty()
            || !target_valid
            || self.version.trim().is_empty()
            || self.signature_path.trim().is_empty()
        {
            return Err(AppError::new(
                AppErrorCode::E008,
                "The release artifact is invalid",
            ));
        }
        Ok(())
    }
}

pub fn validate_release_set(artifacts: &[ReleaseArtifact]) -> Result<(), AppError> {
    if artifacts.is_empty() {
        return Err(AppError::new(
            AppErrorCode::E008,
            "The release artifact set is empty",
        ));
    }
    let version = artifacts[0].version.as_str();
    let mut keys = std::collections::HashSet::new();
    for artifact in artifacts {
        artifact.validate()?;
        if artifact.version != version
            || !keys.insert((artifact.platform.as_str(), artifact.target.as_str()))
        {
            return Err(AppError::new(
                AppErrorCode::E008,
                "The release artifact set is inconsistent",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDecision {
    MinimizeToTray,
    Exit,
    ConfirmExit,
}

impl WindowDecision {
    pub fn from_settings(minimize_to_tray: bool, has_active_tasks: bool) -> Self {
        match (minimize_to_tray, has_active_tasks) {
            (true, true) => Self::ConfirmExit,
            (true, false) => Self::MinimizeToTray,
            (false, _) => Self::Exit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemNotification {
    TaskCompleted { file_name: String },
    TaskFailed { file_name: String },
    UpdateAvailable { version: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCompletionNotification {
    pub title: String,
    pub body: String,
    pub play_sound: bool,
}

pub fn task_completion_notification(
    status: TaskStatus,
    file_name: &str,
    locale: &str,
    notify_on_complete: bool,
    completion_sound: bool,
) -> Option<TaskCompletionNotification> {
    if status != TaskStatus::Completed || !notify_on_complete {
        return None;
    }
    let (title, body) = if locale == "zh-CN" {
        ("下载完成".to_owned(), format!("{file_name} 已下载完成"))
    } else {
        (
            "Download completed".to_owned(),
            format!("{file_name} has finished downloading"),
        )
    };
    Some(TaskCompletionNotification {
        title,
        body,
        play_sound: completion_sound,
    })
}

pub fn map_task_notification(status: &str, file_name: &str) -> SystemNotification {
    if status.eq_ignore_ascii_case("failed") {
        SystemNotification::TaskFailed {
            file_name: file_name.to_owned(),
        }
    } else {
        SystemNotification::TaskCompleted {
            file_name: file_name.to_owned(),
        }
    }
}
