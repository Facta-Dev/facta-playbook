//! Session flows — go `authsession` http.go parity: login/refresh/logout,
//! `me` (get/update), the caller's session management trio, and the admin
//! auth-session endpoints (target sessions, revoke-all, anomaly report).
//!
//! With the api crate's `default = ["mint"]` the login/refresh service access tokens, so the service call sits behind
//! a cfg shim — in a production (`non-mint`) build the route stays mounted
//! (the spec surface is stable) but answers 500 with a redacted detail.
use std::collections::BTreeMap;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use <app>_auth::{LoginInput, SessionTokens};
use <app>_core::{Error, UserId};
use <app>_ports::UpdateMeInput;
use <app>_warden::Principal;

use crate::auth::require_admin;
use crate::error::ApiError;
use crate::routes::{
    AnomalyItem, AuthTokens, MessageResponse, SessionItem, SessionRevoked, SessionRevokedCount,
    clear_auth_cookie_on, request_ip, set_auth_cookie_on, token_pair, user_agent,
};
use crate::state::AppState;

/// go `api.AuthLoginRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthLoginRequest {
    pub email: String,
    pub password: String,
}

/// go `api.AuthRefreshRequest` parity — the logout body carries the opaque
/// token too (go reuses the refresh shape).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthRefreshRequest {
    pub refresh_token: String,
}

/// go `api.UpdateMeRequest` parity — every field optional; maps 1:1 onto the
/// ports' [`UpdateMeInput`] under the go wire names.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateMeRequest {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub phone_number: Option<String>,
    #[serde(default)]
    pub profile_picture: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
}

impl From<UpdateMeRequest> for UpdateMeInput {
    fn from(r: UpdateMeRequest) -> Self {
        Self {
            first_name: r.first_name,
            last_name: r.last_name,
            phone: r.phone_number,
            picture: r.profile_picture,
            language: r.language,
            address: r.address,
            postal: r.postal_code,
        }
    }
}

/// `GET /api/v1/auth/me` body — go `api.MeResponse` parity (slice-local
/// name: the router's legacy `/me` already owns `MeResponse`).
/// `phone_number`/`subscription` are NOT carried: the landed `core::User`
/// row has no such columns (see the route-parity deviations).
#[derive(Debug, Serialize, ToSchema)]
// The four flags are the go `meResponse` JSON keys 1:1; the response schema
// mirrors the wire contract, which has no room for a state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct MeProfileResponse {
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
    pub profile_picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,
    /// go `GithubLinked` parity: linked ⇔ a non-empty github id.
    pub github_connected: bool,
    pub created_at: DateTime<Utc>,
}

/// go `meResponse(user)` parity.
pub(crate) fn me_response(u: <app>_core::User) -> MeProfileResponse {
    MeProfileResponse {
        id: u.id.0,
        github_connected: u.github_id.as_deref().is_some_and(|g| !g.is_empty()),
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
        profile_picture: u.profile_picture,
        last_login: u.last_login,
        created_at: u.created_at,
    }
}

/// Opt-out shim: the mint-calling login path compiles only under `mint`/`test` —
/// the wire surface stays mounted; a production build answers 500 with a
/// redacted detail (see `crate::error`).
async fn minted_login(state: &AppState, input: LoginInput) -> Result<SessionTokens, ApiError> {
    #[cfg(any(test, feature = "mint"))]
    {
        state.session.login(input).await.map_err(Into::into)
    }
    #[cfg(not(any(test, feature = "mint")))]
    {
        let _ = (state, input);
        Err(Error::Internal("minting is disabled in this build".to_owned()).into())
    }
}

/// Opt-out shim: see [`minted_login`].
async fn minted_refresh(
    state: &AppState,
    token: &str,
    ip: Option<&str>,
    ua: Option<&str>,
) -> Result<SessionTokens, ApiError> {
    #[cfg(any(test, feature = "mint"))]
    {
        state
            .session
            .refresh(token, ip, ua)
            .await
            .map_err(Into::into)
    }
    #[cfg(not(any(test, feature = "mint")))]
    {
        let _ = (state, token, ip, ua);
        Err(Error::Internal("minting is disabled in this build".to_owned()).into())
    }
}

/// go `handlers.setCookie(w, res.AccessToken)` + `tokenPair(res)` parity:
/// 200 with the token pair, plus the access-token cookie when cookie mode
/// is on.
fn tokens_with_cookie(state: &AppState, tokens: &SessionTokens) -> Response {
    let mut resp = Json(token_pair(tokens)).into_response();
    set_auth_cookie_on(&mut resp, &state.cfg, &tokens.access_token);
    resp
}

