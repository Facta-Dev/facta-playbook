//! In-memory `SessionRepository` double — Go authsession SQL semantics in
//! memory (CAS rotation, revoke compare-and-swap, multi-ip anomaly).
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::domain::{SessionRepository, SessionUser};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{MultiIpRow, Result, Session, SessionId, User, UserId};

/// In-memory sessions + the joined users (the SQL JOIN, materialized).
#[derive(Debug, Default)]
pub struct MockSessionRepository {
    sessions: Mutex<HashMap<SessionId, Session>>,
    users: Mutex<HashMap<UserId, User>>,
}

impl MockSessionRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the `users` side of the JOIN.
    pub fn seed_user(&self, user: User) {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(user.id, user);
    }

    /// Seed one session row (replaces on id collision).
    pub fn seed_session(&self, session: Session) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(session.id, session);
    }
}
#[async_trait]
impl SessionRepository for MockSessionRepository {
    async fn create_session(&self, session: &Session) -> Result<()> {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id, session.clone());
        Ok(())
    }

    async fn active_session_by_token(&self, token: &str) -> Result<Option<SessionUser>> {
        let (sessions, users) = (self.sessions.lock().unwrap(), self.users.lock().unwrap());
        let Some(s) = sessions
            .values()
            .find(|s| s.refresh_token == token && s.is_active && s.expires_at > Utc::now())
        else {
            return Ok(None);
        };
        // JOIN semantics: no user row → no result.
        let Some(u) = users.get(&s.user_id) else {
            return Ok(None);
        };
        Ok(Some(SessionUser {
            session: s.clone(),
            email: u.email.clone(),
            role: u.role,
            user_active: u.is_active,
            is_blocked: u.is_blocked,
            is_verified: u.is_verified,
        }))
    }

    async fn deactivate_session(&self, session_id: SessionId) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();
        Ok(match sessions.get_mut(&session_id) {
            Some(s) if s.is_active => {
                s.is_active = false;
                true
            }
            _ => false,
        })
    }

    async fn rotate_session(
        &self,
        session_id: SessionId,
        old_token: &str,
        new_refresh_token: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();
        Ok(match sessions.get_mut(&session_id) {
            Some(s) if s.is_active && s.refresh_token == old_token && s.expires_at > Utc::now() => {
                new_refresh_token.clone_into(&mut s.refresh_token);
                s.ip_address = ip_address.map(str::to_owned);
                s.user_agent = user_agent.map(str::to_owned);
                s.last_activity = Utc::now();
                true
            }
            _ => false,
        })
    }

    async fn deactivate_by_token_for_user(&self, user_id: UserId, token: &str) -> Result<()> {
        self.sessions.lock().unwrap().values_mut().for_each(|s| {
            if s.user_id == user_id && s.refresh_token == token && s.is_active {
                s.is_active = false;
            }
        });
        Ok(())
    }

    async fn list_sessions_by_user(
        &self,
        user_id: UserId,
        active_only: bool,
    ) -> Result<Vec<Session>> {
        let mut rows: Vec<Session> = self
            .sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.user_id == user_id && (!active_only || s.is_active))
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            b.last_activity
                .cmp(&a.last_activity)
                .then(a.id.0.cmp(&b.id.0))
        });
        Ok(rows)
    }

    async fn current_session(&self, user_id: UserId) -> Result<Option<Session>> {
        let sessions = self.sessions.lock().unwrap();
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .max_by(|a, b| {
                a.last_activity
                    .cmp(&b.last_activity)
                    .then(a.id.0.cmp(&b.id.0))
            })
            .cloned())
    }

    async fn revoke_session_for_user(
        &self,
        user_id: UserId,
        session_id: SessionId,
    ) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();
        Ok(match sessions.get_mut(&session_id) {
            Some(s) if s.user_id == user_id && s.is_active => {
                s.is_active = false;
                true
            }
            _ => false,
        })
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<i64> {
        let n = self
            .sessions
            .lock()
            .unwrap()
            .values_mut()
            .filter(|s| s.user_id == user_id && s.is_active)
            .map(|s| {
                s.is_active = false;
            })
            .count();
        Ok(i64::try_from(n).unwrap_or(i64::MAX))
    }

    async fn count_expired_active(&self) -> Result<i64> {
        Ok(i64::try_from(
            self.sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| s.is_active && s.expires_at < Utc::now())
                .count(),
        )
        .unwrap_or(i64::MAX))
    }

    async fn multi_ip_sessions(&self, since: DateTime<Utc>) -> Result<Vec<MultiIpRow>> {
        // Active + within-window rows with a non-NULL ip, grouped per user
        // (the SQL GROUP BY), distinct-ip HAVING count > 3.
        let mut per_user: HashMap<UserId, Vec<String>> = HashMap::new();
        self.sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.is_active && s.last_activity >= since)
            .for_each(|s| {
                if let Some(ip) = &s.ip_address {
                    per_user.entry(s.user_id).or_default().push(ip.clone());
                }
            });
        Ok(per_user
            .into_iter()
            .filter_map(|(user_id, ips)| {
                let count = ips.iter().collect::<std::collections::HashSet<_>>().len();
                (count > 3).then_some(MultiIpRow {
                    user_id,
                    ip_count: i64::try_from(count).unwrap_or(i64::MAX),
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::test_user;
    use <app>_core::Role;

    const HOUR: chrono::Duration = chrono::Duration::hours(1);

    fn user(id: u64) -> User {
        test_user(id, "s@t.dev")
    }

    fn uid(n: u64) -> UserId {
        UserId(uuid::Uuid::from_u128(u128::from(n)))
    }

    fn sid(n: u64) -> SessionId {
        SessionId(uuid::Uuid::from_u128(u128::from(n)))
    }

    fn session(id: u64, uid: u64, token: &str, expires_at: chrono::DateTime<Utc>) -> Session {
        Session {
            id: SessionId(uuid::Uuid::from_u128(u128::from(id))),
            user_id: UserId(uuid::Uuid::from_u128(u128::from(uid))),
            refresh_token: token.to_owned(),
            expires_at,
            ip_address: None,
            user_agent: None,
            is_active: true,
            last_activity: Utc::now(),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn active_session_requires_token_active_unexpired_and_user_row() {
        let repo = MockSessionRepository::new();
        repo.seed_user(user(1));
        // Missing user → JOIN drops the row.
        let no_user = session(10, 42, "tok", Utc::now() + HOUR);
        repo.seed_session(no_user);
        assert!(repo.active_session_by_token("tok").await.unwrap().is_none());
        // Happy path.
        let s = session(11, 1, "tok1", Utc::now() + HOUR);
        repo.seed_session(s);
        let su = repo.active_session_by_token("tok1").await.unwrap().unwrap();
        assert_eq!(su.email, "s@t.dev");
        assert_eq!(su.role, Role::Viewer);
        assert!(su.user_active);
        assert_eq!(su.session.refresh_token, "tok1");
        // Inactive → None (is_active AND expiry guard).
        repo.deactivate_session(sid(11)).await.unwrap();
        assert!(
            repo.active_session_by_token("tok1")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn rotation_is_a_compare_and_swap_that_wins_once() {
        let repo = MockSessionRepository::new();
        repo.seed_user(user(1));
        let s = session(11, 1, "tok1", Utc::now() + HOUR);
        repo.seed_session(s);
        // Wrong old token → CAS fails.
        let ok = repo
            .rotate_session(sid(11), "other", "tok2", Some("2.2.2.2"), Some("ua2"))
            .await
            .unwrap();
        assert!(!ok);
        // Right token → CAS wins; id/expires/created stay, token/ip/ua/activity move.
        let ok = repo
            .rotate_session(sid(11), "tok1", "tok2", Some("2.2.2.2"), Some("ua2"))
            .await
            .unwrap();
        assert!(ok);
        let s = repo.sessions.lock().unwrap()[&SessionId(uuid::Uuid::from_u128(11))].clone();
        assert_eq!(s.refresh_token, "tok2");
        assert_eq!(s.ip_address.as_deref(), Some("2.2.2.2"));
        // Replay of the old token can only ever win once.
        let ok = repo
            .rotate_session(sid(11), "tok1", "tok3", Some("3.3.3.3"), None)
            .await
            .unwrap();
        assert!(!ok);
        assert_eq!(s.refresh_token, "tok2");
        // Expired session cannot rotate.
        let exp = session(12, 1, "e1", Utc::now() - HOUR);
        repo.seed_session(exp);
        let ok = repo
            .rotate_session(sid(12), "e1", "e2", None, None)
            .await
            .unwrap();
        assert!(!ok);
    }

    #[tokio::test]
    async fn revocation_is_scoped_and_counted() {
        let repo = MockSessionRepository::new();
        repo.seed_user(user(1));
        repo.seed_user(user(2));
        repo.seed_session(session(10, 1, "a1", Utc::now() + HOUR));
        repo.seed_session(session(11, 1, "a2", Utc::now() + HOUR));
        repo.seed_session(session(20, 2, "b1", Utc::now() + HOUR));
        // Revoking someone else's session id → false (scoped CAS).
        assert!(!repo.revoke_session_for_user(uid(2), sid(10)).await.unwrap());
        assert!(repo.revoke_session_for_user(uid(1), sid(10)).await.unwrap());
        // Already-revoked again → false (is_active guard).
        assert!(!repo.revoke_session_for_user(uid(1), sid(10)).await.unwrap());
        // Deactivate-by-token only hits the matching user's token row.
        repo.deactivate_by_token_for_user(uid(1), "a2")
            .await
            .unwrap();
        assert_eq!(repo.revoke_all_for_user(uid(1)).await.unwrap(), 0);
        assert_eq!(repo.revoke_all_for_user(uid(2)).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn expired_active_and_multi_ip_anomaly_semantics() {
        let repo = MockSessionRepository::new();
        repo.seed_user(user(1));
        repo.seed_user(user(2));
        // Expired-but-active rows are what the cleanup job counts.
        repo.seed_session(session(10, 1, "a1", Utc::now() - HOUR));
        repo.seed_session(session(11, 1, "a2", Utc::now() + HOUR));
        assert_eq!(repo.count_expired_active().await.unwrap(), 1);
        // list_sessions_by_user: active_only filter + last_activity order.
        let all = repo.list_sessions_by_user(uid(1), false).await.unwrap();
        assert_eq!(all.len(), 2);
        let active = repo.list_sessions_by_user(uid(1), true).await.unwrap();
        // is_active-only (the SQL has no expiry guard on the list) — the
        // expired-but-active row still lists.
        assert_eq!(active.len(), 2);
        assert!(active[0].is_active);
        // current = most recent active by last_activity.
        let mut newer = session(12, 1, "a3", Utc::now() + HOUR);
        newer.last_activity = Utc::now() + chrono::Duration::seconds(30);
        repo.seed_session(newer);
        let cur = repo.current_session(uid(1)).await.unwrap().unwrap();
        assert_eq!(cur.id, sid(12));

        // multi_ip: active + last_activity >= since, distinct non-null ips > 3.
        let base = Utc::now() - chrono::Duration::days(1);
        let mk = |id: u64, uid: u64, ip: &str| {
            let mut s = session(id, uid, "ip-tok", Utc::now() + HOUR);
            s.last_activity = base;
            s.ip_address = Some(ip.to_owned());
            s
        };
        repo.seed_session(mk(20, 1, "ip1"));
        repo.seed_session(mk(21, 1, "ip2"));
        repo.seed_session(mk(22, 1, "ip3"));
        // 3 distinct → no row (HAVING > 3).
        assert!(repo.multi_ip_sessions(base).await.unwrap().is_empty());
        repo.seed_session(mk(23, 1, "ip4"));
        let rows = repo.multi_ip_sessions(base).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_id, uid(1));
        assert_eq!(rows[0].ip_count, 4);
        // NULL ip rows are not counted (they cannot reach the threshold).
        let mut null_ip = session(24, 2, "n1", Utc::now() + HOUR);
        null_ip.last_activity = base;
        repo.seed_session(null_ip);
        // window: a row older than `since` drops out.
        let mut old = session(25, 1, "ip5", Utc::now() + HOUR);
        old.last_activity = base - chrono::Duration::seconds(1);
        old.ip_address = Some("ip9".to_owned());
        repo.seed_session(old);
        let rows = repo.multi_ip_sessions(base).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ip_count, 4);
    }
}
