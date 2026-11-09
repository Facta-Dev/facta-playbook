//! Repository port over `email_verifications` + the signup profile seeding
//! writes (Go `authsignup.Store` set, copied exactly per feature).
use async_trait::async_trait;
use <app>_core::{Result, UserId, Verification};

/// Signup-side verification contract (Go `authsignup.Store` minus its user
/// reads, which live on [`crate::domain::UserRepository`]).
#[async_trait]
pub trait EmailVerificationRepository: Send + Sync + 'static {
    async fn create_verification(&self, verification: &Verification) -> Result<()>;
    /// Newest code for the user (Go `SignupLatestVerification`,
    /// `ORDER BY created_at DESC LIMIT 1`).
    async fn latest_verification(&self, user_id: UserId) -> Result<Option<Verification>>;
    /// Atomic single-use consume (Go `SignupUseVerification`): guards
    /// `is_used = false AND token = $2`; `Ok(false)` = raced/taken/wrong.
    async fn use_verification(&self, verification_id: uuid::Uuid, token: &str) -> Result<bool>;
    /// Record one wrong-code guess (Go `SignupBumpVerification`).
    async fn bump_verification(&self, verification_id: uuid::Uuid) -> Result<()>;
    /// Flip `is_verified` (Go `SignupSetVerified`).
    async fn set_email_verified(&self, user_id: UserId) -> Result<()>;
    /// Seed the 1:1 `companies` row, `user_id` PK (Go
    /// `SignupCreateCompanyProfile`).
    async fn create_company_profile(&self, user_id: UserId, company_name: &str) -> Result<()>;
    /// Seed the 1:1 `users_profile` row, `user_id` PK (Go
    /// `SignupCreateUserProfile`).
    async fn create_user_profile(&self, user_id: UserId, full_name: &str) -> Result<()>;
}