/// Login: retry-lock + state gates; 200 with the token pair (and the
/// access-token cookie when cookie mode is on).
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "authsession",
    request_body = AuthLoginRequest,
    responses(
        (status = 200, description = "Token pair", body = AuthTokens),
        (status = 401, description = "Unknown user or wrong password", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Account blocked, inactive, unverified or retry-locked", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AuthLoginRequest>,
) -> Result<Response, ApiError> {
    let tokens = minted_login(
        &state,
        LoginInput {
            email: req.email,
            password: req.password,
            ip_address: request_ip(&headers, None),
            user_agent: user_agent(&headers),
        },
    )
    .await?;
    Ok(tokens_with_cookie(&state, &tokens))
}

/// Refresh: CAS rotation of the opaque token, fresh access token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "authsession",
    request_body = AuthRefreshRequest,
    responses(
        (status = 200, description = "Fresh token pair", body = AuthTokens),
        (status = 401, description = "Invalid or replayed refresh token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Account blocked, inactive or unverified", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AuthRefreshRequest>,
) -> Result<Response, ApiError> {
    let tokens = minted_refresh(
        &state,
        &req.refresh_token,
        request_ip(&headers, None).as_deref(),
        user_agent(&headers).as_deref(),
    )
    .await?;
    Ok(tokens_with_cookie(&state, &tokens))
}

/// Logout: revoke the given refresh token (idempotent) + clear the cookie.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "authsession",
    request_body = AuthRefreshRequest,
    responses(
        (status = 200, description = "Logged out", body = MessageResponse),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<AuthRefreshRequest>,
) -> Result<Response, ApiError> {
    state
        .session
        .logout(UserId(principal.id), &req.refresh_token)
        .await?;
    let mut resp = Json(MessageResponse {
        message: "logged out".to_owned(),
    })
    .into_response();
    clear_auth_cookie_on(&mut resp, &state.cfg);
    Ok(resp)
}

/// The caller's profile row.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "authsession",
    responses(
        (status = 200, description = "Profile", body = MeProfileResponse),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn auth_me(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<MeProfileResponse>, ApiError> {
    Ok(Json(me_response(
        state.session.me(UserId(principal.id)).await?,
    )))
}

/// Partial profile update; returns the fresh row.
#[utoipa::path(
    put,
    path = "/api/v1/auth/me",
    tag = "authsession",
    request_body = UpdateMeRequest,
    responses(
        (status = 200, description = "Updated profile", body = MeProfileResponse),
        (status = 400, description = "Invalid input", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn auth_update_me(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<UpdateMeRequest>,
) -> Result<Json<MeProfileResponse>, ApiError> {
    let user = state
        .session
        .update_me(UserId(principal.id), req.into())
        .await?;
    Ok(Json(me_response(user)))
}

/// go `queryBool` parity: absent → the default; anything but a real bool is
/// `active_only must be a boolean` (go 422 → [`Error::Invalid`], 400).
pub(crate) fn query_bool(
    params: &BTreeMap<String, String>,
    key: &str,
    def: bool,
) -> Result<bool, ApiError> {
    params
        .get(key)
        .filter(|v| !v.is_empty())
        .map_or(Ok(def), |v| {
            v.parse::<bool>()
                .map_err(|_| Error::Invalid(format!("{key} must be a boolean")).into())
        })
}

/// The caller's sessions, newest activity first.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    tag = "authsession",
    params(("active_only" = bool, Query, description = "Only active rows (default true)")),
    responses(
        (status = 200, description = "Session list", body = Vec<SessionItem>),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Query(params): Query<BTreeMap<String, String>>,
) -> Result<Json<Vec<SessionItem>>, ApiError> {
    let active_only = query_bool(&params, "active_only", true)?;
    let rows = state
        .session
        .sessions(UserId(principal.id), active_only)
        .await?;
    Ok(Json(rows.iter().map(SessionItem::from).collect()))
}

/// The caller's most recent active session — `null` body when none.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions/current",
    tag = "authsession",
    responses(
        (status = 200, description = "Current session or null", body = Option<SessionItem>),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn current_session(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<Option<SessionItem>>, ApiError> {
    let row = state.session.current_session(UserId(principal.id)).await?;
    Ok(Json(row.as_ref().map(SessionItem::from)))
}

/// Revoke the caller's CURRENT session: the access token's `jti` is the
/// session id (go parity), so the verified token is re-read for its `jti`
/// ([`crate::auth::resolve_current_claims`]).
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke-current",
    tag = "authsession",
    responses(
        (status = 200, description = "Revoke outcome", body = SessionRevoked),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn revoke_current_session(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    headers: HeaderMap,
) -> Result<Json<SessionRevoked>, ApiError> {
    let claims = crate::auth::resolve_current_claims(&headers, &state.cfg, &state.jwt_codec)?;
    let revoked = state
        .session
        .revoke_current(UserId(principal.id), &claims.jti)
        .await?;
    Ok(Json(SessionRevoked { revoked }))
}

/// Revoke every active session of the caller.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke-all",
    tag = "authsession",
    responses(
        (status = 200, description = "Revoked count", body = SessionRevokedCount),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn revoke_all_sessions(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<SessionRevokedCount>, ApiError> {
    let n = state.session.revoke_all(UserId(principal.id)).await?;
    Ok(Json(SessionRevokedCount { revoked_count: n }))
}

/// go admin-gate parity: RequireRole(Admin) renders 403 "insufficient role".
fn gate_admin(principal: &Principal) -> Result<(), ApiError> {
    require_admin(principal).map_err(Into::into)
}

/// The MAX anomaly scan refuses windows outside `(0, 30d]` (go
/// `maxAnomalyWindow`); the route guard reports the go message verbatim.
const MAX_ANOMALY_WINDOW_MINUTES: i64 = 43_200;

/// Admin: one target user's sessions (`?active_only=`, default true). A
/// non-UUID target id reads as an empty list (go lenient NULL behavior).
#[utoipa::path(
    get,
    path = "/api/v1/admin/auth-session/users/{id}/sessions",
    tag = "authsession",
    params(
        ("id" = String, Path, description = "Target user id"),
        ("active_only" = bool, Query, description = "Only active rows (default true)"),
    ),
    responses(
        (status = 200, description = "Target session list", body = Vec<SessionItem>),
        (status = 403, description = "Not an admin caller", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn admin_list_sessions(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    Query(params): Query<BTreeMap<String, String>>,
) -> Result<Json<Vec<SessionItem>>, ApiError> {
    gate_admin(&principal)?;
    let active_only = query_bool(&params, "active_only", true)?;
    let Ok(uid) = uuid::Uuid::parse_str(&id) else {
        return Ok(Json(Vec::new()));
    };
    let rows = state.session.sessions(UserId(uid), active_only).await?;
    Ok(Json(rows.iter().map(SessionItem::from).collect()))
}

/// Admin: revoke every active session of one target user. A non-UUID id
/// revokes nothing (go lenient NULL behavior).
#[utoipa::path(
    post,
    path = "/api/v1/admin/auth-session/users/{id}/revoke-all",
    tag = "authsession",
    params(("id" = String, Path, description = "Target user id")),
    responses(
        (status = 200, description = "Revoked count", body = SessionRevokedCount),
        (status = 403, description = "Not an admin caller", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn admin_revoke_all_sessions(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<SessionRevokedCount>, ApiError> {
    gate_admin(&principal)?;
    let n = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => state.session.revoke_all(UserId(uid)).await?,
        Err(_) => 0,
    };
    Ok(Json(SessionRevokedCount { revoked_count: n }))
}

/// Admin session-hygiene report (`?window_minutes=`, default 60; the range
/// is `(0, 43200]` — go 422 → [`Error::Invalid`], 400).
#[utoipa::path(
    get,
    path = "/api/v1/admin/auth-session/anomalies",
    tag = "authsession",
    params(("window_minutes" = i64, Query, description = "Scan window in minutes; 60 by default")),
    responses(
        (status = 200, description = "Anomaly report", body = Vec<AnomalyItem>),
        (status = 400, description = "window_minutes out of range", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Not an admin caller", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn admin_anomalies(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Query(params): Query<BTreeMap<String, String>>,
) -> Result<Json<Vec<AnomalyItem>>, ApiError> {
    gate_admin(&principal)?;
    let window = match params.get("window_minutes").filter(|v| !v.is_empty()) {
        None => 60,
        Some(v) => match v.parse::<i64>() {
            Ok(n) if n > 0 && n <= MAX_ANOMALY_WINDOW_MINUTES => n,
            _ => {
                return Err(Error::Invalid("window_minutes must be 1..43200".to_owned()).into());
            }
        },
    };
    let rows = state
        .session
        .anomalies(chrono::Duration::minutes(window))
        .await?;
    Ok(Json(rows.into_iter().map(AnomalyItem::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_bool_defaults_and_refuses_garbage() {
        let mut params = BTreeMap::new();
        assert!(query_bool(&params, "active_only", true).expect("default"));
        params.insert("active_only".to_owned(), "false".to_owned());
        assert!(!query_bool(&params, "active_only", true).expect("false"));
        params.insert("active_only".to_owned(), "nah".to_owned());
        let err = query_bool(&params, "active_only", true).unwrap_err();
        assert_eq!(err.inner.status_code(), 400);
    }

    #[test]
    fn me_response_maps_the_db_strings_and_github_flag() {
        let mut u = <app>_core::User {
            id: <app>_core::UserId(uuid::Uuid::new_v4()),
            email: "me@test.dev".to_owned(),
            first_name: "A".to_owned(),
            last_name: "B".to_owned(),
            role: <app>_core::Role::Viewer,
            user_type: <app>_core::UserType::Company,
            is_active: true,
            is_blocked: false,
            is_verified: true,
            retry_count: 0,
            password: String::new(),
            language: "en".to_owned(),
            profile_picture: None,
            google_id: None,
            github_id: None,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let wire = serde_json::to_value(me_response(u.clone())).expect("wire");
        assert_eq!(wire["role"], "USER");
        assert_eq!(wire["user_type"], "COMPANY");
        assert_eq!(wire["github_connected"], false);
        u.github_id = Some("gh-1".to_owned());
        u.role = <app>_core::Role::Owner;
        let wire = serde_json::to_value(me_response(u)).expect("wire");
        assert_eq!(wire["github_connected"], true);
        assert_eq!(
            wire["role"], "ADMIN",
            "owner rows read as ADMIN (go parity)"
        );
    }
}
