//! In-memory `EmailVerificationRepository` double — Go authsignup SQL
//! semantics in memory. Profile writes are recorded as append-only lists so
//! tests can observe them.
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::domain::EmailVerificationRepository;
use async_trait::async_trait;
use <app>_core::{Result, User, UserId, Verification};

/// In-memory `email_verifications` (insertion order = arrival order).
#[derive(Debug, Default)]
pub struct MockEmailVerificationRepository {
    verifications: Mutex<Vec<Verification>>,
    /// `users` side of `set_email_verified` (UPDATE users).
    users: Mutex<HashMap<UserId, User>>,
    /// Recorded `INSERT INTO companies` writes.
    companies: Mutex<Vec<UserId>>,
    /// Recorded `INSERT INTO users_profile` writes.
    profiles: Mutex<Vec<UserId>>,
}

impl MockEmailVerificationRepository {
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

    /// Seed one verification row (appends; arrival order matters).
    pub fn seed_verification(&self, verification: Verification) {
        self.verifications
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(verification);
    }
}
#[async_trait]
impl EmailVerificationRepository for MockEmailVerificationRepository {
    async fn create_verification(&self, verification: &Verification) -> Result<()> {
        self.verifications
            .lock()
            .unwrap()
            .push(verification.clone());
        Ok(())
    }

    async fn latest_verification(&self, user_id: UserId) -> Result<Option<Verification>> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|v| v.user_id == user_id)
            .cloned())
    }

    async fn use_verification(&self, id: uuid::Uuid, token: &str) -> Result<bool> {
        let mut verifications = self.verifications.lock().unwrap();
        // Single-use CAS: `WHERE id AND is_used = false AND token`.
        Ok(match verifications.iter_mut().find(|v| v.id == id) {
            Some(v) if !v.used && v.token == token => {
                v.used = true;
                true
            }
            _ => false,
        })
    }

    async fn bump_verification(&self, id: uuid::Uuid) -> Result<()> {
        // Unconditional `attempts = attempts + 1 WHERE id` (no error on miss).
        if let Some(v) = self
            .verifications
            .lock()
            .unwrap()
            .iter_mut()
            .find(|v| v.id == id)
        {
            v.attempts += 1;
        }
        Ok(())
    }

    async fn set_email_verified(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.is_verified = true;
        }
        Ok(())
    }

    async fn create_company_profile(&self, user_id: UserId, _company_name: &str) -> Result<()> {
        self.companies.lock().unwrap().push(user_id);
        Ok(())
    }

    async fn create_user_profile(&self, user_id: UserId, _full_name: &str) -> Result<()> {
        self.profiles.lock().unwrap().push(user_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::test_user;
    use chrono::Utc;

    fn uid(n: u64) -> UserId {
        UserId(uuid::Uuid::from_u128(u128::from(n)))
    }

    fn verification(
        id: u64,
        user_id: UserId,
        token: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Verification {
        Verification {
            id: uuid::Uuid::from_u128(u128::from(id)),
            user_id,
            token: token.to_owned(),
            expires_at,
            used: false,
            attempts: 0,
        }
    }

    #[tokio::test]
    async fn verification_is_single_use_by_token_cas() {
        let repo = MockEmailVerificationRepository::new();
        repo.seed_user(test_user(1, "v@t.dev"));
        repo.seed_verification(verification(
            100,
            uid(1),
            "tok1",
            Utc::now() + chrono::Duration::hours(1),
        ));
        // First consume wins.
        assert!(
            repo.use_verification(uuid::Uuid::from_u128(100), "tok1")
                .await
                .unwrap()
        );
        // Replay dies on is_used = false.
        assert!(
            !repo
                .use_verification(uuid::Uuid::from_u128(100), "tok1")
                .await
                .unwrap()
        );
        // Wrong token dies on token = $2.
        repo.seed_verification(verification(
            101,
            uid(1),
            "tok2",
            Utc::now() + chrono::Duration::hours(1),
        ));
        assert!(
            !repo
                .use_verification(uuid::Uuid::from_u128(101), "tok1")
                .await
                .unwrap()
        );
        // Attempts bump unconditionally (Go BumpVerification).
        repo.bump_verification(uuid::Uuid::from_u128(101))
            .await
            .unwrap();
        repo.bump_verification(uuid::Uuid::from_u128(100))
            .await
            .unwrap();
        let v = repo.latest_verification(uid(1)).await.unwrap().unwrap();
        assert_eq!(v.id, uuid::Uuid::from_u128(101));
        assert_eq!(v.attempts, 1);
        assert!(!v.used);
        // latest returns the newest row for the user.
        repo.seed_verification(verification(
            102,
            uid(1),
            "tok3",
            Utc::now() + chrono::Duration::hours(1),
        ));
        let v2 = repo.latest_verification(uid(1)).await.unwrap().unwrap();
        assert_eq!(v2.id, uuid::Uuid::from_u128(102));
        // set_email_verified flips the user flag.
        repo.set_email_verified(uid(1)).await.unwrap();
        let u = repo.users.lock().unwrap()[&uid(1)].clone();
        assert!(u.is_verified);
        // Profile writes are recorded (signup profile seeding).
        repo.create_company_profile(uid(1), "acme").await.unwrap();
        repo.create_user_profile(uid(1), "Full Name").await.unwrap();
        assert_eq!(repo.companies.lock().unwrap().len(), 1);
        assert_eq!(repo.profiles.lock().unwrap().len(), 1);
    }
}
