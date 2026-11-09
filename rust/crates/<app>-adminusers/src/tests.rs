//! The adminusers slice's tests + in-crate in-memory doubles so the slice's
//! test build stays self-contained (the workspace doubles live in
//! `<app>-ports::mock`; the api crate's wiring tests use those).
#![allow(clippy::unimplemented)] // unexercised port methods are loud no-ops

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;

use crate::AdminUsersService;
use <app>_core::{Error, Result, Role, User, UserId, UserType};
use <app>_ports::{AuditLog, AuditLogRepository, UserPatch, UserRepository};

/// In-memory `users` table double — only the adminusers surface is real;
/// every other port method is an `unimplemented!` (loud, never hit).
struct TestUsers {
    rows: Mutex<Vec<User>>,
}

impl TestUsers {
    fn new(rows: Vec<User>) -> Self {
        Self {
            rows: Mutex::new(rows),
        }
    }

    /// Sync readback for assertions.
    fn read(&self, id: UserId) -> Option<User> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned()
    }
}

#[async_trait]
impl UserRepository for TestUsers {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn get_user_by_id(&self, user_id: UserId) -> Result<Option<User>> {
        Ok(self.read(user_id))
    }

    async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.google_id.as_deref() == Some(google_id))
            .cloned())
    }

    async fn get_user_by_github_id(&self, github_id: &str) -> Result<Option<User>> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.github_id.as_deref() == Some(github_id))
            .cloned())
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        self.rows.lock().unwrap().push(user.clone());
        Ok(())
    }

    async fn update(&self, user_id: UserId, patch: &UserPatch) -> Result<()> {
        if let Some(u) = self
            .rows
            .lock()
            .unwrap()
            .iter_mut()
            .find(|u| u.id == user_id)
        {
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
        Ok(())
    }

    async fn update_me(
        &self,
        _: UserId,
        _: &<app>_ports::UpdateMeInput,
    ) -> Result<Option<User>> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn after_failed_login(&self, _: UserId, _: i32) -> Result<bool> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn after_successful_login(&self, _: UserId) -> Result<()> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn link_google(&self, _: UserId, _: &str, _: &str) -> Result<()> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn link_github(&self, _: UserId, _: &str, _: &str) -> Result<()> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn github_only(&self, _: UserId, _: &str) -> Result<()> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn save_github_login(&self, _: UserId, _: &str, _: &str) -> Result<()> {
        unimplemented!("not exercised by the adminusers tests")
    }

    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let mut rows = self.rows.lock().unwrap().clone();
        // ORDER BY created_at DESC.
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let skip = usize::try_from(offset.max(0)).unwrap_or(usize::MAX);
        let take = usize::try_from(limit.max(0)).unwrap_or(0);
        Ok(rows.into_iter().skip(skip).take(take).collect())
    }

    async fn count_users(&self) -> Result<i64> {
        Ok(i64::try_from(self.rows.lock().unwrap().len()).unwrap_or(i64::MAX))
    }

    async fn upgrade_to_admin(&self, user_id: UserId) -> Result<()> {
        if let Some(u) = self
            .rows
            .lock()
            .unwrap()
            .iter_mut()
            .find(|u| u.id == user_id)
        {
            u.role = Role::Admin;
            u.user_type = UserType::Admin;
        }
        Ok(())
    }
}

/// In-memory `audit_logs` double; `failing()` reproduces a down store.
struct MemAudits {
    rows: Mutex<Vec<AuditLog>>,
    fails: bool,
}

