//! Postgres adapter for `PasswordResetRepository` (Go authpassword SQL
//! semantics 1:1).
use crate::domain::PasswordResetRepository;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Error, PasswordReset, Result, SessionId, UserId};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgPasswordResetRepository {
    pool: PgPool,
}

impl PgPasswordResetRepository {
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
struct ResetRow {
    id: uuid::Uuid,
    user_id: Option<uuid::Uuid>,
    email: String,
    token: String,
    expires_at: DateTime<Utc>,
    is_used: bool,
    attempts: i32,
}

impl From<ResetRow> for PasswordReset {
    fn from(r: ResetRow) -> Self {
        Self {
            id: r.id,
            user_id: UserId(r.user_id.unwrap_or_default()),
            email: r.email,
            token: r.token,
            expires_at: r.expires_at,
            used: r.is_used,
            attempts: r.attempts,
        }
    }
}

#[async_trait]
impl PasswordResetRepository for PgPasswordResetRepository {
    async fn count_resets_since(&self, email: &str, since: DateTime<Utc>) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM password_resets WHERE email = $1 AND created_at >= $2",
        )
        .bind(email)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(map)?;
        Ok(n)
    }

    async fn create_reset(&self, reset: &PasswordReset) -> Result<()> {
        sqlx::query(
            "INSERT INTO password_resets (id, user_id, email, token, expires_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(reset.id)
        .bind(reset.user_id.0)
        .bind(&reset.email)
        .bind(&reset.token)
        .bind(reset.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn latest_reset(&self, email: &str) -> Result<Option<PasswordReset>> {
        let row = sqlx::query_as::<_, ResetRow>(
            "SELECT id, user_id, email, token, expires_at, is_used, attempts \
             FROM password_resets WHERE email = $1 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn mark_reset_used(&self, reset_id: uuid::Uuid, token: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE password_resets SET is_used = true \
             WHERE id = $1 AND is_used = false AND token = $2",
        )
        .bind(reset_id)
        .bind(token)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn bump_reset(&self, reset_id: uuid::Uuid) -> Result<()> {
        sqlx::query("UPDATE password_resets SET attempts = attempts + 1 WHERE id = $1")
            .bind(reset_id)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn set_password_and_verify(&self, user_id: UserId, password_hash: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password = $2, is_verified = true, updated_at = now() \
             WHERE id = $1",
        )
        .bind(user_id.0)
        .bind(password_hash)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn set_password(&self, user_id: UserId, password_hash: &str) -> Result<()> {
        sqlx::query("UPDATE users SET password = $2, updated_at = now() WHERE id = $1")
            .bind(user_id.0)
            .bind(password_hash)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn revoke_all_sessions(&self, user_id: UserId) -> Result<()> {
        sqlx::query("UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active")
            .bind(user_id.0)
            .execute(&self.pool)
            .await
            .map_err(map)?;
        Ok(())
    }

    async fn revoke_other_sessions(
        &self,
        user_id: UserId,
        keep_session_id: SessionId,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET is_active = false \
             WHERE user_id = $1 AND is_active AND id <> $2",
        )
        .bind(user_id.0)
        .bind(keep_session_id.0)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }
}
