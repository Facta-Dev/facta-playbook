//! Identity domain vocabulary — the go identity-core port (spec: the go
//! template's `migrations/*.sql` + feature slice stores). Pure data, zero
//! I/O; table names in doc comments map 1:1 to the DDL in
//! `migrations/0002_identity_core.up.sql`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// Strongly-typed id. Newtype around UUID — the compiler rejects a
/// `UserId` passed where another id type is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserId(pub uuid::Uuid);

impl UserId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

/// One identity account (DB `users`). The narrower go per-slice projections
/// collapse onto this struct (go parity note: go keeps per-feature row
/// types).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: crate::rbac::Role,
    pub user_type: UserType,
    pub is_active: bool,
    pub is_blocked: bool,
    pub is_verified: bool,
    pub retry_count: i32,
    /// Argon2id hash (`password` column). Plain `String` — no newtype.
    pub password: String,
    pub language: String,
    pub profile_picture: Option<String>,
    pub google_id: Option<String>,
    pub github_id: Option<String>,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Account flavor (DB `users.user_type` CHECK `('ADMIN','COMPANY','USER')`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum UserType {
    Admin,
    Company,
    User,
}

impl UserType {
    /// Canonical DB string (`as_str` parity with [`crate::WidgetKind`]).
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Admin => "ADMIN",
            Self::Company => "COMPANY",
            Self::User => "USER",
        }
    }

    /// DB string back into a variant (`None` = unknown string).
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "ADMIN" => Some(Self::Admin),
            "COMPANY" => Some(Self::Company),
            "USER" => Some(Self::User),
            _ => None,
        }
    }
}

impl crate::rbac::Role {
    /// DB `users.role` CHECK strings (`ADMIN|MODERATOR|USER|GUEST`), go
    /// rolex parity. `Owner` has no DB string of its own — an owner row is
    /// stored as `ADMIN` (rank `>= ADMIN` is what the gates check).
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Admin | Self::Owner => "ADMIN",
            Self::Editor => "MODERATOR",
            Self::Viewer => "USER",
        }
    }

    /// DB string back into a rank. Provisional mapping per the scan's risk
    /// note: `GUEST` collapses onto `Viewer` (go ranks it distinct but the
    /// shipped `can()` matrix treats read-only the same).
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "ADMIN" => Some(Self::Admin),
            "MODERATOR" => Some(Self::Editor),
            "USER" | "GUEST" => Some(Self::Viewer),
            _ => None,
        }
    }
}

/// One browser session (DB `sessions`). `refresh_token` is the opaque
/// rotation token; `jti == id` (go parity), so deactivating the row revokes
/// the access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub refresh_token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub is_active: bool,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SessionId(pub uuid::Uuid);

/// One email-verification attempt (DB `email_verifications`, go
/// `authsignup.Verification`). `attempts` counts wrong-code guesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub id: uuid::Uuid,
    pub user_id: UserId,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub attempts: i32,
}

/// One password-reset attempt (DB `password_resets`, go `authpassword.Reset`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordReset {
    pub id: uuid::Uuid,
    pub user_id: UserId,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub attempts: i32,
}

/// A message slice code wants delivered (go `email.Message`) — enqueued
/// into the `email_jobs` outbox through the ports outbox contract.
#[allow(clippy::derive_partial_eq_without_eq)] // serde_json::Value: Eq is not implementable
#[derive(Debug, Clone, PartialEq)]
pub struct EmailMessage {
    pub to: String,
    pub template_key: String,
    pub vars: Json,
}

/// One queued outbox row (DB `email_jobs`, go `email.Job`). `status` stays
/// a plain string: `'queued'|'sending'|'sent'|'failed'` per the DB CHECK.
#[allow(clippy::derive_partial_eq_without_eq)] // serde_json::Value: Eq is not implementable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmailJob {
    pub id: uuid::Uuid,
    pub to: String,
    pub template_key: String,
    pub vars: Json,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// One session-hygiene anomaly (go `authsession.Anomaly`): expired rows
/// still marked active, or one user spanning too many IPs.
#[allow(clippy::derive_partial_eq_without_eq)] // serde_json::Value: Eq is not implementable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anomaly {
    /// Wire name `type` (go field `Type`); `kind` because `type` is a Rust
    /// keyword.
    #[serde(rename = "type")]
    pub kind: String,
    pub count: i64,
    pub details: Json,
}

/// `sessions`-table aggregation behind the multi-IP anomaly scan (go
/// `MultiIPSessions` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiIpRow {
    pub user_id: UserId,
    pub ip_count: i64,
}

#[cfg(test)]
mod tests {
    use super::UserType;

    #[test]
    fn user_type_db_twin_roundtrips() {
        for (v, s) in [
            (UserType::Admin, "ADMIN"),
            (UserType::Company, "COMPANY"),
            (UserType::User, "USER"),
        ] {
            assert_eq!(v.as_db_str(), s);
            assert_eq!(UserType::from_db_str(s), Some(v));
        }
        assert_eq!(UserType::from_db_str("NOPE"), None);
    }

    #[test]
    fn role_db_twin_roundtrips() {
        use crate::rbac::Role;
        for (v, s) in [
            (Role::Admin, "ADMIN"),
            (Role::Editor, "MODERATOR"),
            (Role::Viewer, "USER"),
        ] {
            assert_eq!(Role::as_db_str(v), s);
        }
        // GUEST is read-side only: it collapses onto Viewer, stored as "USER".
        assert_eq!(
            Role::as_db_str(Role::from_db_str("GUEST").expect("GUEST")),
            "USER"
        );
        assert_eq!(Role::from_db_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::from_db_str("GUEST"), Some(Role::Viewer));
        assert_eq!(Role::from_db_str("NOPE"), None);
    }
}