impl MemAudits {
    fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            fails: false,
        }
    }

    fn failing() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            fails: true,
        }
    }

    /// Sync readback for assertions (insertion order).
    fn entries(&self) -> Vec<AuditLog> {
        self.rows.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuditLogRepository for MemAudits {
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
        if self.fails {
            return Err(Error::Internal("db down".into()));
        }
        self.rows.lock().unwrap().push(AuditLog {
            id: uuid::Uuid::now_v7(),
            actor_user_id: actor_id,
            action: action.to_owned(),
            entity: entity.to_owned(),
            entity_id: entity_id.map(str::to_owned),
            payload,
            ip_address: ip.map(str::to_owned),
            user_agent: ua.map(str::to_owned),
            created_at: Utc::now(),
        });
        Ok(())
    }

    async fn list_audit(&self, limit: i64, offset: i64) -> Result<Vec<AuditLog>> {
        let rows = self.entries();
        let skip = usize::try_from(offset.max(0)).unwrap_or(usize::MAX);
        let take = usize::try_from(limit.max(0)).unwrap_or(usize::MAX);
        Ok(rows.into_iter().rev().skip(skip).take(take).collect())
    }

    async fn count_audit(&self) -> Result<i64> {
        Ok(i64::try_from(self.rows.lock().unwrap().len()).unwrap_or(i64::MAX))
    }
}

// ── fixtures ───────────────────────────────────────────────────────────────

fn uid(n: u64) -> UserId {
    UserId(uuid::Uuid::from_u128(u128::from(n)))
}

/// Row fixture: Viewer/USER, active, verified-never-happened.
fn user(id: u64, email: &str) -> User {
    let now = Utc::now();
    User {
        id: uid(id),
        email: email.to_owned(),
        first_name: "First".into(),
        last_name: "Last".into(),
        role: Role::Viewer,
        user_type: UserType::User,
        is_active: true,
        is_blocked: false,
        is_verified: false,
        retry_count: 0,
        password: "hash".into(),
        language: "en".into(),
        profile_picture: None,
        google_id: None,
        github_id: None,
        last_login: None,
        created_at: now,
        updated_at: now,
    }
}

/// Row fixture: role + `user_type` ADMIN.
fn admin_row(id: u64, email: &str) -> User {
    let mut u = user(id, email);
    u.role = Role::Admin;
    u.user_type = UserType::Admin;
    u
}

/// Service + handles on the doubles (for readback assertions).
fn svc(rows: &[User], whitelist: &[&str]) -> (AdminUsersService, Arc<TestUsers>, Arc<MemAudits>) {
    let users = Arc::new(TestUsers::new(rows.to_vec()));
    let audits = Arc::new(MemAudits::new());
    (
        AdminUsersService::new(
            users.clone(),
            &whitelist
                .iter()
                .map(|e| (*e).to_owned())
                .collect::<Vec<_>>(),
        ),
        users,
        audits,
    )
}

fn patch(is_active: Option<bool>, is_blocked: Option<bool>, role: Option<Role>) -> UserPatch {
    UserPatch {
        is_active,
        is_blocked,
        role,
    }
}

// ── list / get ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_page_and_total_and_get_roundtrips() {
    let (svc, _users, _audits) = svc(&[user(1, "a@t.dev")], &[]);
    let (rows, total) = svc.list(20, 0).await.unwrap();
    assert_eq!((rows.len(), total), (1, 1));
    assert_eq!(rows[0].id, uid(1));
    // Get roundtrips the stored row.
    let got = svc.get(&uid(1).0.to_string()).await.unwrap();
    assert_eq!(got.email, "a@t.dev");
}

#[tokio::test]
async fn get_and_update_treat_unknown_or_non_uuid_ids_as_404() {
    let (svc, _users, audits) = svc(&[user(1, "a@t.dev")], &[]);
    assert!(matches!(svc.get("ghost").await, Err(Error::NotFound)));
    assert!(matches!(
        svc.get(&uid(99).0.to_string()).await,
        Err(Error::NotFound)
    ));
    // A rank-legal update on a missing row is a miss and never audited.
    let err = svc
        .update(
            uid(9),
            &[Role::Admin],
            &uid(99).0.to_string(),
            Some("9.9.9.9"),
            Some("ua"),
            patch(None, None, Some(Role::Editor)),
            audits.as_ref(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), 404);
    assert_eq!(audits.count_audit().await.unwrap(), 0);
}

