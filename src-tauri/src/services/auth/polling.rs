use std::{sync::Arc, time::Duration};

use secrecy::SecretString;

use crate::models::{AppError, AuthStatus};

use super::{
    manager::AuthManager,
    ports::{QrPollResult, SessionValidation, StoredCredential},
};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

impl AuthManager {
    pub(super) async fn poll_until_terminal(
        self: &Arc<Self>,
        generation: u64,
        qrcode_key: SecretString,
        deadline: Duration,
    ) {
        loop {
            self.sleeper.sleep(POLL_INTERVAL).await;
            if !self.is_current(generation) {
                return;
            }
            if self.clock.monotonic_elapsed() >= deadline {
                self.commit_terminal(generation, AuthStatus::Expired, None);
                return;
            }

            match self.qr.poll_qr(&qrcode_key).await {
                Ok(QrPollResult::WaitingScan) => {
                    self.commit_waiting_status(generation, AuthStatus::WaitingScan)
                }
                Ok(QrPollResult::WaitingConfirm) => {
                    self.commit_waiting_status(generation, AuthStatus::WaitingConfirm)
                }
                Ok(QrPollResult::Expired) => {
                    self.commit_terminal(generation, AuthStatus::Expired, None);
                    return;
                }
                Ok(QrPollResult::Confirmed(credential)) => {
                    self.complete_login(generation, deadline, credential).await;
                    return;
                }
                Err(error) => {
                    self.commit_error(generation, error);
                    return;
                }
            }
        }
    }

    async fn complete_login(
        &self,
        generation: u64,
        deadline: Duration,
        credential: StoredCredential,
    ) {
        if !self.ensure_active_before_deadline(generation, deadline) {
            return;
        }
        let account = match self.qr.validate_session(&credential).await {
            Ok(SessionValidation::Valid(account)) => account,
            Ok(SessionValidation::Invalid) => {
                self.commit_error(
                    generation,
                    AppError::internal("Bilibili rejected the login credential"),
                );
                return;
            }
            Err(error) => {
                self.commit_error(generation, error);
                return;
            }
        };

        let Ok(_permit) = self.credential_gate.acquire().await else {
            self.commit_error(
                generation,
                AppError::internal("Credential operations are unavailable"),
            );
            return;
        };
        if !self.ensure_active_before_deadline(generation, deadline) {
            return;
        }
        if let Err(error) = self.store.save(&credential).await {
            self.commit_error(generation, error);
            return;
        }
        if self.is_current(generation) && self.clock.monotonic_elapsed() < deadline {
            self.commit_authenticated(generation, credential, account);
            return;
        }

        if let Err(error) = self.store.delete().await {
            self.commit_error_for_current_generation(error);
        } else if self.is_current(generation) {
            self.commit_terminal(generation, AuthStatus::Expired, None);
        }
    }

    fn ensure_active_before_deadline(&self, generation: u64, deadline: Duration) -> bool {
        if !self.is_current(generation) {
            return false;
        }
        if self.clock.monotonic_elapsed() >= deadline {
            self.commit_terminal(generation, AuthStatus::Expired, None);
            return false;
        }
        true
    }
}
