//! `#[cfg(test)]` in-crate doubles for the identity ports — the go
//! per-slice fakes parity. One shared in-memory store (the "database")
//! behind a single `Mutex` with five thin adapters.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use <app>_core::{
    EmailJob, EmailMessage, Error, MultiIpRow, PasswordReset, Result, Role, Session, SessionId,
    User, UserId, UserType, Verification,
};
use <app>_ports::{
    EmailVerificationRepository, OutboxRepository, PasswordResetRepository, SessionRepository,
    SessionUser, UpdateMeInput, UserPatch, UserRepository,
};

/// u64 → seed id (`Uuid::from_u128`), test fixtures only.
#[must_use]
pub fn uid(n: u64) -> UserId {
    UserId(uuid::Uuid::from_u128(u128::from(n)))
}

/// Seed session id.
#[must_use]
pub fn sid(n: u64) -> SessionId {
    SessionId(uuid::Uuid::from_u128(u128::from(n)))
}

/// User + session fixtures.
pub mod fixture {
    use super::*;

    /// Default-unverified `USER` row (tests mutate the returned value).
    #[must_use]
    pub fn user(id: u64, email: &str) -> User {
        let now = Utc::now();
        User {
            id: uid(id),
            email: email.to_owned(),
            first_name: "Ada".to_owned(),
            last_name: "Lovelace".to_owned(),
            role: Role::Viewer,
            user_type: UserType::User,
            is_active: true,
            is_blocked: false,
            is_verified: false,
            retry_count: 0,
            password: "x".to_owned(),
            language: "en".to_owned(),
            profile_picture: None,
            google_id: None,
            github_id: None,
            last_login: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Active session with a one-hour absolute lifetime.
    #[must_use]
    pub fn session(n: u64, user_n: u64, token: &str) -> Session {
        let now = Utc::now();
        Session {
            id: sid(n),
            user_id: uid(user_n),
            refresh_token: token.to_owned(),
            ip_address: Some("10.0.0.1".to_owned()),
            user_agent: Some("test/1".to_owned()),
            is_active: true,
            last_activity: now,
            expires_at: now + Duration::hours(1),
            created_at: now,
        }
    }
}

/// Shared in-memory store behind every double (the "database").
#[derive(Debug, Default)]
pub struct Store {
    /// All user rows.
    pub users: HashMap<UserId, User>,
    /// All session rows.
    pub sessions: HashMap<SessionId, Session>,
    /// `email_verifications`, insertion order = arrival order.
    pub verifications: Vec<Verification>,
    /// `password_resets` with the insert-time `created_at` (rate window).
    pub resets: Vec<(PasswordReset, DateTime<Utc>)>,
    /// Enqueued email messages (insertion order).
    pub emails: Vec<EmailMessage>,
    /// Recorded profile-seeding writes: `("companies"|"users_profile", id)`.
    pub profiles: Vec<(&'static str, UserId)>,
}

type Shared = Arc<Mutex<Store>>;

/// All five doubles over one shared [`Store`].
#[derive(Debug)]
pub struct Fakes {
    /// Shared state (read/seeding in tests).
    pub store: Shared,
    /// `users` contract.
    pub users: Arc<FakeUserRepo>,
    /// `sessions` contract.
    pub sessions: Arc<FakeSessionRepo>,
    /// `email_verifications` contract.
    pub verifications: Arc<FakeVerificationRepo>,
    /// `password_resets` contract.
    pub resets: Arc<FakeResetRepo>,
    /// `email_jobs` (outbox) contract.
    pub outbox: Arc<FakeOutboxRepo>,
}

impl Default for Fakes {
    fn default() -> Self {
        Self::new()
    }
}

impl Fakes {
    #[must_use]
    pub(crate) fn new() -> Self {
        let store: Shared = Arc::new(Mutex::new(Store::default()));
        Self {
            store: store.clone(),
            users: Arc::new(FakeUserRepo(store.clone())),
            sessions: Arc::new(FakeSessionRepo(store.clone())),
            verifications: Arc::new(FakeVerificationRepo(store.clone())),
            resets: Arc::new(FakeResetRepo(store.clone())),
            outbox: Arc::new(FakeOutboxRepo(store)),
        }
    }

    /// Locked snapshot for reads/seeding in tests.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Store> {
        self.store.lock().expect("test store")
    }
}

/// users-table double (go fakes parity: unexpected calls panic loudly).
#[derive(Debug)]
pub struct FakeUserRepo(Shared);

fn keep_picture(current: Option<String>, incoming: &str) -> String {
    match current {
        Some(p) if !p.is_empty() => p,
        _ => incoming.to_owned(),
    }
}

fn dup_of(store: &Store, row: &User) -> bool {
    store.users.values().any(|u| {
        u.email == row.email
            || (row.google_id.is_some() && u.google_id == row.google_id)
            || (row.github_id.is_some() && u.github_id == row.github_id)
    })
}

#[async_trait]
impl UserRepository for FakeUserRepo {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .0
            .lock()
            .expect("store")
            .users
            .values()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>> {
        Ok(self.0.lock().expect("store").users.get(&id).cloned())
    }
    async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
        let s = self.0.lock().expect("store");
        Ok(s.users
            .values()
            .find(|u| u.google_id.as_deref() == Some(google_id))
            .cloned())
    }
    async fn get_user_by_github_id(&self, github_id: &str) -> Result<Option<User>> {
        let s = self.0.lock().expect("store");
        Ok(s.users
            .values()
            .find(|u| u.github_id.as_deref() == Some(github_id))
            .cloned())
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        let mut s = self.0.lock().expect("store");
        if dup_of(&s, user) {
            return Err(Error::Conflict("duplicate".to_owned()));
        }
        s.users.insert(user.id, user.clone());
        drop(s);
        Ok(())
    }

    async fn update(&self, user_id: UserId, patch: &UserPatch) -> Result<()> {
        let mut s = self.0.lock().expect("store");
        if let Some(u) = s.users.get_mut(&user_id) {
            if let Some(v) = patch.is_active {
                u.is_active = v;
            }
            if let Some(v) = patch.is_blocked {
                u.is_blocked = v;
            }
            if let Some(v) = patch.role {
                u.role = v;
            }
        }
        drop(s);
        Ok(())
    }

    async fn update_me(&self, user_id: UserId, patch: &UpdateMeInput) -> Result<Option<User>> {
        let mut s = self.0.lock().expect("store");
        let Some(u) = s.users.get_mut(&user_id) else {
            return Ok(None);
        };
        if let Some(v) = patch.first_name.clone() {
            u.first_name = v;
        }
        if let Some(v) = patch.last_name.clone() {
            u.last_name = v;
        }
        if let Some(v) = patch.language.clone() {
            u.language = v;
        }
        let updated = u.clone();
        drop(s);
        Ok(Some(updated))
    }

    async fn after_failed_login(&self, user_id: UserId, lock_at: i32) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let Some(u) = s.users.get_mut(&user_id) else {
            return Err(Error::NotFound);
        };
        u.retry_count += 1;
        u.is_blocked = u.retry_count >= lock_at;
        let blocked = u.is_blocked;
        drop(s);
        Ok(blocked)
    }