// ── update: validation, patch + audit ──────────────────────────────────────

#[tokio::test]
async fn update_rejects_empty_patch_and_applies_role_patch_with_audit() {
    let (svc, users, audits) = svc(&[user(1, "a@t.dev")], &[]);
    // Empty patch → 400 (go writes 422; recorded deviation).
    let err = svc
        .update(
            uid(9),
            &[Role::Admin],
            &uid(1).0.to_string(),
            None,
            None,
            UserPatch::default(),
            audits.as_ref(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), 400);

    let row = svc
        .update(
            uid(9),
            &[Role::Admin],
            &uid(1).0.to_string(),
            Some("9.9.9.9"),
            Some("ua"),
            patch(None, None, Some(Role::Editor)),
            audits.as_ref(),
        )
        .await
        .unwrap();
    assert_eq!(row.role, Role::Editor, "fresh row carries the new role");
    assert_eq!(users.read(uid(1)).unwrap().role, Role::Editor);
    // Exactly one audit entry with the go payload projection.
    let entries = audits.entries();
    assert_eq!(entries.len(), 1);
    let a = &entries[0];
    assert_eq!(a.action, "user.updated");
    assert_eq!(a.entity, "user");
    assert_eq!(a.entity_id.as_deref(), Some(&*uid(1).0.to_string()));
    assert_eq!(a.actor_user_id, Some(uid(9)));
    assert_eq!(
        a.payload.as_ref().unwrap(),
        &serde_json::json!({"role": "MODERATOR"})
    );
    assert_eq!(a.ip_address.as_deref(), Some("9.9.9.9"));
    assert_eq!(a.user_agent.as_deref(), Some("ua"));
}

#[tokio::test]
async fn update_applies_nullable_booleans_with_payload_parity() {
    let (svc, users, audits) = svc(&[user(1, "a@t.dev")], &[]);
    svc.update(
        uid(9),
        &[Role::Admin],
        &uid(1).0.to_string(),
        None,
        None,
        patch(Some(false), None, None),
        audits.as_ref(),
    )
    .await
    .unwrap();
    assert!(!users.read(uid(1)).unwrap().is_active);
    let a = &audits.entries()[0];
    assert_eq!(
        a.payload.as_ref().unwrap(),
        &serde_json::json!({"is_active": false})
    );
}

// ── update: the rank gate (go authz.CanManage) ─────────────────────────────

#[tokio::test]
async fn update_enforces_self_or_subordinate() {
    let (svc, _users, audits) = svc(
        &[
            admin_row(1, "admin@t.dev"),
            user(2, "mod@t.dev"),
            admin_row(3, "other@t.dev"),
        ],
        &[],
    );
    let target2 = uid(2).0.to_string();
    // Admin may manage the strictly lower-ranked MODERATOR row.
    svc.update(
        uid(9),
        &[Role::Admin],
        &target2,
        None,
        None,
        patch(Some(true), None, None),
        audits.as_ref(),
    )
    .await
    .unwrap();
    // An ADMIN cannot reach another ADMIN's row (strict `>`).
    let err = svc
        .update(
            uid(9),
            &[Role::Admin],
            &uid(3).0.to_string(),
            None,
            None,
            patch(Some(true), None, None),
            audits.as_ref(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), 403);
    // A stale viewer token cannot manage its own (higher-ranked) row.
    let err = svc
        .update(
            uid(1),
            &[Role::Viewer],
            &uid(1).0.to_string(),
            None,
            None,
            patch(Some(true), None, None),
            audits.as_ref(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), 403);
    // Self-management while the rank holds: admin on its own row.
    svc.update(
        uid(1),
        &[Role::Admin],
        &uid(1).0.to_string(),
        None,
        None,
        patch(Some(true), None, None),
        audits.as_ref(),
    )
    .await
    .unwrap();
    // Strongest-of-roles parity: a [viewer, admin] token governs as admin
    // (require_admin's `.any()` reading).
    svc.update(
        uid(9),
        &[Role::Viewer, Role::Admin],
        &target2,
        None,
        None,
        patch(Some(true), None, None),
        audits.as_ref(),
    )
    .await
    .unwrap();
    assert_eq!(audits.count_audit().await.unwrap(), 3);
}

