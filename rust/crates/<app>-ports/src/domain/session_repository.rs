//! Repository port over the `sessions` table (Go `authsession.Store`
//! session half + `authoauth.CreateSession`).
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{MultiIpRow, Result, Role, Session, SessionId, UserId};

/// A session joined with the account-state fields needed to mint an access
/// token (Go `authsession.SessionUser`): rotation re-checks account state so
/// blocked/deactivated users cannot ride an existing session past refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUser {
    pub session: Session,
    pub email: String,
    pub role: Role,
    pub user_active: bool,
    pub is_blocked: bool,
    pub is_verified: bool,
}

/// Durable refresh-session contract. One row per issued refresh token; the
/// access token is a short-lived JWT and is never stored.
#[async_trait]
pub trait SessionRepository: Send + Sync + 'static {
    async fn create_session(&self, session: &Session) -> Result<()>;
    /// The active, unexpired session for a refresh token, joined with the
    /// account-state fields (Go `ActiveSessionByToken`).
    async fn active_session_by_token(&self, refresh_token: &str) -> Result<Option<SessionUser>>;
    /// Atomic claim of the refresh-row: `Ok(true)` claimed, `Ok(false)` when
    /// the session was already deactivated/revoked.
    async fn deactivate_session(&self, session_id: SessionId) -> Result<bool>;
    /// In-place CAS rotation (Go `RotateSession`): the row keeps its id and
    /// `expires_at` (absolute lifetime fixed at login); the WHERE guard on
    /// the old token rejects a replayed token. `Ok(false)` = lost the CAS.
    async fn rotate_session(
        &self,
        session_id: SessionId,
        old_token: &str,
        new_refresh_token: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<bool>;
    async fn deactivate_by_token_for_user(
        &self,
        user_id: UserId,
        refresh_token: &str,
    ) -> Result<()>;
    /// All (optionally active-only) sessions of a user, newest activity
    /// first (Go `ListSessionsByUser`, `$2 = false OR is_active` parity).
    async fn list_sessions_by_user(
        &self,
        user_id: UserId,
        active_only: bool,
    ) -> Result<Vec<Session>>;
    /// The user's most recent active session by `last_activity`
    /// (Go `CurrentSession`).
    async fn current_session(&self, user_id: UserId) -> Result<Option<Session>>;
    /// Compare-and-swap on `is_active` scoped to the owning user: revoking
    /// someone else's session id reads as `Ok(false)`.
    async fn revoke_session_for_user(&self, user_id: UserId, session_id: SessionId)
    -> Result<bool>;
    /// Deactivate every active session of the user; returns how many rows
    /// were claimed (Go `RevokeAllForUser`).
    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<i64>;
    /// Active rows past expiry (Go `CountExpiredActive`).
    async fn count_expired_active(&self) -> Result<i64>;
    /// Users whose active sessions since `since` span more than 3 distinct
    /// IPs (Go `MultiIPSessions`; NULL `ip_address` rows are not counted).
    async fn multi_ip_sessions(&self, since: DateTime<Utc>) -> Result<Vec<MultiIpRow>>;
}