    async fn after_successful_login(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.retry_count = 0;
            u.last_login = Some(Utc::now());
        }
        Ok(())
    }

    async fn link_google(&self, user_id: UserId, google_id: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.google_id = Some(google_id.to_owned());
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.take(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn link_github(&self, user_id: UserId, github_id: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.github_id = Some(github_id.to_owned());
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.take(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn github_only(&self, user_id: UserId, github_id: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.github_id = Some(github_id.to_owned());
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn save_github_login(&self, user_id: UserId, email: &str, picture: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.email = email.to_owned();
            u.is_verified = true;
            u.profile_picture = Some(keep_picture(u.profile_picture.take(), picture));
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn list_users(&self, _limit: i64, _offset: i64) -> Result<Vec<User>> {
        unimplemented!("auth fakes: admin listing not exercised")
    }

    async fn count_users(&self) -> Result<i64> {
        unimplemented!("auth fakes: admin count not exercised")
    }

    async fn upgrade_to_admin(&self, _user_id: UserId) -> Result<()> {
        unimplemented!("auth fakes: admin promotion not exercised")
    }
}

/// sessions-table double.
#[derive(Debug)]
pub struct FakeSessionRepo(Shared);

#[async_trait]
impl SessionRepository for FakeSessionRepo {
    async fn create_session(&self, session: &Session) -> Result<()> {
        self.0
            .lock()
            .expect("store")
            .sessions
            .insert(session.id, session.clone());
        Ok(())
    }

    async fn active_session_by_token(&self, refresh_token: &str) -> Result<Option<SessionUser>> {
        let s = self.0.lock().expect("store");
        let now = Utc::now();
        Ok(s.sessions
            .values()
            .find(|s| s.is_active && s.refresh_token == refresh_token && s.expires_at > now)
            .and_then(|session| {
                let user = s.users.get(&session.user_id)?;
                Some(SessionUser {
                    session: session.clone(),
                    email: user.email.clone(),
                    role: user.role,
                    user_active: user.is_active,
                    is_blocked: user.is_blocked,
                    is_verified: user.is_verified,
                })
            }))
    }

    async fn deactivate_session(&self, session_id: SessionId) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let claimed = s.sessions.get(&session_id).is_some_and(|ses| ses.is_active);
        if claimed {
            s.sessions.get_mut(&session_id).expect("checked").is_active = false;
        }
        drop(s);
        Ok(claimed)
    }

    async fn rotate_session(
        &self,
        session_id: SessionId,
        old_token: &str,
        new_refresh_token: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let claimed = s.sessions.get(&session_id).is_some_and(|ses| {
            ses.is_active && ses.refresh_token == old_token && ses.expires_at > Utc::now()
        });
        if claimed {
            let ses = s.sessions.get_mut(&session_id).expect("checked");
            ses.refresh_token = new_refresh_token.to_owned();
            ses.ip_address = ip_address.map(str::to_owned);
            ses.user_agent = user_agent.map(str::to_owned);
        }
        drop(s);
        Ok(claimed)
    }

    async fn deactivate_by_token_for_user(
        &self,
        user_id: UserId,
        refresh_token: &str,
    ) -> Result<()> {
        let mut s = self.0.lock().expect("store");
        if let Some(ses) = s.sessions.values_mut().find(|ses| {
            ses.user_id == user_id && ses.is_active && ses.refresh_token == refresh_token
        }) {
            ses.is_active = false;
        }
        drop(s);
        Ok(())
    }

    async fn list_sessions_by_user(
        &self,
        user_id: UserId,
        active_only: bool,
    ) -> Result<Vec<Session>> {
        let mut rows: Vec<Session> = self
            .0
            .lock()
            .expect("store")
            .sessions
            .values()
            .filter(|ses| ses.user_id == user_id && (!active_only || ses.is_active))
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        Ok(rows)
    }

    async fn current_session(&self, user_id: UserId) -> Result<Option<Session>> {
        Ok(self
            .0
            .lock()
            .expect("store")
            .sessions
            .values()
            .filter(|ses| ses.user_id == user_id && ses.is_active)
            .max_by_key(|ses| ses.last_activity)
            .cloned())
    }

    async fn revoke_session_for_user(
        &self,
        user_id: UserId,
        session_id: SessionId,
    ) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let claimed = s
            .sessions
            .values()
            .any(|ses| ses.id == session_id && ses.user_id == user_id && ses.is_active);
        if claimed {
            s.sessions.get_mut(&session_id).expect("checked").is_active = false;
        }
        drop(s);
        Ok(claimed)
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<i64> {
        let mut s = self.0.lock().expect("store");
        let mut claimed = 0;
        for ses in s.sessions.values_mut() {
            if ses.user_id == user_id && ses.is_active {
                ses.is_active = false;
                claimed += 1;
            }
        }
        drop(s);
        Ok(claimed)
    }

    async fn count_expired_active(&self) -> Result<i64> {
        let now = Utc::now();
        let count = self
            .0
            .lock()
            .expect("store")
            .sessions
            .values()
            .filter(|ses| ses.is_active && ses.expires_at <= now)
            .count();
        Ok(i64::try_from(count).unwrap_or(i64::MAX))
    }

    /// Users with more than 3 distinct IPs among the active sessions since
    /// `since` (go `MultiIPSessions` threshold parity).
    async fn multi_ip_sessions(&self, since: DateTime<Utc>) -> Result<Vec<MultiIpRow>> {
        let s = self.0.lock().expect("store");
        let mut ip_counts: HashMap<UserId, Vec<String>> = HashMap::new();
        for ses in s.sessions.values() {
            if ses.is_active
                && ses.last_activity >= since
                && let Some(ip) = &ses.ip_address
            {
                let seen = ip_counts.entry(ses.user_id).or_default();
                if !seen.iter().any(|i| i == ip) {
                    seen.push(ip.clone());
                }
            }
        }
        drop(s);
        Ok(ip_counts
            .iter()
            .filter_map(|(user_id, ips)| {
                let count = i64::try_from(ips.len()).unwrap_or(i64::MAX);
                (count > 3).then_some(MultiIpRow {
                    user_id: *user_id,
                    ip_count: count,
                })
            })
            .collect())
    }
}

/// `email_verifications` + profile seeding double.
#[derive(Debug)]
pub struct FakeVerificationRepo(Shared);

#[async_trait]
impl EmailVerificationRepository for FakeVerificationRepo {
    async fn create_verification(&self, verification: &Verification) -> Result<()> {
        self.0
            .lock()
            .expect("store")
            .verifications
            .push(verification.clone());
        Ok(())
    }

    async fn latest_verification(&self, user_id: UserId) -> Result<Option<Verification>> {
        let s = self.0.lock().expect("store");
        Ok(s.verifications
            .iter()
            .rev()
            .find(|v| v.user_id == user_id)
            .cloned())
    }

    async fn use_verification(&self, id: uuid::Uuid, token: &str) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let claimed = s
            .verifications
            .iter_mut()
            .any(|v| v.id == id && !v.used && v.token == token);
        if claimed {
            let v = s
                .verifications
                .iter_mut()
                .find(|v| v.id == id)
                .expect("checked");
            v.used = true;
        }
        drop(s);
        Ok(claimed)
    }

    async fn bump_verification(&self, id: uuid::Uuid) -> Result<()> {
        if let Some(v) = self
            .0
            .lock()
            .expect("store")
            .verifications
            .iter_mut()
            .find(|v| v.id == id)
        {
            v.attempts += 1;
        }
        Ok(())
    }

    async fn set_email_verified(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.is_verified = true;
        }
        Ok(())
    }

    async fn create_company_profile(&self, user_id: UserId, _name: &str) -> Result<()> {
        self.0
            .lock()
            .expect("store")
            .profiles
            .push(("companies", user_id));
        Ok(())
    }

    async fn create_user_profile(&self, user_id: UserId, _name: &str) -> Result<()> {
        self.0
            .lock()
            .expect("store")
            .profiles
            .push(("users_profile", user_id));
        Ok(())
    }
}

/// `password_resets` + password/session revocation double.
#[derive(Debug)]
pub struct FakeResetRepo(Shared);

#[async_trait]
impl PasswordResetRepository for FakeResetRepo {
    async fn count_resets_since(&self, email: &str, since: DateTime<Utc>) -> Result<i64> {
        let s = self.0.lock().expect("store");
        let count = s
            .resets
            .iter()
            .filter(|(reset, created)| reset.email == email && created >= &since)
            .count();
        drop(s);
        Ok(i64::try_from(count).unwrap_or(i64::MAX))
    }

    async fn create_reset(&self, reset: &PasswordReset) -> Result<()> {
        self.0
            .lock()
            .expect("store")
            .resets
            .push((reset.clone(), Utc::now()));
        Ok(())
    }

    async fn latest_reset(&self, email: &str) -> Result<Option<PasswordReset>> {
        let s = self.0.lock().expect("store");
        Ok(s.resets
            .iter()
            .rev()
            .find(|(reset, _)| reset.email == email)
            .map(|(r, _)| r.clone()))
    }

    async fn mark_reset_used(&self, id: uuid::Uuid, token: &str) -> Result<bool> {
        let mut s = self.0.lock().expect("store");
        let claimed = s
            .resets
            .iter()
            .any(|(reset, _)| reset.id == id && !reset.used && reset.token == token);
        if claimed {
            let e = s
                .resets
                .iter_mut()
                .find(|(r, _)| r.id == id)
                .expect("checked");
            e.0.used = true;
        }
        drop(s);
        Ok(claimed)
    }

    async fn bump_reset(&self, id: uuid::Uuid) -> Result<()> {
        if let Some(e) = self
            .0
            .lock()
            .expect("store")
            .resets
            .iter_mut()
            .find(|(reset, _)| reset.id == id)
        {
            e.0.attempts += 1;
        }
        Ok(())
    }

    async fn set_password_and_verify(&self, user_id: UserId, hash: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.password = hash.to_owned();
            u.is_verified = true;
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn set_password(&self, user_id: UserId, hash: &str) -> Result<()> {
        if let Some(u) = self.0.lock().expect("store").users.get_mut(&user_id) {
            u.password = hash.to_owned();
            u.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn revoke_all_sessions(&self, user_id: UserId) -> Result<()> {
        let mut s = self.0.lock().expect("store");
        for ses in s.sessions.values_mut() {
            if ses.user_id == user_id && ses.is_active {
                ses.is_active = false;
            }
        }
        drop(s);
        Ok(())
    }

    async fn revoke_other_sessions(&self, user_id: UserId, keep: SessionId) -> Result<()> {
        let mut s = self.0.lock().expect("store");
        for ses in s.sessions.values_mut() {
            if ses.user_id == user_id && ses.is_active && ses.id != keep {
                ses.is_active = false;
            }
        }
        drop(s);
        Ok(())
    }
}

/// `email_jobs` double (services only enqueue; the worker claims).
#[derive(Debug)]
pub struct FakeOutboxRepo(Shared);

#[async_trait]
impl OutboxRepository for FakeOutboxRepo {
    async fn enqueue(&self, message: &EmailMessage) -> Result<()> {
        self.0.lock().expect("store").emails.push(message.clone());
        Ok(())
    }

    async fn complete(&self, _job_id: uuid::Uuid) -> Result<()> {
        unimplemented!("auth fakes: outbox worker not exercised")
    }

    async fn fail(&self, _job: &EmailJob, _cause: &str) -> Result<()> {
        unimplemented!("auth fakes: outbox worker not exercised")
    }

    async fn claim(&self, _now: DateTime<Utc>, _limit: i64) -> Result<Vec<EmailJob>> {
        unimplemented!("auth fakes: outbox worker not exercised")
    }
}
