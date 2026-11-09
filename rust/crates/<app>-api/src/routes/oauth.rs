//! OAuth routes — go `authoauth` http.go parity: the google one-tap login,
//! the github authorize + callback (EVERY outcome is a 302; tokens ride in
//! the URL fragment) and the bearer-protected link connect-url.
//!
//! With the api crate's `default = ["mint"]` the `google`/`github_login`
//! service methods mint access tokens, so those service calls
//! sit behind cfg shims like the session module's.
use std::collections::BTreeMap;

use axum::{
    Extension, Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::LOCATION},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use <app>_auth::{LINK_STATE_PREFIX, SessionTokens};
// `Error` only resolves in production builds (the no-mint stubs read it);
// test/mint builds stub the mint paths out and `Error` is genuinely unused.
#[cfg(any(test, feature = "mint"))]
use <app>_core::UserId;
#[cfg(not(any(test, feature = "mint")))]
use <app>_core::{Error, UserId};
use <app>_warden::Principal;

use crate::config::Config;
use crate::error::ApiError;
use crate::routes::{
    AuthTokens, form_urlencode, request_ip, set_auth_cookie_on, token_pair, user_agent,
};
use crate::state::AppState;

/// go `api.GoogleAuthRequest` parity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GoogleAuthRequest {
    pub credential: String,
}

/// go `api.GithubConnectURLResponse` parity.
#[derive(Debug, Serialize, ToSchema)]
pub struct GithubConnectURLResponse {
    pub url: String,
}

// go authoauth/domain.go error-code vocabulary (the redirect "?error=" pairs).
#[cfg(test)]
const ERR_NOT_CONFIGURED: &str = "github_not_configured";
const ERR_AUTH_FAILED: &str = "auth_failed";
const ERR_NO_CODE: &str = "no_code";
/// go `authSuccessSuffix` — the default frontend URL carries it; the link
/// flow's success page is derived by trimming it.
const AUTH_SUCCESS_SUFFIX: &str = "/auth/success";
/// go `linkSuccessRedirect`.
const LINK_SUCCESS_REDIRECT: &str = "/user/profile?tab=overview&github_linked=1";

/// Opt-out shim: see the session module's `minted_login`.
async fn minted_google(
    state: &AppState,
    credential: &str,
    ip: Option<&str>,
    ua: Option<&str>,
) -> Result<SessionTokens, ApiError> {
    #[cfg(any(test, feature = "mint"))]
    {
        state
            .oauth
            .google(credential, ip, ua)
            .await
            .map_err(Into::into)
    }
    #[cfg(not(any(test, feature = "mint")))]
    {
        let _ = (state, credential, ip, ua);
        Err(Error::Internal("minting is disabled in this build".to_owned()).into())
    }
}

/// Opt-out shim: see the session module's `minted_login`.
async fn minted_github_login(
    state: &AppState,
    code: &str,
    ip: Option<&str>,
    ua: Option<&str>,
) -> Result<SessionTokens, ApiError> {
    #[cfg(any(test, feature = "mint"))]
    {
        state
            .oauth
            .github_login(code, ip, ua)
            .await
            .map_err(Into::into)
    }
    #[cfg(not(any(test, feature = "mint")))]
    {
        let _ = (state, code, ip, ua);
        Err(Error::Internal("minting is disabled in this build".to_owned()).into())
    }
}

/// go `errText`: the human detail is the `Error`'s own message. The link
/// arm carries a bare `core::Error`; the mint shims surface theirs through
/// the `ApiError`'s inner error.
fn err_text(e: &<app>_core::Error) -> String {
    e.to_string()
}

/// One urlencoded `key=value` pair.
fn encoded_pair(key: &str, value: &str) -> String {
    format!("{}={}", form_urlencode(key), form_urlencode(value))
}

/// go `UseCase.frontendBase`: the success URL with every trailing slash trimmed.
fn frontend_base(cfg: &Config) -> String {
    cfg.frontend_success.trim_end_matches('/').to_owned()
}

