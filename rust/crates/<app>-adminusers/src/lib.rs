//! Admin-users use-case (go `features/adminusers` parity): the paginated
//! user roster, a single fetch and the rank-gated partial update behind the
//! ADMIN rank, an append-only `audit_logs` trail written on every mutation,
//! plus the unauthenticated whitelist bootstrap upgrade
//! (`<APP>_ALLOWED_ADMIN_EMAILS`).
//!
//! Wiring note (go `http.go` handler parity): `compose()` pre-wires the users
//! repository + the whitelist, and the audit port rides [`update`] — an admin
//! handler passes `state.audits_repo.as_ref()` so the use-case, not the
//! controller, owns the audit write.
use std::collections::HashSet;
use std::sync::Arc;

use <app>_core::{Error, Result, Role, User, UserId, UserType};
use <app>_ports::{AuditLogRepository, UserPatch, UserRepository};
use serde::Serialize;

/// Audit vocabulary (go `Update` parity): the action/entity pair stamped on
/// every admin mutation's `audit_logs` row.
const AUDIT_ACTION_UPDATED: &str = "user.updated";
const AUDIT_ENTITY: &str = "user";

/// The rank gate's 403 (go `authz.ErrForbidden` message parity).
const INSUFFICIENT_ROLE: &str = "insufficient role";

/// Go `ErrNotWhitelisted`: the bootstrap upgrade denies any email absent from
/// `<APP>_ALLOWED_ADMIN_EMAILS` before a user row is even looked up.
const NOT_WHITELISTED: &str = "email is not authorized to upgrade to admin";

/// Bootstrap-upgrade outcome (go `UpgradeResult` parity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminUpgradeResult {
    pub email: String,
    pub message: String,
    /// `false` reads as "the account already was an admin".
    pub upgraded: bool,
}

/// The slice's service. Cheap to clone; wired by the composition root with a
/// concrete repo, tested with in-crate doubles (see `tests.rs`).
#[derive(Clone)]
pub struct AdminUsersService {
    users: Arc<dyn UserRepository>,
    whitelist: HashSet<String>,
}

impl std::fmt::Debug for AdminUsersService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminUsersService").finish_non_exhaustive()
    }
}

/// Go `rolex.Rank` on the core `Role` ranks (an owner row is stored as
/// `ADMIN`, so owner and admin rank equal — the strict `>` gate keeps admin
/// rows reachable only by themselves, go hierarchy parity).
const fn rank(r: Role) -> u8 {
    match r {
        Role::Owner | Role::Admin => 3,
        Role::Editor => 2,
        Role::Viewer => 1,
    }
}

/// The token's governing rank: the strongest of its roles (`0` when empty —
/// an empty role list gates everything off, fail-closed).
fn effective_rank(roles: &[Role]) -> u8 {
    roles.iter().copied().map(rank).max().unwrap_or(0)
}

/// Go `authz.Context::CanManage` — self-or-subordinate, strict ranks: an
/// ADMIN cannot manage another ADMIN, and a stale actor token cannot outrank
/// the row's stored role (the actor must still rank at-or-above its own row).
fn can_manage(actor_id: UserId, actor_rank: u8, target: &User) -> bool {
    let target_rank = rank(target.role);
    (actor_id == target.id && target_rank <= actor_rank) || actor_rank > target_rank
}

/// Go `uid()` parity: a non-UUID id parses to `None` (reads as a miss → 404).
fn parse_id(raw: &str) -> Option<UserId> {
    uuid::Uuid::parse_str(raw).ok().map(UserId)
}

/// Go `payloadOf`: the compact patch projection for the audit payload — only
/// the `Some` arms appear (go's nil arms are omitted) and the role carries the
/// DB-vocabulary string (e.g. `"MODERATOR"`).
fn audit_payload(patch: &UserPatch) -> serde_json::Value {
    let mut p = serde_json::Map::new();
    if let Some(v) = patch.is_active {
        p.insert("is_active".into(), v.into());
    }
    if let Some(v) = patch.is_blocked {
        p.insert("is_blocked".into(), v.into());
    }
    if let Some(r) = patch.role {
        p.insert("role".into(), r.as_db_str().into());
    }
    serde_json::Value::Object(p)
}

