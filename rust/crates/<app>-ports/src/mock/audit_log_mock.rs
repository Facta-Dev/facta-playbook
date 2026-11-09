//! In-memory `AuditLogRepository` double — append + newest-first list.
use std::sync::{Mutex, PoisonError};

use crate::domain::{AuditLog, AuditLogRepository};
use async_trait::async_trait;
use <app>_core::{Result, UserId};
/// In-memory `audit_logs` (append-only; newest first on list).
#[derive(Debug, Default)]
pub struct MockAuditLogRepository {
    entries: Mutex<Vec<AuditLog>>,
}

impl MockAuditLogRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-back helper for tests (insertion order).
    pub fn all(&self) -> Vec<AuditLog> {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl AuditLogRepository for MockAuditLogRepository {
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
        self.entries.lock().unwrap().push(AuditLog {
            id: uuid::Uuid::now_v7(),
            actor_user_id: actor_id,
            action: action.to_owned(),
            entity: entity.to_owned(),
            entity_id: entity_id.map(str::to_owned),
            payload,
            ip_address: ip.map(str::to_owned),
            user_agent: ua.map(str::to_owned),
            created_at: chrono::Utc::now(),
        });
        Ok(())
    }

    async fn list_audit(&self, limit: i64, offset: i64) -> Result<Vec<AuditLog>> {
        // ORDER BY created_at DESC — newest first; the double uses reverse
        // arrival order (all stamps come from one clock, now_v7 ids tiebreak).
        let entries = self.entries.lock().unwrap();
        let skip = usize::try_from(offset.max(0)).unwrap_or(usize::MAX);
        let take = usize::try_from(limit.max(0)).unwrap_or(usize::MAX);
        Ok(entries
            .iter()
            .rev()
            .skip(skip)
            .take(take)
            .cloned()
            .collect())
    }

    async fn count_audit(&self) -> Result<i64> {
        Ok(i64::try_from(self.entries.lock().unwrap().len()).unwrap_or(i64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(n: u64) -> UserId {
        UserId(uuid::Uuid::from_u128(u128::from(n)))
    }

    #[tokio::test]
    async fn audit_write_list_newest_first_count_and_paging() {
        let repo = MockAuditLogRepository::new();
        repo.write_audit(
            Some(uid(7)),
            "user.update",
            "user",
            Some("u-7"),
            Some(serde_json::json!({"k": "v"})),
            Some("1.1.1.1"),
            Some("ua"),
        )
        .await
        .unwrap();
        repo.write_audit(None, "session.revoke", "session", None, None, None, None)
            .await
            .unwrap();
        // Newest first: the revocation (2nd write) lists first.
        let rows = repo.list_audit(10, 0).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].action, "session.revoke");
        assert_eq!(rows[1].action, "user.update");
        assert_eq!(rows[1].actor_user_id, Some(uid(7)));
        assert_eq!(rows[1].entity_id.as_deref(), Some("u-7"));
        assert_eq!(rows[1].payload.as_ref().unwrap()["k"], "v");
        assert_eq!(rows[0].actor_user_id, None);
        // count + paging.
        assert_eq!(repo.count_audit().await.unwrap(), 2);
        let page = repo.list_audit(1, 1).await.unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].action, "user.update");
    }
}
