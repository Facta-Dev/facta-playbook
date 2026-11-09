//! Postgres adapter for `SessionRepository`. The ONLY place session SQL
//! lives; runtime `sqlx::query*` calls (no compile-time macros).
//!
//! SQL semantics mirror Go authsession queries 1:1 (CAS rotation, revoke
//! compare-and-swap, anomaly aggregates).
use crate::domain::{SessionRepository, SessionUser};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Error, MultiIpRow, Result, Role, Session, SessionId, UserId};
use sqlx::PgPool;

/// Concrete adapter. Cheap to clone — the pool is an `Arc` inside.
#[derive(Debug, Clone)]
pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
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

/// The joined row for `active_session_by_token` (session columns + the
/// account-state fields the token minter needs); the four state bools are the
/// real `users`/`sessions` columns (go parity), not states to re-model.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, sqlx::FromRow)]
struct SessionAuthRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    refresh_token: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    is_active: bool,
    expires_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    created_at: DateTime<Utc>,
    email: String,
    role: String,
    user_is_active: bool,
    is_blocked: bool,
    is_verified: bool,
}

impl From<SessionAuthRow> for SessionUser {
    fn from(r: SessionAuthRow) -> Self {
        Self {
            session: Session {
                id: SessionId(r.id),
                user_id: UserId(r.user_id),
                refresh_token: r.refresh_token,
                expires_at: r.expires_at,
                ip_address: r.ip_address,
                user_agent: r.user_agent,
                is_active: r.is_active,
                last_activity: r.last_activity,
                created_at: r.created_at,
            },
            email: r.email,
            role: match r.role.as_str() {
                "ADMIN" => Role::Admin,
                "MODERATOR" => Role::Editor,
                _ => Role::Viewer,
            },
            user_active: r.user_is_active,
            is_blocked: r.is_blocked,
            is_verified: r.is_verified,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    refresh_token: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    is_active: bool,
    last_activity: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<SessionRow> for Session {
    fn from(r: SessionRow) -> Self {
        Self {
            id: SessionId(r.id),
            user_id: UserId(r.user_id),
            refresh_token: r.refresh_token,
            expires_at: r.expires_at,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            is_active: r.is_active,
            last_activity: r.last_activity,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CountRow {
    n: i64,
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create_session(&self, session: &Session) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, refresh_token, ip_address, \
             user_agent, expires_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session.id.0)
        .bind(session.user_id.0)
        .bind(&session.refresh_token)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn active_session_by_token(&self, refresh_token: &str) -> Result<Option<SessionUser>> {
        let row = sqlx::query_as::<_, SessionAuthRow>(
            "SELECT s.id, s.user_id, s.refresh_token, s.ip_address, s.user_agent, \
             s.is_active, s.last_activity, s.expires_at, s.created_at, \
             u.email, u.role, \
             u.is_active AS user_is_active, u.is_blocked, u.is_verified \
             FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.refresh_token = $1 AND s.is_active AND s.expires_at > now()",
        )
        .bind(refresh_token)
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn deactivate_session(&self, session_id: SessionId) -> Result<bool> {
        let result =
            sqlx::query("UPDATE sessions SET is_active = false WHERE id = $1 AND is_active")
                .bind(session_id.0)
                .execute(&self.pool)
                .await
                .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn rotate_session(
        &self,
        session_id: SessionId,
        old_token: &str,
        new_refresh_token: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE sessions SET refresh_token = $3, ip_address = $4, \
             user_agent = $5, last_activity = now() \
             WHERE id = $1 AND refresh_token = $2 AND is_active \
             AND expires_at > now()",
        )
        .bind(session_id.0)
        .bind(old_token)
        .bind(new_refresh_token)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn deactivate_by_token_for_user(
        &self,
        user_id: UserId,
        refresh_token: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET is_active = false \
             WHERE refresh_token = $1 AND user_id = $2 AND is_active",
        )
        .bind(refresh_token)
        .bind(user_id.0)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn list_sessions_by_user(
        &self,
        user_id: UserId,
        active_only: bool,
    ) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, user_id, refresh_token, ip_address, user_agent, is_active, \
             last_activity, expires_at, created_at FROM sessions \
             WHERE user_id = $1 AND ($2 = false OR is_active) \
             ORDER BY last_activity DESC",
        )
        .bind(user_id.0)
        .bind(active_only)
        .fetch_all(&self.pool)
        .await
        .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn current_session(&self, user_id: UserId) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, user_id, refresh_token, ip_address, user_agent, is_active, \
             last_activity, expires_at, created_at FROM sessions \
             WHERE user_id = $1 AND is_active \
             ORDER BY last_activity DESC LIMIT 1",
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.map(Into::into))
    }

    async fn revoke_session_for_user(
        &self,
        user_id: UserId,
        session_id: SessionId,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE sessions SET is_active = false \
             WHERE user_id = $1 AND id = $2 AND is_active",
        )
        .bind(user_id.0)
        .bind(session_id.0)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(result.rows_affected() > 0)
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<i64> {
        let result =
            sqlx::query("UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active")
                .bind(user_id.0)
                .execute(&self.pool)
                .await
                .map_err(map)?;
        Ok(i64::try_from(result.rows_affected()).unwrap_or(i64::MAX))
    }

    async fn count_expired_active(&self) -> Result<i64> {
        let row = sqlx::query_as::<_, CountRow>(
            "SELECT count(*)::int8 AS n FROM sessions \
             WHERE is_active AND expires_at < now()",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map)?;
        Ok(row.n)
    }

    async fn multi_ip_sessions(&self, since: DateTime<Utc>) -> Result<Vec<MultiIpRow>> {
        let rows = sqlx::query_as::<_, MultiIpPgRow>(
            "SELECT user_id, count(DISTINCT ip_address)::int8 AS ip_count \
             FROM sessions WHERE is_active AND last_activity >= $1 \
             GROUP BY user_id HAVING count(DISTINCT ip_address) > 3",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

/// The multi-ip anomaly aggregate row.
#[derive(Debug, sqlx::FromRow)]
struct MultiIpPgRow {
    user_id: uuid::Uuid,
    ip_count: i64,
}

impl From<MultiIpPgRow> for MultiIpRow {
    fn from(r: MultiIpPgRow) -> Self {
        Self {
            user_id: UserId(r.user_id),
            ip_count: r.ip_count,
        }
    }
}
