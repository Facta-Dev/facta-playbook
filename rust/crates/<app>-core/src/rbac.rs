//! Role-based access control. One function — `can(role, action)` — decides
//! every authorization question. No role inheritance: the matrix is explicit,
//! so reading one `match` arm tells you exactly what a role may do.
//!
//! ponytail: `Role` and `Action` are generic placeholders. Replace them with
//! your own `<scope>` vocabulary (e.g. `OrderRead`, `FinanceAdmin`). Keep
//! `can` as a `const fn` so it compiles to a jump table and can be used in
//! const contexts. Mirror the frontend `can()` so action names stay in sync.

use serde::{Deserialize, Serialize};

/// A capability a caller may or may not have. Coarse-grained on purpose; add
/// arms as the product grows, and update the `can()` matrix in the same commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Action {
    /// Read any resource in the `<scope>`.
    Read,
    /// Create new resources.
    Create,
    /// Update existing resources.
    Update,
    /// Delete resources.
    Delete,
    /// System-level administration (config, user management).
    Admin,
}

/// A caller's role. Drives every `can()` decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Role {
    /// Read-only access.
    Viewer,
    /// Read + create + update (no delete, no admin).
    Editor,
    /// Full CRUD on `<scope>` resources (no system admin).
    Admin,
    /// Unrestricted — every `can()` returns `true`.
    Owner,
}

/// The single authorization decision point. `const fn` so it compiles to a
/// jump table and can be used in const contexts. Explicit per (role, action) —
/// no wildcards that silently grant new actions to a role.
///
/// `match_same_arms` is allowed on purpose: each arm documents a *distinct*
/// capability that happens to grant access. Collapsing unrelated capabilities
/// into one `|` arm to dedup the `true` bodies would destroy the readability
/// that is the entire point of an explicit RBAC matrix.
#[must_use]
#[allow(clippy::match_same_arms)]
pub const fn can(role: Role, action: Action) -> bool {
    use Action as A;
    use Role as R;

    // Owner can do everything.
    if matches!(role, R::Owner) {
        return true;
    }

    match (role, action) {
        // Everyone can read.
        (_, A::Read) => true,

        // Editor: read + create + update (no delete, no admin).
        (R::Editor, A::Create | A::Update) => true,

        // Admin: full CRUD on scope resources (no system admin).
        (R::Admin, A::Create | A::Update | A::Delete) => true,

        // Everything else is denied.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_can_do_everything() {
        for a in [
            Action::Read,
            Action::Create,
            Action::Update,
            Action::Delete,
            Action::Admin,
        ] {
            assert!(can(Role::Owner, a), "owner denied {a:?}");
        }
    }

    #[test]
    fn viewer_can_only_read() {
        assert!(can(Role::Viewer, Action::Read));
        assert!(!can(Role::Viewer, Action::Create));
        assert!(!can(Role::Viewer, Action::Update));
        assert!(!can(Role::Viewer, Action::Delete));
        assert!(!can(Role::Viewer, Action::Admin));
    }

    #[test]
    fn editor_can_create_and_update_but_not_delete() {
        assert!(can(Role::Editor, Action::Read));
        assert!(can(Role::Editor, Action::Create));
        assert!(can(Role::Editor, Action::Update));
        assert!(!can(Role::Editor, Action::Delete));
        assert!(!can(Role::Editor, Action::Admin));
    }

    #[test]
    fn admin_can_crud_but_not_system_admin() {
        assert!(can(Role::Admin, Action::Read));
        assert!(can(Role::Admin, Action::Create));
        assert!(can(Role::Admin, Action::Update));
        assert!(can(Role::Admin, Action::Delete));
        assert!(!can(Role::Admin, Action::Admin));
    }

    #[test]
    fn only_owner_can_admin() {
        assert!(can(Role::Owner, Action::Admin));
        assert!(!can(Role::Admin, Action::Admin));
        assert!(!can(Role::Editor, Action::Admin));
        assert!(!can(Role::Viewer, Action::Admin));
    }
}
