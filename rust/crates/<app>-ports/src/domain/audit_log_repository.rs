//! Repository port over `audit_logs` (Go `adminusers.WriteAudit` + the
//! adminusers list/count views; append-only table).
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{Result, UserId};

/// One `audit_logs` row (Go writes it; the admin views read it back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLog {
    pub id: uuid::Uuid,
    pub actor_user_id: Option<UserId>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Append-only audit contract.
#[async_trait]
pub trait AuditLogRepository: Send + Sync + 'static {
    /// Record one audit entry; the actor may be unknown (`None` = NULL, Go
    /// `""` = NULL parity). Payload is JSONB.
    // Go `WriteAudit` passes one arg per column — the port keeps the 1:1
    // column list, hence the extra arg.
    #[allow(clippy::too_many_arguments)]
    async fn write_audit(
        &self,
        actor_id: Option<UserId>,
        action: &str,
        entity: &str,
        entity_id: Option<&str>,
        payload: Option<serde_json::Value>,
        ip: Option<&str>,
        ua: Option<&str>,
    ) -> Result<()>;
    /// One page of entries, newest first.
    async fn list_audit(&self, limit: i64, offset: i64) -> Result<Vec<AuditLog>>;
    /// Total entry count.
    async fn count_audit(&self) -> Result<i64>;
}
