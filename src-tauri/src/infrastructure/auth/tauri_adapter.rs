use tauri::{AppHandle, Emitter};

use crate::{
    models::{AppError, AuthStateEvent},
    services::auth::ports::AuthEventSink,
};

pub(crate) const AUTH_STATE_EVENT: &str = "auth://state";

pub(crate) struct TauriAuthEventSink {
    app: AppHandle,
}

impl TauriAuthEventSink {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl AuthEventSink for TauriAuthEventSink {
    fn emit(&self, event: AuthStateEvent) -> Result<(), AppError> {
        self.app
            .emit(AUTH_STATE_EVENT, event)
            .map_err(|_| AppError::internal("Unable to publish authentication state"))
    }
}

#[cfg(test)]
mod tests {
    use super::AUTH_STATE_EVENT;

    #[test]
    fn auth_event_name_is_stable() {
        assert_eq!(AUTH_STATE_EVENT, "auth://state");
    }
}
