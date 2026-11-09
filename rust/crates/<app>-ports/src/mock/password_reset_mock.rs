//! In-memory `PasswordResetRepository` double — Go authpassword SQL
//! semantics in memory.
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::domain::PasswordResetRepository;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{PasswordReset, Result, Session, SessionId, User, UserId};

/// One stored reset row with its DB default `created_at` stamp.
#[derive(Debug, Clone)]
struct ResetEntry {
    reset: PasswordReset,
    created_at: DateTime<Utc>,
}

/// In-memory `password_resets` (insertion order = arrival order).
#[derive(Debug, Default)]
pub struct MockPasswordResetRepository {
    resets: Mutex<Vec<ResetEntry>>,
    /// `users` side of the password UPDATEs.
    users: Mutex<HashMap<UserId, User>>,
    /// `sessions` side of the revocation UPDATEs.
    sessions: Mutex<HashMap<SessionId, Session>>,
}

impl MockPasswordResetRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the `users` side.
    pub fn seed_user(&self, user: User) {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(user.id, user);
    }

    /// Seed the `sessions` side of the revocation writes.
    pub fn seed_session(&self, session: Session) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(session.id, session);
    }
}
#[async_trait]
impl PasswordResetRepository for MockPasswordResetRepository {
    async fn count_resets_since(&self, email: &str, since: DateTime<Utc>) -> Result<i64> {
        Ok(i64::try_from(
            self.resets
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.reset.email == email && e.created_at >= since)
                .count(),
        )
        .unwrap_or(i64::MAX))
    }

    async fn create_reset(&self, reset: &PasswordReset) -> Result<()> {
        self.resets.lock().unwrap().push(ResetEntry {
            reset: reset.clone(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    async fn latest_reset(&self, email: &str) -> Result<Option<PasswordReset>> {
        Ok(self
            .resets
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|e| e.reset.email == email)
            .map(|e| e.reset.clone()))
    }

    async fn mark_reset_used(&self, id: uuid::Uuid, token: &str) -> Result<bool> {
        let mut resets = self.resets.lock().unwrap();
        Ok(match resets.iter_mut().find(|e| e.reset.id == id) {
            Some(e) if !e.reset.used && e.reset.token == token => {
                e.reset.used = true;
                true
            }
            _ => false,
        })
    }

    async fn bump_reset(&self, id: uuid::Uuid) -> Result<()> {
        if let Some(e) = self
            .resets
            .lock()
            .unwrap()
            .iter_mut()
            .find(|e| e.reset.id == id)
        {
            e.reset.attempts += 1;
        }
        Ok(())
    }

    async fn set_password_and_verify(&self, user_id: UserId, hash: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            hash.clone_into(&mut u.password);
            u.is_verified = true;
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn set_password(&self, user_id: UserId, hash: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            hash.clone_into(&mut u.password);
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn revoke_all_sessions(&self, user_id: UserId) -> Result<()> {
        self.sessions.lock().unwrap().values_mut().for_each(|s| {
            if s.user_id == user_id && s.is_active {
                s.is_active = false;
            }
        });
        Ok(())
    }

    async fn revoke_other_sessions(&self, user_id: UserId, keep: SessionId) -> Result<()> {
        self.sessions.lock().unwrap().values_mut().for_each(|s| {
            if s.user_id == user_id && s.is_active && s.id != keep {
                s.is_active = false;
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::test_user;

    fn uid(n: u64) -> UserId {
        UserId(uuid::Uuid::from_u128(u128::from(n)))
    }

    fn sid(n: u64) -> SessionId {
        SessionId(uuid::Uuid::from_u128(u128::from(n)))
    }

    fn user(id: u64) -> User {
        test_user(id, "r@t.dev")
    }

    fn session(id: u64, u: u64, token: &str) -> Session {
        Session {
            id: sid(id),
            user_id: uid(u),
            refresh_token: token.to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            ip_address: None,
            user_agent: None,
            is_active: true,
            last_activity: Utc::now(),
            created_at: Utc::now(),
        }
    }

    fn reset(id: u64, user_id: UserId, token: &str) -> PasswordReset {
        PasswordReset {
            id: uuid::Uuid::from_u128(u128::from(id)),
            user_id,
            email: "reset@t.dev".to_owned(),
            token: token.to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            used: false,
            attempts: 0,
        }
    }

    #[tokio::test]
    async fn reset_window_latest_and_single_use_cas() {
        let repo = MockPasswordResetRepository::new();
        // count_resets_since counts within the window for that email only.
        repo.create_reset(&reset(100, uid(1), "tok1"))
            .await
            .unwrap();
        repo.create_reset(&reset(101, uid(1), "tok2"))
            .await
            .unwrap();
        assert_eq!(
            repo.count_resets_since("reset@t.dev", Utc::now() - chrono::Duration::hours(1))
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            repo.count_resets_since("reset@t.dev", Utc::now() + chrono::Duration::days(1))
                .await
                .unwrap(),
            0
        );
        // latest_reset: newest row for the email.
        let latest = repo.latest_reset("reset@t.dev").await.unwrap().unwrap();
        assert_eq!(latest.id, uuid::Uuid::from_u128(101));
        // mark_reset_used is a token CAS: wins once, dies on replay or wrong token.
        assert!(
            repo.mark_reset_used(uuid::Uuid::from_u128(101), "tok2")
                .await
                .unwrap()
        );
        assert!(
            !repo
                .mark_reset_used(uuid::Uuid::from_u128(101), "tok2")
                .await
                .unwrap()
        );
        assert!(
            !repo
                .mark_reset_used(uuid::Uuid::from_u128(100), "tok2")
                .await
                .unwrap()
        );
        assert!(
            repo.mark_reset_used(uuid::Uuid::from_u128(100), "tok1")
                .await
                .unwrap()
        );
        // bump counts failed attempts (unconditional UPDATE, Go parity).
        repo.bump_reset(uuid::Uuid::from_u128(101)).await.unwrap();
        let r = repo.latest_reset("reset@t.dev").await.unwrap().unwrap();
        assert_eq!(r.attempts, 1);
        // Different email is isolated.
        assert!(repo.latest_reset("other@t.dev").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn password_updates_and_session_revocations() {
        let repo = MockPasswordResetRepository::new();
        repo.seed_user(user(1));
        repo.seed_session(session(10, 1, "keep"));
        repo.seed_session(session(11, 1, "drop"));
        repo.seed_session(session(20, 2, "unrelated"));
        // set_password_and_verify flips is_verified along with password.
        repo.set_password_and_verify(uid(1), "argon2-hash-1")
            .await
            .unwrap();
        let u = repo.users.lock().unwrap()[&uid(1)].clone();
        assert!(u.is_verified);
        assert_eq!(u.password, "argon2-hash-1");
        // set_password leaves verification alone.
        repo.set_password(uid(1), "argon2-hash-2").await.unwrap();
        let u = repo.users.lock().unwrap()[&uid(1)].clone();
        assert_eq!(u.password, "argon2-hash-2");
        // revoke_all_sessions deactivates every active session of the user.
        repo.revoke_all_sessions(uid(1)).await.unwrap();
        assert!(!repo.sessions.lock().unwrap()[&sid(10)].is_active);
        assert!(!repo.sessions.lock().unwrap()[&sid(11)].is_active);
        assert!(repo.sessions.lock().unwrap()[&sid(20)].is_active);
        // revoke_other_sessions keeps only the named session.
        // revive the two revoked rows, then keep only session 10 alive.
        {
            let mut sessions = repo.sessions.lock().unwrap();
            sessions.get_mut(&sid(10)).unwrap().is_active = true;
            sessions.get_mut(&sid(11)).unwrap().is_active = true;
        }
        repo.revoke_other_sessions(uid(1), sid(10)).await.unwrap();
        assert!(repo.sessions.lock().unwrap()[&sid(10)].is_active);
        assert!(!repo.sessions.lock().unwrap()[&sid(11)].is_active);
        assert!(repo.sessions.lock().unwrap()[&sid(20)].is_active);
    }
}