impl AdminUsersService {
    /// Wire the service. `allowed_admin_emails` is the
    /// `<APP>_ALLOWED_ADMIN_EMAILS` whitelist behind the bootstrap upgrade
    /// (entries lowercased + trimmed, empties dropped — empty whitelist =
    /// the endpoint always denies).
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>, allowed_admin_emails: &[String]) -> Self {
        let whitelist = allowed_admin_emails
            .iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty())
            .collect::<HashSet<_>>();
        Self { users, whitelist }
    }

    /// One page plus the total row count (go `List`).
    ///
    /// # Errors
    /// Repo passthrough.
    pub async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<User>, i64)> {
        let rows = self.users.list_users(limit, offset).await?;
        let total = self.users.count_users().await?;
        Ok((rows, total))
    }

    /// Single fetch (go `Get`); a non-UUID or unknown id is a miss (404).
    ///
    /// # Errors
    /// [`Error::NotFound`] on a non-UUID or unknown id; repo passthrough.
    pub async fn get(&self, id: &str) -> Result<User> {
        let uid = parse_id(id).ok_or(Error::NotFound)?;
        self.users.get_user_by_id(uid).await?.ok_or(Error::NotFound)
    }

    /// Apply the patch, enforce self-or-subordinate and write the audit entry
    /// (go `Update`). Audit failure never blocks the mutation — it is logged
    /// loudly so the trail gap stays visible.
    ///
    /// `actor_id` + `actor_roles` come from the verified token (the go token
    /// carries one role; the rust codec carries a list, so the gate reads the
    /// strongest role — `.any()` parity with `require_admin`). `audits` is the
    /// audit port for this mutation (go wires `WriteAudit` into the slice
    /// store; `compose()` hands this service only the users repo).
    ///
    /// # Errors
    /// [`Error::Invalid`] (400) on an empty patch, [`Error::NotFound`] (404)
    /// on a non-UUID or unknown id **before** the write (never audited),
    /// [`Error::Forbidden`] (403) on a rank violation; repo passthrough.
    #[allow(clippy::too_many_arguments)] // go `Update` parity: the api handler passes each concern in one call
    pub async fn update(
        &self,
        actor_id: UserId,
        actor_roles: &[Role],
        target_id: &str,
        ip: Option<&str>,
        ua: Option<&str>,
        patch: UserPatch,
        audits: &dyn AuditLogRepository,
    ) -> Result<User> {
        if patch == UserPatch::default() {
            return Err(Error::Invalid("empty update".into()));
        }
        // Existence first: an unknown id is a miss and is never audited.
        let uid = parse_id(target_id).ok_or(Error::NotFound)?;
        let Some(target) = self.users.get_user_by_id(uid).await? else {
            return Err(Error::NotFound);
        };
        if !can_manage(actor_id, effective_rank(actor_roles), &target) {
            return Err(Error::Forbidden(INSUFFICIENT_ROLE.into()));
        }
        self.users.update(uid, &patch).await?;
        if let Err(e) = audits
            .write_audit(
                Some(actor_id),
                AUDIT_ACTION_UPDATED,
                AUDIT_ENTITY,
                Some(target_id),
                Some(audit_payload(&patch)),
                ip,
                ua,
            )
            .await
        {
            tracing::error!(
                action = AUDIT_ACTION_UPDATED,
                user_id = %target_id,
                error = %e,
                "audit write failed"
            );
        }
        self.get(target_id).await
    }

    /// Unauthenticated bootstrap upgrade (go `UpgradeToAdmin` parity): a
    /// whitelisted email may promote its own account to role + `user_type`
    /// ADMIN. The whitelist is checked before any user lookup, so a
    /// non-whitelisted caller cannot probe for account existence; a
    /// whitelisted but unknown email is still a miss (404).
    ///
    /// # Errors
    /// [`Error::Forbidden`] (403) on a non-whitelisted email,
    /// [`Error::NotFound`] (404) on a whitelisted but unknown email; repo
    /// passthrough.
    pub async fn upgrade_to_admin(&self, raw_email: &str) -> Result<AdminUpgradeResult> {
        let email = raw_email.trim().to_lowercase();
        if !self.whitelist.contains(&email) {
            return Err(Error::Forbidden(NOT_WHITELISTED.into()));
        }
        let Some(row) = self.users.get_user_by_email(&email).await? else {
            return Err(Error::NotFound);
        };
        if row.role == Role::Admin && row.user_type == UserType::Admin {
            return Ok(AdminUpgradeResult {
                email,
                message: "User is already an admin".into(),
                upgraded: false,
            });
        }
        self.users.upgrade_to_admin(row.id).await?;
        Ok(AdminUpgradeResult {
            message: format!("User '{email}' has been upgraded to admin"),
            email,
            upgraded: true,
        })
    }
}

#[cfg(test)]
mod tests;
