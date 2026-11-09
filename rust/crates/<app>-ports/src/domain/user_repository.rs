//! Repository port over the `users` table (Go parity:
//! `authsignup`/`authsession`/`authpassword`/`authoauth`/`adminusers` Store
//! surfaces merged onto one port).
//!
//! The narrower Go per-slice projections collapse onto [`<app>_core::User`].
//! SQL lives only in the Pg adapters.
use async_trait::async_trait;
use <app>_core::{Result, Role, User, UserId};

/// Admin-side patch (Go `adminusers.Patch`): every field `None` = unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserPatch {
    pub is_active: Option<bool>,
    pub is_blocked: Option<bool>,
    /// `None` leaves the role untouched (Go `""` = untouched).
    pub role: Option<Role>,
}

/// Partial profile update (Go `UpdateMeInput`): `None` = unchanged;
/// `Some("")` sets the empty string (presence, not emptiness).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateMeInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub picture: Option<String>,
    pub language: Option<String>,
    pub address: Option<String>,
    pub postal: Option<String>,
}

/// CRUD/pipeline contract over user storage.
#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn get_user_by_id(&self, user_id: UserId) -> Result<Option<User>>;
    /// Full row by its (unique) `google_id` (Go `authoauth` parity).
    async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>>;
    /// Full row by its (unique) `github_id`.
    async fn get_user_by_github_id(&self, github_id: &str) -> Result<Option<User>>;
    /// INSERT a new user row (Go `SignupCreateUser` / oauth `CreateUser`):
    /// dup email/provider id reads as [`<app>_core::Error::Conflict`] on the
    /// Postgres adapter (unique violation), like Go's 23505 mapping.
    async fn create_user(&self, user: &User) -> Result<()>;
    /// Admin COALESCE patch (Go `adminusers.Store.Update`): `None` arms keep
    /// the current value; a missing id is a silent no-op (Go `:exec`).
    async fn update(&self, user_id: UserId, patch: &UserPatch) -> Result<()>;
    /// COALESCE profile update + re-read (Go `SessionUpdateMe`):
    /// [`None`] = leave unchanged, `Some("")` sets `''`; `Ok(None)` when the
    /// id is unknown (0 affected rows).
    async fn update_me(&self, user_id: UserId, patch: &UpdateMeInput) -> Result<Option<User>>;
    /// SQL-side increment + lock check (Go `SessionAfterFailedLogin`):
    /// `is_blocked = retry_count + 1 >= lock_at`. Unknown id is an error.
    async fn after_failed_login(&self, user_id: UserId, lock_at: i32) -> Result<bool>;
    /// Reset the retry counter and stamp `last_login`.
    async fn after_successful_login(&self, user_id: UserId) -> Result<()>;
    /// Attach the google id, verify the email, fill the picture only when
    /// currently empty (Go `authoauth.LinkGoogle` COALESCE/NULLIF parity).
    async fn link_google(&self, user_id: UserId, google_id: &str, picture: &str) -> Result<()>;
    /// Attach the github id (Go `LinkGithub` parity).
    async fn link_github(&self, user_id: UserId, github_id: &str, picture: &str) -> Result<()>;
    /// Link-flow write: attach the github id only, no verification side
    /// effects (Go `GithubOnly`).
    async fn github_only(&self, user_id: UserId, github_id: &str) -> Result<()>;
    /// Login-flow identity refresh: re-stamp email + verification +
    /// picture-if-empty (Go `SaveGithubLogin`).
    async fn save_github_login(&self, user_id: UserId, email: &str, picture: &str) -> Result<()>;
    /// One page of users, newest first (Go `AdminListUsers`).
    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>>;
    /// Total user row count (Go `AdminCountUsers`).
    async fn count_users(&self) -> Result<i64>;
    /// Bootstrap promotion: role + `user_type` move to ADMIN
    /// (Go `AdminUpgradeToAdmin`).
    async fn upgrade_to_admin(&self, user_id: UserId) -> Result<()>;
}
