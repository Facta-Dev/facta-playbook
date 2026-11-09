//! Admin roster routes — go `adminusers` http.go parity: the admin-gated
//! user list / detail / update and the UNAUTHENTICATED bootstrap
//! `upgrade-to-admin` (the `<APP>_ALLOWED_ADMIN_EMAILS` whitelist is its
//! only gate).
use std::collections::BTreeMap;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use <app>_core::{Error, Role, User, UserId};
use <app>_ports::UserPatch;
use <app>_warden::Principal;

use crate::auth::require_admin;
use crate::error::ApiError;
use crate::routes::{request_ip, user_agent};
use crate::state::AppState;

/// go `api.AdminUser` parity (slice-local mapping, DB strings for enums).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUser {
    pub id: uuid::Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    /// DB role string (`ADMIN|MODERATOR|USER`).
    pub role: String,
    /// DB user-type string (`ADMIN|COMPANY|USER`).
    pub user_type: String,
    pub is_active: bool,
    pub is_blocked: bool,
    pub is_verified: bool,
    pub retry_count: i32,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// go `api.AdminUserList` parity.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserList {
    pub items: Vec<AdminUser>,
    pub total: i64,
}

/// go `adminModel(row)` parity.
pub(crate) fn admin_user(u: User) -> AdminUser {
    AdminUser {
        id: u.id.0,
        email: u.email,
        first_name: u.first_name,
        last_name: u.last_name,
        role: u.role.as_db_str().to_owned(),
        user_type: u.user_type.as_db_str().to_owned(),
        is_active: u.is_active,
        is_blocked: u.is_blocked,
        is_verified: u.is_verified,
        retry_count: u.retry_count,
        language: u.language,
        last_login: u.last_login,
        created_at: u.created_at,
        updated_at: u.updated_at,
    }
}

/// go `api.AdminUpdateUser` parity — every field optional; an all-empty
/// patch is the go "invalid request".
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AdminUpdateUser {
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub is_blocked: Option<bool>,
    /// DB role string; `""`/absent leaves the role untouched (go `""`).
    #[serde(default)]
    pub role: Option<String>,
}

/// go `api.AuthUpgradeResponse` parity.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthUpgradeResponse {
    pub email: String,
    pub message: String,
    /// `false` reads as "the account already was an admin".
    pub upgraded: bool,
}

/// go `httpx.QueryPage` parity: `limit` default 20 clamped to 1..100,
/// `offset` default 0 with `offset >= 0`; out-of-range reads as
/// [`Error::Invalid`] (go 422 → 400 per the template's documented
/// deviation).
pub(crate) fn query_page(params: &BTreeMap<String, String>) -> Result<(i64, i64), ApiError> {
    let limit = match params.get("limit").filter(|v| !v.is_empty()) {
        None => 20,
        Some(v) => {
            let n: i64 = v
                .parse()
                .map_err(|_| Error::Invalid("limit must be 1..100".to_owned()))?;
            if !(1..=100).contains(&n) {
                return Err(Error::Invalid("limit must be 1..100".to_owned()).into());
            }
            n
        }
    };
    let offset = match params.get("offset").filter(|v| !v.is_empty()) {
        None => 0,
        Some(v) => {
            let n: i64 = v
                .parse()
                .map_err(|_| Error::Invalid("offset must be >= 0".to_owned()))?;
            if n < 0 {
                return Err(Error::Invalid("offset must be >= 0".to_owned()).into());
            }
            n
        }
    };
    Ok((limit, offset))
}

