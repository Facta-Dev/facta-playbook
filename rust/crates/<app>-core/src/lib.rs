//! Domain vocabulary for `<app>`. Zero I/O. Depends on nothing but serde/uuid.
//!
//! ponytail: `widgets` is an example domain entity. Replace with your own
//! `<scope>` entities (e.g. `accounts`, `orders`). Keep this crate pure: no
//! `async`, no `sqlx`, no HTTP.

pub mod identity;
pub mod rbac;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use identity::{
    Anomaly, EmailJob, EmailMessage, MultiIpRow, PasswordReset, Session, SessionId, User, UserId,
    UserType, Verification,
};
pub use rbac::{Action, Role, can};

/// Strongly-typed id. A newtype around UUID prevents passing a `WidgetId`
/// where another id is expected — the compiler enforces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WidgetId(pub uuid::Uuid);

impl WidgetId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

/// A closed vocabulary. Mirror your DB enum here so the two never drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum WidgetKind {
    Sprocket,
    Cog,
    Flange,
}

impl WidgetKind {
    /// The canonical DB string for this variant. Used by the Postgres adapter
    /// when binding INSERT/UPDATE columns and by row mapping on the way back.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sprocket => "sprocket",
            Self::Cog => "cog",
            Self::Flange => "flange",
        }
    }
}

/// A domain entity — plain data, no behavior that needs the outside world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Widget {
    pub id: WidgetId,
    pub name: String,
    pub kind: WidgetKind,
}

/// The one error type the whole workspace speaks. Adapters map their foreign
/// errors (sqlx, reqwest) into these variants at the boundary.
#[derive(Debug, Error)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("unauthorized")]
    Unauthorized,
    /// A user token/credential was missing, malformed, expired or revoked.
    #[error("authentication required: {0}")]
    Unauthenticated(String),
    /// Authenticated but not allowed.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// A uniqueness/state conflict (e.g. duplicate name).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Rate-limited (per-minute burst). Maps to 429 + `Retry-After`.
    #[error("rate limited: retry in {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },
    /// Quota exhausted (accumulated volume, e.g. monthly). Maps to 429.
    #[error("quota exceeded: used {used} of {quota}")]
    QuotaExceeded {
        /// Total allowance for the caller's tier.
        quota: i64,
        /// Units already consumed before this op.
        used: i64,
        /// RFC3339 instant when the window resets.
        reset_at: String,
    },
    #[error("upstream {service}: {detail}")]
    Upstream {
        service: &'static str,
        detail: String,
    },
    #[error("internal: {0}")]
    Internal(String),
}

impl Error {
    /// Suggested HTTP status code. Used by the API layer's `IntoResponse`.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Invalid(_) => 400,
            Self::Unauthorized | Self::Unauthenticated(_) => 401,
            Self::Forbidden(_) => 403,
            Self::Conflict(_) => 409,
            Self::RateLimited { .. } | Self::QuotaExceeded { .. } => 429,
            Self::Upstream { .. } => 502,
            Self::Internal(_) => 500,
        }
    }
}

/// Workspace-wide `Result`. Every port method returns this.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_id_is_unique() {
        let a = WidgetId::new();
        let b = WidgetId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn widget_kind_roundtrips_as_str() {
        assert_eq!(WidgetKind::Sprocket.as_str(), "sprocket");
        assert_eq!(WidgetKind::Cog.as_str(), "cog");
        assert_eq!(WidgetKind::Flange.as_str(), "flange");
    }

    #[test]
    fn status_code_maps_each_variant() {
        assert_eq!(Error::NotFound.status_code(), 404);
        assert_eq!(Error::Invalid("x".into()).status_code(), 400);
        assert_eq!(Error::Unauthorized.status_code(), 401);
        assert_eq!(Error::Unauthenticated("x".into()).status_code(), 401);
        assert_eq!(Error::Forbidden("x".into()).status_code(), 403);
        assert_eq!(Error::Conflict("x".into()).status_code(), 409);
        assert_eq!(
            Error::RateLimited {
                retry_after_secs: 30
            }
            .status_code(),
            429
        );
        assert_eq!(
            Error::QuotaExceeded {
                quota: 100,
                used: 100,
                reset_at: "2026-08-01T00:00:00Z".into()
            }
            .status_code(),
            429
        );
        assert_eq!(
            Error::Upstream {
                service: "db",
                detail: "down".into()
            }
            .status_code(),
            502
        );
        assert_eq!(Error::Internal("oops".into()).status_code(), 500);
    }
}
