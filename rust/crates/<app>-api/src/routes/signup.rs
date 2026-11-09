//! Signup / verify / resend — go `authsignup` http.go parity. Unauthenticated
//! routes; 201 only for a fresh signup (go parity).
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;

use <app>_auth::SignupInput;

use crate::error::ApiError;
use crate::routes::{EmailRequest, MessageResponse};
use crate::state::AppState;

/// go `api.SignupRequest` parity — optional fields default to the empty
/// string, exactly as the go pointer-model decodes them (`user_type` is
/// passed through verbatim; the service normalizes it case-insensitively).
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub user_type: String,
}

/// go `api.VerifyEmailRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub code: String,
}

/// Register an account: unverified + a 6-digit code in the outbox. A
/// re-signup of an unverified address re-issues the code (201); a verified
/// address conflicts (409).
#[utoipa::path(
    post,
    path = "/api/v1/auth/signup",
    tag = "authsignup",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "Signup accepted", body = MessageResponse),
        (status = 400, description = "Invalid email/password/user type", body = crate::error::ErrorEnvelope),
        (status = 409, description = "Email already registered", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    state
        .signup
        .signup(SignupInput {
            email: req.email,
            first_name: req.first_name,
            last_name: req.last_name,
            password: req.password,
            language: req.language,
            user_type: req.user_type,
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            message: "signup successful".to_owned(),
        }),
    ))
}

/// Consume the 6-digit code: flips `is_verified`.
#[utoipa::path(
    post,
    path = "/api/v1/auth/verify",
    tag = "authsignup",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified", body = MessageResponse),
        (status = 400, description = "Code invalid, used or expired", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state.signup.verify_email(&req.email, &req.code).await?;
    Ok(Json(MessageResponse {
        message: "email verified".to_owned(),
    }))
}

/// Re-issue a fresh code (unknown + verified addresses read as success —
/// no existence leak).
#[utoipa::path(
    post,
    path = "/api/v1/auth/verify/resend",
    tag = "authsignup",
    request_body = EmailRequest,
    responses(
        (status = 200, description = "A fresh code was sent (when the account exists)", body = MessageResponse),
    )
)]
pub async fn resend(
    State(state): State<AppState>,
    Json(req): Json<EmailRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    state.signup.resend_verification(&req.email).await?;
    Ok(Json(MessageResponse {
        message: "verification code sent".to_owned(),
    }))
}
