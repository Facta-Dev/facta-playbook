//! Password flows — go `authpassword` http.go parity: forgot (rate-limited,
//! no existence leak), validate (check without consuming), reset, and the
//! authed change (the caller's `jti` session survives revocation).
use axum::{Extension, Json, extract::State, http::HeaderMap};
use serde::Deserialize;
use utoipa::ToSchema;

use <app>_core::{Error, SessionId, UserId};
use <app>_warden::Principal;

use crate::auth::resolve_current_claims;
use crate::error::ApiError;
use crate::routes::{EmailRequest, MessageResponse};
use crate::state::AppState;

/// go `api.ForgotPasswordRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// go `api.ValidateCodeRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateCodeRequest {
    pub email: String,
    pub code: String,
}

/// go `api.ResetPasswordRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

/// go `api.ChangePasswordRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

const RESET_SENT: &str = "if the account exists, a reset code was sent";

/// Request a reset code (rate-limited per email; unknown addresses read as
/// success — no existence leak).
#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    tag = "authpassword",
    request_body = EmailRequest,
    responses(
        (status = 200, description = "Code sent when the account exists", body = MessageResponse),
        (status = 429, description = "Too many codes requested", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<EmailRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state.password.forgot(&req.email).await?;
    Ok(Json(MessageResponse {
        message: RESET_SENT.to_owned(),
    }))
}

/// Check a code WITHOUT consuming it (go `Validate`).
#[utoipa::path(
    post,
    path = "/api/v1/auth/validate-code",
    tag = "authpassword",
    request_body = ValidateCodeRequest,
    responses(
        (status = 200, description = "Code accepted", body = MessageResponse),
        (status = 400, description = "Code invalid, used, expired or exhausted", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn validate_code(
    State(state): State<AppState>,
    Json(req): Json<ValidateCodeRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state.password.validate(&req.email, &req.code).await?;
    Ok(Json(MessageResponse {
        message: "code is valid".to_owned(),
    }))
}

/// Consume the code + set the new password (auto-verify + optional
/// revoke-all).
#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    tag = "authpassword",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password updated", body = MessageResponse),
        (status = 400, description = "Code or new password refused", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state
        .password
        .reset(&req.email, &req.code, &req.new_password)
        .await?;
    Ok(Json(MessageResponse {
        message: "password updated".to_owned(),
    }))
}

/// Authed change: the current password is re-verified and every OTHER
/// active session is revoked — the caller's own survives via
/// `jti == sessions.id` (the verified token is re-read for its `jti`).
#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    tag = "authpassword",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password updated", body = MessageResponse),
        (status = 400, description = "New password too short", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Wrong current password", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    // The middleware has already verified this token; re-reading the same
    // token's claims is the only way to reach its `jti` (the session id)
    // from the handler — the `Principal` keeps only (id, roles).
    let claims = resolve_current_claims(&headers, &state.cfg, &state.jwt_codec)?;
    let session_id = SessionId(
        uuid::Uuid::parse_str(&claims.jti)
            .map_err(|_| Error::Unauthenticated("invalid or expired token".to_owned()))?,
    );
    state
        .password
        .change(
            UserId(principal.id),
            session_id,
            &req.current_password,
            &req.new_password,
        )
        .await?;
    Ok(Json(MessageResponse {
        message: "password updated".to_owned(),
    }))
}
