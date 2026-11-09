//! Repository port over `password_resets` + the password writes + session
//! revocations (Go `authpassword.Store` set, copied exactly per feature).
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{PasswordReset, Result, SessionId, UserId};

/// Reset-side contract (Go `authpassword.Store` minus its user reads, which
/// live on [`crate::domain::UserRepository`]).
#[async_trait]
pub trait PasswordResetRepository: Send + Sync + 'static {
    /// Resets created for `email` since `since` (Go `CountResetsSince`).
    async fn count_resets_since(&self, email: &str, since: DateTime<Utc>) -> Result<i64>;
    async fn create_reset(&self, reset: &PasswordReset) -> Result<()>;
    /// Newest code for the email (Go `LatestReset`,
    /// `ORDER BY created_at DESC LIMIT 1`).
    async fn latest_reset(&self, email: &str) -> Result<Option<PasswordReset>>;
    /// Atomic single-use consume (Go `MarkResetUsed`): guards
    /// `is_used = false AND token = $2`; `Ok(false)` = raced/taken/wrong.
    async fn mark_reset_used(&self, reset_id: uuid::Uuid, token: &str) -> Result<bool>;
    /// Record one wrong-code guess (Go `BumpReset`).
    async fn bump_reset(&self, reset_id: uuid::Uuid) -> Result<()>;
    /// Write the password hash AND verify the email (Go
    /// `PassSetPasswordAndVerify`).
    async fn set_password_and_verify(&self, user_id: UserId, password_hash: &str) -> Result<()>;
    /// Write the password hash only (Go `PassSetPassword`).
    async fn set_password(&self, user_id: UserId, password_hash: &str) -> Result<()>;
    /// Deactivate every active session of the user (Go `RevokeAllSessions`).
    async fn revoke_all_sessions(&self, user_id: UserId) -> Result<()>;
    /// Deactivate every active session except `keep_session_id` (Go
    /// `RevokeOtherSessions`; the current session rides `jti == sessions.id`).
    async fn revoke_other_sessions(
        &self,
        user_id: UserId,
        keep_session_id: SessionId,
    ) -> Result<()>;
}
