//! Postgres adapter for `AuditLogRepository` (Go `adminusers.WriteAudit` +
//! the adminusers list/count views).
use crate::domain::{AuditLog, AuditLogRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Error, Result, UserId};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
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
struct AuditRow {
    id: uuid::Uuid,
    actor_user_id: Option<uuid::Uuid>,
    action: String,
    entity: String,
    entity_id: Option<String>,
    payload: Option<serde_json::Value>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<AuditRow> for AuditLog {
    fn from(r: AuditRow) -> Self {
        Self {
            id: r.id,
            actor_user_id: r.actor_user_id.map(UserId),
            action: r.action,
            entity: r.entity,
            entity_id: r.entity_id,
            payload: r.payload,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            created_at: r.created_at,
        }
    }
}
#[async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    async fn write_audit(
        &self,
        actor_id: Option<UserId>,
        action: &str,
        entity: &str,
        entity_id: Option<&str>,
        payload: Option<serde_json::Value>,
        ip: Option<&str>,
        ua: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_logs (id, actor_user_id, action, entity, entity_id, \
             payload, ip_address, user_agent) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(actor_id.map(|u| u.0))
        .bind(action)
        .bind(entity)
        .bind(entity_id)
        .bind(payload)
        .bind(ip)
        .bind(ua)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn list_audit(&self, limit: i64, offset: i64) -> Result<Vec<AuditLog>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT id, actor_user_id, action, entity, entity_id, payload, \
             ip_address, user_agent, created_at FROM audit_logs \
             ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_audit(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await
            .map_err(map)?;
        Ok(n)
    }
}
