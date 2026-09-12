use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppErrorCode {
    E001,
    E002,
    E003,
    E004,
    E005,
    E006,
    E007,
    E008,
    E009,
    E010,
    #[serde(rename = "E_INTERNAL")]
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(AppErrorCode::Internal, message)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, AppErrorCode};
    use serde_json::json;

    #[test]
    fn error_serializes_without_empty_details() {
        let value = serde_json::to_value(AppError {
            code: AppErrorCode::E003,
            message: "Invalid link".into(),
            details: None,
        })
        .expect("AppError should serialize");

        assert_eq!(value, json!({ "code": "E003", "message": "Invalid link" }));
    }

    #[test]
    fn internal_error_uses_the_stable_fallback_code() {
        let value = serde_json::to_value(AppError::internal("Unexpected error"))
            .expect("AppError should serialize");

        assert_eq!(
            value,
            json!({ "code": "E_INTERNAL", "message": "Unexpected error" })
        );
    }
}
