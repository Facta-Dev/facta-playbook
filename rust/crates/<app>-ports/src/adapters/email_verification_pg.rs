//! Postgres adapter for `EmailVerificationRepository` + the signup profile
//! seeding writes (Go authsignup SQL semantics 1:1).
use crate::domain::EmailVerificationRepository;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Error, Result, UserId, Verification};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgEmailVerificationRepository {
    pool: PgPool,
}

impl PgEmailVerificationRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Map the foreign error into the domain error at the boundary.
fn map(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound,
        other => Error::Internal(other.to_string()),
    }
}

#[derive(Debug, sqlx::FromRow)]
struct VerificationRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    token: String,
    expires_at: DateTime<Utc>,
    is_used: bool,
    attempts: i32,
}

impl From<VerificationRow> for Verification {
    fn from(r: VerificationRow) -> Self {
        Self {
            id: r.id,
            user_id: UserId(r.user_id),
            token: r.token,
            expires_at: r.expires_at,
            used: r.is_used,
            attempts: r.attempts,
        }
    }
}
#[async_trait]
impl EmailVerificationRepository for PgEmailVerificationRepository {
    async fn create_verification(&self, verification: &Verification) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_verifications (id, user_id, token, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(verification.id)
        .bind(verification.user_id.0)
        .bind(&verification.token)
        .bind(verification.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn latest_verification(&self, user_id: UserId) -> Result<Option<Verification>> {
        let row = sqlx::query_as::<_, VerificationRow>(
            "SELECT id, user_id, token, expires_at, is_used, attempts \
             FROM email_verifications WHERE user_id = $1 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn use_verification(&self, verification_id: uuid::Uuid, token: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE email_verifications SET is_used = true \
             WHERE id = $1 AND is_used = false AND token = $2",
        )
        .bind(verification_id)
        .bind(token)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn bump_verification(&self, verification_id: uuid::Uuid) -> Result<()> {
        sqlx::query("UPDATE email_verifications SET attempts = attempts + 1 WHERE id = $1")
            .bind(verification_id)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn set_email_verified(&self, user_id: UserId) -> Result<()> {
        sqlx::query("UPDATE users SET is_verified = true, updated_at = now() WHERE id = $1")
            .bind(user_id.0)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn create_company_profile(&self, user_id: UserId, company_name: &str) -> Result<()> {
        sqlx::query("INSERT INTO companies (user_id, company_name) VALUES ($1, $2)")
            .bind(user_id.0)
            .bind(company_name)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn create_user_profile(&self, user_id: UserId, full_name: &str) -> Result<()> {
        sqlx::query("INSERT INTO users_profile (user_id, full_name) VALUES ($1, $2)")
            .bind(user_id.0)
            .bind(full_name)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }
}
