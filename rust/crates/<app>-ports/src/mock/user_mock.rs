//! In-memory `UserRepository` double — Go-fake parity for the authsignup /
//! authsession / authoauth / adminusers SQL semantics.
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::domain::{UpdateMeInput, UserPatch, UserRepository};
use async_trait::async_trait;
use chrono::Utc;
use <app>_core::{Error, Result, User, UserId};

/// In-memory users table (exact-match lookups like the SQL UNIQUE columns).
#[derive(Debug, Default)]
pub struct MockUserRepository {
    users: Mutex<HashMap<UserId, User>>,
}

impl MockUserRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed one row (replaces on id collision).
    pub fn seed(&self, user: User) {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(user.id, user);
    }
}

fn keep_picture(current: Option<&str>, incoming: &str) -> String {
    match current {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => incoming.to_owned(),
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn get_user_by_id(&self, user_id: UserId) -> Result<Option<User>> {
        Ok(self.users.lock().unwrap().get(&user_id).cloned())
    }

    async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.google_id.as_deref() == Some(google_id))
            .cloned())
    }

    async fn get_user_by_github_id(&self, github_id: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.github_id.as_deref() == Some(github_id))
            .cloned())
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        // 0001 unique indexes: email, google_id, github_id (23505 → Conflict).
        let taken = users.values().any(|u| {
            u.email == user.email
                || matches!(
                    (&u.google_id, &user.google_id),
                    (Some(a), Some(b)) if a == b && !b.is_empty()
                )
                || matches!(
                    (&u.github_id, &user.github_id),
                    (Some(a), Some(b)) if a == b && !b.is_empty()
                )
        });
        if taken {
            return Err(Error::Conflict("email already in use".to_owned()));
        }
        users.insert(user.id, user.clone());
        drop(users);
        Ok(())
    }

    async fn update(&self, user_id: UserId, patch: &UserPatch) -> Result<()> {
        // Go UpdateUsers: silent no-op on a missing id (0 rows, no error).
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            if let Some(v) = patch.is_active {
                u.is_active = v;
            }
            if let Some(v) = patch.is_blocked {
                u.is_blocked = v;
            }
            if let Some(v) = patch.role {
                u.role = v;
            }
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_me(&self, user_id: UserId, patch: &UpdateMeInput) -> Result<Option<User>> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            // ponytail: core `User` carries no phone/address/postal columns, so
            // the double applies only the fields it has; the Pg adapter writes all
            // seven (Go-parity SQL).
            if let Some(v) = &patch.first_name {
                u.first_name.clone_from(v);
            }
            if let Some(v) = &patch.last_name {
                u.last_name.clone_from(v);
            }
            if let Some(v) = &patch.picture {
                u.profile_picture = Some(v.clone());
            }
            if let Some(v) = &patch.language {
                u.language.clone_from(v);
            }
            u.updated_at = Utc::now();
            Ok(Some(u.clone()))
        } else {
            Ok(None)
        }
    }

    async fn after_failed_login(&self, user_id: UserId, lock_at: i32) -> Result<bool> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.retry_count += 1;
            u.is_blocked = u.retry_count >= lock_at;
            u.updated_at = Utc::now();
            Ok(u.is_blocked)
        } else {
            // RETURNING with 0 rows → RowNotFound → Error::NotFound.
            Err(Error::NotFound)
        }
    }

    async fn after_successful_login(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.retry_count = 0;
            u.last_login = Some(Utc::now());
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn link_google(&self, user_id: UserId, google_id: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.google_id = Some(google_id.to_owned());
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.as_deref(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn link_github(&self, user_id: UserId, github_id: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.github_id = Some(github_id.to_owned());
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.as_deref(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn github_only(&self, user_id: UserId, github_id: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.github_id = Some(github_id.to_owned());
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn save_github_login(&self, user_id: UserId, email: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            email.clone_into(&mut u.email);
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.as_deref(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let mut rows: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
        // ORDER BY created_at DESC (id tiebreak keeps the double deterministic).
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.0.cmp(&b.id.0)));
        Ok(rows
            .into_iter()
            .skip(usize::try_from(offset.max(0)).unwrap_or(usize::MAX))
            .take(usize::try_from(limit.max(0)).unwrap_or(0))
            .collect())
    }

    async fn count_users(&self) -> Result<i64> {
        Ok(i64::try_from(self.users.lock().unwrap().len()).unwrap_or(i64::MAX))
    }

    async fn upgrade_to_admin(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self.users.lock().unwrap().get_mut(&user_id) {
            u.role = <app>_core::Role::Admin;
            u.user_type = <app>_core::UserType::Admin;
            u.updated_at = Utc::now();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    fn uid(n: u64) -> UserId {
        UserId(uuid::Uuid::from_u128(u128::from(n)))
    }
    use super::*;
    use crate::mock::test_user;
    use <app>_core::{Role, UserType};

    fn user(id: u64, email: &str) -> User {
        test_user(id, email)
    }

    #[tokio::test]
    async fn create_duplicate_email_or_provider_is_conflict() {
        let repo = MockUserRepository::new();
        repo.create_user(&user(1, "a@t.dev")).await.unwrap();
        // Same email → 23505 → Conflict (Go EMAIL_TAKEN parity).
        let err = repo.create_user(&user(2, "a@t.dev")).await.unwrap_err();
        assert!(matches!(err, Error::Conflict(_)));
        // Same nonempty google_id → same unique-index conflict.
        let mut g = user(3, "g@t.dev");
        g.google_id = Some("gid1".to_owned());
        repo.create_user(&g).await.unwrap();
        let mut g2 = user(4, "g2@t.dev");
        g2.google_id = Some("gid1".to_owned());
        let err = repo.create_user(&g2).await.unwrap_err();
        assert!(matches!(err, Error::Conflict(_)));
    }

    #[tokio::test]
    async fn after_failed_login_locks_at_threshold_and_successful_login_resets() {
        let repo = MockUserRepository::new();
        repo.seed(user(1, "a@t.dev"));
        // lock_at = 3: 1st/2nd failures stay open, 3rd locks (SQL-side
        // increment: is_blocked = NEW retry_count >= lock_at).
        assert!(!repo.after_failed_login(uid(1), 3).await.unwrap());
        assert!(!repo.after_failed_login(uid(1), 3).await.unwrap());
        assert!(repo.after_failed_login(uid(1), 3).await.unwrap());
        assert_eq!(
            repo.get_user_by_id(uid(1))
                .await
                .unwrap()
                .unwrap()
                .retry_count,
            3
        );
        repo.after_successful_login(uid(1)).await.unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert_eq!(u.retry_count, 0);
        // Go parity: the success UPDATE clears retry_count but does NOT clear
        // is_blocked — a locked user stays locked until an admin unblocks.
        assert!(u.is_blocked);
        assert!(u.last_login.is_some());
    }

    #[tokio::test]
    async fn admin_patch_coalesces_and_update_me_applies_some_fields() {
        let repo = MockUserRepository::new();
        repo.seed(user(1, "a@t.dev"));
        // All-None patch leaves the row untouched (COALESCE($n, col)).
        repo.update(
            uid(1),
            &UserPatch {
                is_active: None,
                is_blocked: None,
                role: None,
            },
        )
        .await
        .unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert!(u.is_active && !u.is_blocked && u.role == Role::Viewer);
        // Partial admin patch touches exactly the given flags.
        repo.update(
            uid(1),
            &UserPatch {
                is_active: None,
                is_blocked: Some(true),
                role: None,
            },
        )
        .await
        .unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert!(u.is_active && u.is_blocked);
        // update_me: absent fields unchanged, given fields applied.
        repo.update_me(
            uid(1),
            &UpdateMeInput {
                first_name: Some("Nova".to_owned()),
                last_name: None,
                phone: None,
                picture: Some("pic".to_owned()),
                language: Some("pt".to_owned()),
                address: None,
                postal: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert_eq!(u.first_name, "Nova");
        assert_eq!(u.last_name, "Last");
        assert_eq!(u.profile_picture.as_deref(), Some("pic"));
        assert_eq!(u.language, "pt");
    }

    #[tokio::test]
    async fn oauth_link_keeps_existing_picture_and_github_save_swaps_email() {
        let repo = MockUserRepository::new();
        let mut u = user(1, "a@t.dev");
        u.profile_picture = Some("keep-me".to_owned());
        repo.seed(u);
        // COALESCE(NULLIF(profile_picture, ''), $3): nonempty wins.
        repo.link_google(uid(1), "gid", "new-pic").await.unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert_eq!(u.google_id.as_deref(), Some("gid"));
        assert!(u.is_verified);
        assert_eq!(u.profile_picture.as_deref(), Some("keep-me"));
        // Empty stored picture → the provider picture fills it.
        let mut u2 = user(2, "b@t.dev");
        u2.profile_picture = Some(String::new());
        repo.seed(u2);
        repo.link_github(uid(2), "ghid", "gh-pic").await.unwrap();
        let u2 = repo.get_user_by_id(uid(2)).await.unwrap().unwrap();
        assert_eq!(u2.profile_picture.as_deref(), Some("gh-pic"));
        // save_github_login: only email + verified + picture — github_id NOT touched.
        repo.save_github_login(uid(2), "moved@t.dev", "p")
            .await
            .unwrap();
        let u2 = repo.get_user_by_id(uid(2)).await.unwrap().unwrap();
        assert_eq!(u2.email, "moved@t.dev");
        assert!(u2.is_verified);
        assert_eq!(u2.github_id.as_deref(), Some("ghid"));
        assert_eq!(u2.profile_picture.as_deref(), Some("gh-pic"));
    }

    #[tokio::test]
    async fn list_users_newest_first_and_missing_id_reads_none() {
        let repo = MockUserRepository::new();
        let mut a = user(1, "a@t.dev");
        a.created_at -= chrono::Duration::hours(2);
        let mut b = user(2, "b@t.dev");
        b.created_at -= chrono::Duration::hours(1);
        repo.seed(a.clone());
        repo.seed(b);
        let rows = repo.list_users(10, 0).await.unwrap();
        assert_eq!(rows[0].id, uid(2));
        assert_eq!(rows[1].id, uid(1));
        // LIMIT/OFFSET paging.
        let page = repo.list_users(1, 1).await.unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, uid(1));
        // Missing id reads None (no error).
        assert!(repo.get_user_by_id(uid(99)).await.unwrap().is_none());
        // count reflects the table.
        assert_eq!(repo.count_users().await.unwrap(), 2);
        // upgrade_to_admin flips both columns (Go UpgradeToAdmin).
        repo.upgrade_to_admin(uid(1)).await.unwrap();
        let u = repo.get_user_by_id(uid(1)).await.unwrap().unwrap();
        assert!(u.role == Role::Admin && u.user_type == UserType::Admin);
    }
}
