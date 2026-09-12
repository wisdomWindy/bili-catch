use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::models::{AppError, AppInfo, HealthStatus};

pub mod audio;
pub(crate) mod auth;
pub mod download;
pub mod download_runtime;
mod parser;
pub mod settings;
pub mod system;
pub mod tasks;
pub mod video;
pub(crate) use parser::ParserService;

pub fn app_info(name: &str, version: &str) -> Result<AppInfo, AppError> {
    if name.trim().is_empty() || version.trim().is_empty() {
        return Err(AppError::internal("Application metadata is unavailable"));
    }

    Ok(AppInfo {
        name: name.to_owned(),
        version: version.to_owned(),
    })
}

pub fn health_status() -> Result<HealthStatus, AppError> {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| AppError::internal("System clock is unavailable"))?;

    Ok(HealthStatus {
        status: "ok",
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::{app_info, health_status};

    #[test]
    fn app_info_rejects_missing_metadata() {
        let error = app_info("", "0.1.0").expect_err("blank names must fail");
        assert_eq!(error.message, "Application metadata is unavailable");
    }

    #[test]
    fn health_status_contains_an_rfc3339_timestamp() {
        let health = health_status().expect("health status should be available");
        assert_eq!(health.status, "ok");
        assert!(time::OffsetDateTime::parse(
            &health.timestamp,
            &time::format_description::well_known::Rfc3339
        )
        .is_ok());
    }
}