/// List users: `?limit=&offset=` (go `QueryPage` semantics).
#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    tag = "adminusers",
    params(
        ("limit" = i64, Query, description = "Page size, 1..100 (default 20)"),
        ("offset" = i64, Query, description = "Page offset (default 0)"),
    ),
    responses(
        (status = 200, description = "User page", body = AdminUserList),
        (status = 400, description = "Out-of-range limit/offset", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Not an admin caller", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Query(params): Query<BTreeMap<String, String>>,
) -> Result<Json<AdminUserList>, ApiError> {
    require_admin(&principal)?;
    let (limit, offset) = query_page(&params)?;
    let (rows, total) = state.admin_users.list(limit, offset).await?;
    Ok(Json(AdminUserList {
        items: rows.into_iter().map(admin_user).collect(),
        total,
    }))
}

/// Fetch one user by id (go `uuidPtr` semantics: unknown or non-uuid → 404
/// from the service's parse gate).
#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{id}",
    tag = "adminusers",
    params(("id" = String, Path, description = "Target user id")),
    responses(
        (status = 200, description = "The user row", body = AdminUser),
        (status = 404, description = "Unknown id", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Not an admin caller", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<AdminUser>, ApiError> {
    require_admin(&principal)?;
    Ok(Json(admin_user(state.admin_users.get(&id).await?)))
}

/// go `roleString` parity: absent/empty leaves the role untouched; anything
/// else must be a DB role string (go's oapi enum — invalid → 400 here).
fn patch_role(raw: Option<&str>) -> Result<Option<Role>, ApiError> {
    raw.filter(|s| !s.is_empty()).map_or(Ok(None), |s| {
        Role::from_db_str(s)
            .map(Some)
            .ok_or_else(|| Error::Invalid("invalid request".to_owned()).into())
    })
}

/// Update one user's flags/role (rank-gated + audited in the service).
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}",
    tag = "adminusers",
    params(("id" = String, Path, description = "Target user id")),
    request_body = AdminUpdateUser,
    responses(
        (status = 200, description = "Updated row", body = AdminUser),
        (status = 400, description = "Empty or invalid patch", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Below the target's rank", body = crate::error::ErrorEnvelope),
        (status = 404, description = "Unknown id", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AdminUpdateUser>,
) -> Result<Json<AdminUser>, ApiError> {
    require_admin(&principal)?;
    let patch = UserPatch {
        is_active: req.is_active,
        is_blocked: req.is_blocked,
        role: patch_role(req.role.as_deref())?,
    };
    let row = state
        .admin_users
        .update(
            UserId(principal.id),
            &principal.roles,
            &id,
            request_ip(&headers, None).as_deref(),
            user_agent(&headers).as_deref(),
            patch,
            state.audits_repo.as_ref(),
        )
        .await?;
    Ok(Json(admin_user(row)))
}

/// Unauthenticated bootstrap: a whitelisted email promotes its own account
/// to ADMIN. The empty whitelist disables the route entirely (403).
#[utoipa::path(
    post,
    path = "/api/v1/auth/upgrade-to-admin",
    tag = "adminusers",
    request_body = crate::routes::EmailRequest,
    responses(
        (status = 200, description = "Upgrade outcome", body = AuthUpgradeResponse),
        (status = 403, description = "Email not whitelisted or the route disabled", body = crate::error::ErrorEnvelope),
        (status = 404, description = "Whitelisted but unknown email", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn upgrade_to_admin(
    State(state): State<AppState>,
    Json(req): Json<crate::routes::EmailRequest>,
) -> Result<Json<AuthUpgradeResponse>, ApiError> {
    let res = state.admin_users.upgrade_to_admin(&req.email).await?;
    Ok(Json(AuthUpgradeResponse {
        email: res.email,
        message: res.message,
        upgraded: res.upgraded,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_page_matches_go_semantics() {
        let p = BTreeMap::new();
        assert_eq!(query_page(&p).expect("defaults"), (20, 0));
        let mut p = BTreeMap::new();
        p.insert("limit".to_owned(), "5".to_owned());
        p.insert("offset".to_owned(), "10".to_owned());
        assert_eq!(query_page(&p).expect("ok"), (5, 10));
        for (k, v) in [("limit", "0"), ("limit", "101"), ("limit", "x")] {
            let mut p = BTreeMap::new();
            p.insert(k.to_owned(), v.to_owned());
            let e = query_page(&p).unwrap_err();
            assert_eq!(e.inner.status_code(), 400);
        }
        let mut p = BTreeMap::new();
        p.insert("offset".to_owned(), "-1".to_owned());
        assert_eq!(query_page(&p).unwrap_err().inner.status_code(), 400);
    }

    #[test]
    fn patch_role_matches_go_string_parity() {
        assert!(patch_role(None).expect("none").is_none());
        assert!(patch_role(Some("")).expect("empty").is_none());
        assert_eq!(patch_role(Some("ADMIN")).expect("admin"), Some(Role::Admin));
        let e = patch_role(Some("nope")).unwrap_err();
        assert_eq!(e.inner.status_code(), 400);
    }
}