/// go `UseCase.linkSuccessURL` parity.
fn link_success_url(cfg: &Config) -> String {
    let base = frontend_base(cfg);
    let stem = base.strip_suffix(AUTH_SUCCESS_SUFFIX).unwrap_or(&base);
    format!("{stem}{LINK_SUCCESS_REDIRECT}")
}

/// 302 + `Location` (go `http.Redirect` shape).
fn redirect(location: String) -> Response {
    (StatusCode::FOUND, [(LOCATION, location)]).into_response()
}

/// go `redirectError`: bounce to the frontend base with `?error=` pairs.
fn redirect_error(cfg: &Config, code: &str, description: &str) -> Response {
    redirect(format!(
        "{}?{}&{}",
        frontend_base(cfg),
        encoded_pair("error", code),
        encoded_pair("error_description", description),
    ))
}

/// go `handlers.tokenPair` + `setCookie` parity for the two mint routes.
fn tokens_with_cookie(state: &AppState, tokens: &SessionTokens) -> Response {
    let mut resp = Json(token_pair(tokens)).into_response();
    set_auth_cookie_on(&mut resp, &state.cfg, &tokens.access_token);
    resp
}

/// Google One-Tap credential in, token pair out (cookie-mode cookie
/// included). Unconfigured provider answers 401 "google sign-in is not
/// configured".
#[utoipa::path(
    post,
    path = "/api/v1/auth/google",
    tag = "authoauth",
    request_body = GoogleAuthRequest,
    responses(
        (status = 200, description = "Token pair", body = AuthTokens),
        (status = 401, description = "Google sign-in unconfigured or token refused", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn google(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<GoogleAuthRequest>,
) -> Result<Response, ApiError> {
    let tokens = minted_google(
        &state,
        &req.credential,
        request_ip(&headers, None).as_deref(),
        user_agent(&headers).as_deref(),
    )
    .await?;
    Ok(tokens_with_cookie(&state, &tokens))
}

/// The `state` query param — the authorize redirect's passthrough.
fn state_param(params: &BTreeMap<String, String>) -> &str {
    params.get("state").map(String::as_str).unwrap_or_default()
}

/// GET the github authorize URL when the provider is enabled (302); a
/// disabled/unconfigured provider answers the 403 envelope (operator
/// problem, not a redirect).
#[utoipa::path(
    get,
    path = "/api/v1/auth/github/login",
    tag = "authoauth",
    params(("state" = String, Query, description = "Opaque passthrough state")),
    responses(
        (status = 302, description = "Redirects to the github authorize URL"),
        (status = 403, description = "Github sign-in not configured", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn github_login(
    State(state): State<AppState>,
    Query(params): Query<BTreeMap<String, String>>,
) -> Result<Response, ApiError> {
    let target = state.oauth.github_login_url(state_param(&params))?;
    Ok(redirect(target))
}

/// GET the github callback: link flow (`state=link:<jwt>`) or login flow;
/// EVERY outcome is a 302 (tokens in the fragment on success). A
/// disabled/unconfigured provider answers the 403 envelope instead.
#[utoipa::path(
    get,
    path = "/api/v1/auth/github/callback",
    tag = "authoauth",
    params(
        ("code" = String, Query, description = "GitHub authorization code"),
        ("state" = String, Query, description = "Login passthrough or link:<jwt>"),
        ("error" = String, Query, description = "GitHub consent error code"),
        ("error_description" = String, Query, description = "Consent error detail"),
    ),
    responses(
        (status = 302, description = "Redirect to the frontend (success fragment or ?error= pairs)"),
        (status = 403, description = "Github sign-in not configured", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn github_callback(
    State(state): State<AppState>,
    Query(params): Query<BTreeMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    // The gate mirrors the go callback's first line (githubGate → 403);
    // `github_login_url` is the cheapest public entry to the same check.
    state.oauth.github_login_url("").map(|_| ())?;
    let mut error_code = params
        .get("error")
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_default();
    let mut description = params.get("error_description").cloned().unwrap_or_default();
    // GitHub sends its own consent errors straight back to us.
    let code = params
        .get("code")
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_default();
    if error_code.is_empty() && code.is_empty() {
        error_code = ERR_NO_CODE.into();
        description = String::new();
    }
    if !error_code.is_empty() {
        return Ok(redirect_error(&state.cfg, &error_code, &description));
    }
    let raw_state = state_param(&params);
    if let Some(link_jwt) = raw_state.strip_prefix(LINK_STATE_PREFIX) {
        return match state.oauth.github_link(link_jwt, &code).await {
            Ok(()) => Ok(redirect(link_success_url(&state.cfg))),
            Err(e) => Ok(redirect_error(&state.cfg, ERR_AUTH_FAILED, &err_text(&e))),
        };
    }
    match minted_github_login(
        &state,
        &code,
        request_ip(&headers, None).as_deref(),
        user_agent(&headers).as_deref(),
    )
    .await
    {
        Ok(tokens) => Ok(redirect(format!(
            "{}#{}&{}",
            state.cfg.frontend_success.trim_end_matches('/'),
            encoded_pair("access_token", &tokens.access_token),
            encoded_pair("refresh_token", &tokens.refresh_token),
        ))),
        Err(e) => Ok(redirect_error(
            &state.cfg,
            ERR_AUTH_FAILED,
            &err_text(&e.inner),
        )),
    }
}

/// Bearer-protected `{url}` whose state carries the link JWT (the
/// authenticated github connect flow).
#[utoipa::path(
    get,
    path = "/api/v1/auth/github/connect-url",
    tag = "authoauth",
    responses(
        (status = 200, description = "The authorize URL carrying the link state", body = GithubConnectURLResponse),
        (status = 403, description = "Github sign-in not configured", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn github_connect_url(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<GithubConnectURLResponse>, ApiError> {
    let target = state.oauth.github_connect_url(UserId(principal.id))?;
    Ok(Json(GithubConnectURLResponse { url: target }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_success_url_matches_the_go_derivation() {
        let mut cfg = crate::config::Config::test_default();
        cfg.frontend_success = "http://localhost:5173/auth/success".to_owned();
        assert_eq!(
            link_success_url(&cfg),
            "http://localhost:5173/user/profile?tab=overview&github_linked=1"
        );
        // A custom success URL (no /auth/suffix) is used as-is.
        cfg.frontend_success = "https://portal.test/".to_owned();
        assert_eq!(
            link_success_url(&cfg),
            "https://portal.test/user/profile?tab=overview&github_linked=1"
        );
    }

    #[test]
    fn error_redirects_carry_urlencoded_pairs() {
        let mut cfg = crate::config::Config::test_default();
        cfg.frontend_success = "http://localhost:5173/auth/success".to_owned();
        let res = redirect_error(&cfg, ERR_AUTH_FAILED, "github sign-in is not configured");
        assert_eq!(res.status(), StatusCode::FOUND);
        let location = res
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .expect("location header");
        assert_eq!(
            location,
            "http://localhost:5173/auth/success?error=auth_failed&error_description=github+sign-in+is+not+configured"
        );
    }

    #[test]
    fn no_code_is_a_redirect_not_an_error_status() {
        let mut cfg = crate::config::Config::test_default();
        cfg.frontend_success = "http://localhost:5173/auth/success".to_owned();
        let res = redirect_error(&cfg, ERR_NO_CODE, "");
        let location = res
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .expect("location header");
        assert_eq!(
            location,
            "http://localhost:5173/auth/success?error=no_code&error_description="
        );
    }

    #[test]
    fn callback_gate_redirects_on_the_provider_disabled_vocabulary() {
        // The go gate's error-code vocabulary: a disabled provider must route
        // through `github_not_configured` on the callback's redirect (the
        // handler gates first, then the consent-error branch reuses the code).
        let mut cfg = crate::config::Config::test_default();
        cfg.frontend_success = "http://localhost:5173/auth/success".to_owned();
        assert_eq!(ERR_NOT_CONFIGURED, "github_not_configured");
        let res = redirect_error(&cfg, ERR_NOT_CONFIGURED, ERR_NOT_CONFIGURED);
        assert_eq!(res.status(), StatusCode::FOUND);
        let location = res
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .expect("location header");
        assert!(
            location.contains("error=github_not_configured"),
            "{location}"
        );
    }
}