#[tokio::test]
async fn empty_actor_role_list_gates_everything_off() {
    let (svc, _users, audits) = svc(&[user(1, "a@t.dev")], &[]);
    let err = svc
        .update(
            uid(9),
            &[],
            &uid(1).0.to_string(),
            None,
            None,
            patch(Some(true), None, None),
            audits.as_ref(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), 403);
}

#[tokio::test]
async fn update_tolerates_audit_failure() {
    let (svc, users, _audits) = svc(&[user(1, "a@t.dev")], &[]);
    let row = svc
        .update(
            uid(9),
            &[Role::Admin],
            &uid(1).0.to_string(),
            None,
            None,
            patch(None, Some(true), None),
            &MemAudits::failing(),
        )
        .await
        .unwrap();
    assert!(row.is_blocked, "mutation landed despite the audit failure");
    assert!(users.read(uid(1)).unwrap().is_blocked);
}

// ── upgrade_to_admin ───────────────────────────────────────────────────────

#[tokio::test]
async fn upgrade_whitelist_gates_before_lookup_then_promotes() {
    let (svc, users, _audits) = svc(&[user(1, "a@t.dev")], &["a@t.dev", "rowless@t.dev"]);
    // Non-whitelisted → 403 before any user lookup (anti-probe gate).
    let err = svc.upgrade_to_admin("ghost@t.dev").await.unwrap_err();
    assert_eq!(err.status_code(), 403);
    // Whitelisted but unknown email → 404.
    let err2 = svc.upgrade_to_admin("rowless@t.dev").await.unwrap_err();
    assert_eq!(err2.status_code(), 404);
    // Whitelisted + existing row → promotion flips role + user_type.
    let res = svc.upgrade_to_admin("a@t.dev").await.unwrap();
    assert!(res.upgraded);
    assert_eq!(res.email, "a@t.dev");
    assert_eq!(res.message, "User 'a@t.dev' has been upgraded to admin");
    let u = users.read(uid(1)).unwrap();
    assert_eq!(u.role, Role::Admin);
    assert_eq!(u.user_type, UserType::Admin);
    // Re-upgrade reads as "already an admin" (no further repo write).
    let again = svc.upgrade_to_admin("a@t.dev").await.unwrap();
    assert!(!again.upgraded);
    assert_eq!(again.message, "User is already an admin");
}

#[tokio::test]
async fn whitelist_is_normalized_and_caller_case_is_insensitive() {
    let (svc, users, _audits) = svc(
        // DB emails are stored lowercase (the service normalizes the caller's
        // case the same way the whitelist entries are normalized).
        &[user(1, "a@t.dev"), user(2, "b@y.test")],
        &[" A@T.dev ", "", "b@y.test"],
    );
    // Caller case/format is normalized before the set check.
    svc.upgrade_to_admin("  A@T.DEV ").await.unwrap();
    assert_eq!(users.read(uid(1)).unwrap().role, Role::Admin);
    // The other whitelisted entry promotes its row too.
    svc.upgrade_to_admin("b@y.test").await.unwrap();
    assert_eq!(users.read(uid(2)).unwrap().user_type, UserType::Admin);
}

#[tokio::test]
async fn empty_whitelist_denies_every_email() {
    let (svc, _users, _audits) = svc(&[user(1, "a@t.dev")], &[]);
    let err = svc.upgrade_to_admin("a@t.dev").await.unwrap_err();
    assert_eq!(err.status_code(), 403);
}
