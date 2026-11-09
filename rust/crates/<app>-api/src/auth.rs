//! Identity extraction + the rank gate — go `authx` parity for the axum side.
//!
//! * [`get_current_user`] / [`CurrentUser`] — identity in, 401 out (go
//!   `authx.IdentityFrom` / `RequireAuth`), bearer first, cookie fallback when
//!   `<APP>_AUTH_COOKIE` mode is on.
//! * [`require_admin`] — go `authx.RequireRole(rolex.Admin)` rank parity.
//!
//! Error surface: reuses the warden's [`AuthError`] envelope (same JSON shape
//! as `ApiError`) so auth and CRUD surfaces render one consistent body.
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use <app>_core::{Error, Result};
use <app>_warden::{AuthError, Principal};

use crate::state::AppState;

/// Go `authx.IdentityFrom` parity: read the [`Principal`] that a
/// `require_auth` middleware (or the [`CurrentUser`] extractor) already put in
/// the extensions. 401 when absent.
///
/// # Errors
/// [`AuthError`] (401 envelope) when no principal is attached.
pub fn get_current_user(parts: &Parts) -> std::result::Result<Principal, AuthError> {
    parts.extensions.get::<Principal>().cloned().ok_or_else(|| {
        // core Display prefixes "authentication required" — the inner
        // text names the failure mode, not the prefix (go detail parity).
        AuthError::new(Error::Unauthenticated("no principal in request".into()))
    })
}

/// Extractor: bearer-or-cookie identity (go `authx.bearerToken`/`RequireAuth`
/// parity). Reads `Authorization: Bearer …` first, then the
/// `<app>_access_token` cookie — only while `<APP>_AUTH_COOKIE` mode is on —
/// verifies with the state's codec, and ALSO inserts the resolved
/// [`Principal`] into the request extensions so handlers mixing
/// `Extension<Principal>` keep working.
#[derive(Debug)]
pub struct CurrentUser(pub Principal);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        let principal = resolve_current_user(&parts.headers, &state.cfg, &state.jwt_codec)
            .map_err(|e| match e {
                // Surface the correlation id in the 401 envelope (L6).
                e @ Error::Unauthenticated(_) => AuthError::new(e).with_request_id(
                    parts
                        .headers
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned),
                ),
                e => AuthError::new(e),
            })?;
        parts.extensions.insert(principal.clone());
        Ok(Self(principal))
    }
}

/// Bearer-or-cookie resolution (go `bearerToken` + `RequireAuth` parity):
/// `Authorization: Bearer` wins; the `auth_cookie_name()` cookie is the
/// fallback; nothing found → "authentication required"; a present-but-bad
/// token → the codec's generic 401.
fn resolve_current_user(
    headers: &axum::http::HeaderMap,
    cfg: &crate::config::Config,
    codec: &<app>_warden::JwtCodec,
) -> Result<Principal> {
    let token = bearer_token(headers)
        .or_else(|| {
            cfg.auth_cookie_name()
                .and_then(|name| cookie_value(headers, name))
        })
        .ok_or_else(|| Error::Unauthenticated("missing bearer token or cookie".into()))?;
    let claims: <app>_warden::Claims = codec.verify(&token)?;
    Ok(Principal {
        id: claims.sub,
        roles: claims.roles,
    })
}

/// The full verified claims of the request's bearer/cookie token — go
/// `authx.IdentityFrom` parity for the fields beyond [`Principal`] (login
/// handlers stamp `jti == sessions.id`, which `/revoke-current` and
/// password `change` need). Bearer first, cookie fallback; a present-but-bad
/// token → the codec's generic 401.
///
/// # Errors
/// [`Error::Unauthenticated`] (401) when the token is absent or rejected.
pub fn resolve_current_claims(
    headers: &axum::http::HeaderMap,
    cfg: &crate::config::Config,
    codec: &<app>_warden::JwtCodec,
) -> Result<<app>_warden::Claims> {
    let token = bearer_token(headers)
        .or_else(|| {
            cfg.auth_cookie_name()
                .and_then(|name| cookie_value(headers, name))
        })
        .ok_or_else(|| Error::Unauthenticated("missing bearer token or cookie".into()))?;
    codec.verify(&token)
}

/// `Authorization: Bearer <token>` — case-insensitive scheme (RFC 7235), the
/// same shape as the warden's (private) `extract_bearer`.
fn bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let v = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let (scheme, rest) = v.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = rest.trim();
    (!token.is_empty()).then(|| token.to_owned())
}

/// First matching cookie value from the `Cookie` header (RFC 6265 §4.2: the
/// first pair wins; names are case-sensitive).
fn cookie_value(headers: &axum::http::HeaderMap, cookie_name: &str) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    raw.split(';').map(str::trim).find_map(|kv| {
        let (name, value) = kv.split_once('=')?;
        (name == cookie_name).then(|| value.trim_matches('"').to_owned())
    })
}

/// Go `authx.RequireRole(rolex.Admin)` parity — the rank gate on the identity
/// roles. Explicit rank check (core `can(role, Action::Admin)` is owner-only
/// by matrix design), so the go ADMIN role maps to core `Role::Admin` and
/// both pass; anything else is 403 "insufficient role" (go message parity).
///
/// # Errors
/// [`Error::Forbidden`] when no role meets the admin rank.
pub fn require_admin(principal: &Principal) -> Result<()> {
    if principal.roles.iter().any(|r| {
        matches!(
            r,
            <app>_core::rbac::Role::Admin | <app>_core::rbac::Role::Owner
        )
    }) {
        return Ok(());
    }
    Err(Error::Forbidden("insufficient role".into()))
}

/// axum middleware, the go `RequireAuth(verifier, cookieName)` parity over
/// `AppState`: bearer first, cookie fallback while `cfg.auth_cookie_name()` is
/// `Some`, then verify + attach the [`Principal`] extension. Layer with
/// `axum::middleware::from_fn_with_state(state.clone(), require_auth_cookie)`.
pub async fn require_auth_cookie(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> std::result::Result<axum::response::Response, AuthError> {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let principal =
        resolve_current_user(req.headers(), &state.cfg, &state.jwt_codec).map_err(|e| {
            AuthError::new(
                if matches!(e, Error::Unauthorized | Error::Unauthenticated(_)) {
                    // The 401 envelope: missing/bad token.
                    e
                } else {
                    // Belt-and-suspenders: never leak a non-auth reason.
                    Error::Unauthenticated("invalid or expired token".into())
                },
            )
            .with_request_id(request_id)
        })?;
    req.extensions_mut().insert(principal);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, body::Body, http::StatusCode, routing::get};
    use <app>_core::rbac::Role;
    use <app>_mailer::NoopMailer;
    use <app>_ports::mock::{
        MockAuditLogRepository, MockEmailVerificationRepository, MockOutboxRepository,
        MockPasswordResetRepository, MockSessionRepository, MockUserRepository,
    };
    use <app>_warden::JwtCodec;
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn codec() -> JwtCodec {
        JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!")
    }

    /// Unix seconds now (avoids pulling chrono into dev-deps).
    #[allow(clippy::cast_possible_wrap)]
    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs() as i64
    }

    /// Dead pool + mock repos + the test codec + cookie mode off.
    fn test_state() -> AppState {
        let mut cfg = crate::config::Config::test_default();
        cfg.auth_cookie = false;
        app_state(cfg)
    }

    fn app_state(cfg: crate::config::Config) -> AppState {
        AppState::with_repos(
            sqlx::postgres::PgPoolOptions::new()
                .acquire_timeout(std::time::Duration::from_secs(1))
                .connect_lazy("postgres://u:p@127.0.0.1:1/db")
                .expect("lazy pool"),
            Arc::new(MockUserRepository::default()),
            Arc::new(MockSessionRepository::default()),
            Arc::new(MockEmailVerificationRepository::default()),
            Arc::new(MockPasswordResetRepository::default()),
            Arc::new(MockOutboxRepository::default()),
            Arc::new(MockAuditLogRepository::default()),
            Arc::new(<app>_ports::mock::MockWidgetRepository::default()),
            Arc::new(<app>_loom::Hub::new(16)),
            Arc::new(NoopMailer),
            codec(),
            cfg,
        )
    }

    /// Mint a token for `roles` and format as a `Bearer` header value.
    fn bearer_for(codec: &JwtCodec, roles: Vec<Role>) -> String {
        let t = now();
        let token = codec.mint(Uuid::new_v4(), roles, t, t + 900).expect("mint");
        format!("Bearer {token}")
    }

    /// Mint a raw token (no scheme) for the cookie tests.
    fn raw_token(codec: &JwtCodec) -> String {
        let t = now();
        codec
            .mint(Uuid::new_v4(), vec![Role::Viewer], t, t + 900)
            .expect("mint")
    }

    /// One-shot router mounting the extractor; echoes `user:<id>`.
    fn extractor_app(state: AppState) -> Router {
        Router::new()
            .route(
                "/whoami",
                // axum 0.7+ handrolls no sync `Handler`: the closure must yield a
                // Future.
                get(|user: CurrentUser| async move { Json(format!("user:{}", user.0.id)) }),
            )
            .with_state(state)
    }

    /// Middleware-protected router echoing the admin check result.
    fn middleware_app(state: AppState) -> Router {
        Router::new()
            .route("/secure", get(|| async { Json("ok".to_owned()) }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                require_auth_cookie,
            ))
            .with_state(state)
    }

    #[tokio::test]
    async fn extractor_accepts_valid_bearer() {
        let state = test_state();
        let auth = bearer_for(&codec(), vec![Role::Admin]);
        let res = extractor_app(state)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn extractor_rejects_when_no_token_at_all() {
        let res = extractor_app(test_state())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        // Envelope parity with the warden's AuthError (auth_* body shape).
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn extractor_rejects_invalid_token() {
        let res = extractor_app(test_state())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", "Bearer garbage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cookie_fallback_requires_cookie_mode() {
        // <APP>_AUTH_COOKIE=false: a cookie alone must NOT authenticate.
        let state = test_state(); // auth_cookie = false
        let token = raw_token(&codec());
        let res = extractor_app(state)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("cookie", format!("<app>_access_token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cookie_fallback_accepts_token_in_cookie_mode() {
        let mut cfg = crate::config::Config::test_default();
        cfg.auth_cookie = true;
        let token = raw_token(&codec());
        let res = extractor_app(app_state(cfg))
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("cookie", format!("<app>_access_token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "cookie mode: cookie accepted");
    }

    #[tokio::test]
    async fn bearer_wins_over_cookie() {
        // Go bearerToken order: header first — a garbage cookie must not
        // shadow a valid bearer, and vice versa a garbage bearer must not
        // fall through to a valid cookie.
        let mut cfg = crate::config::Config::test_default();
        cfg.auth_cookie = true;
        let auth = bearer_for(&codec(), vec![Role::Viewer]);
        let res = extractor_app(app_state(cfg))
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", &auth)
                    .header("cookie", "<app>_access_token=garbage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "valid bearer wins");
    }

    #[tokio::test]
    async fn middleware_accepts_bearer_and_rejects_absent() {
        let state = test_state();
        let auth = bearer_for(&codec(), vec![Role::Owner]);
        let app = middleware_app(state);
        let res = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/secure")
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/secure")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_admin_is_the_rank_gate() {
        // Go RequireRole(rolex.Admin): core Admin and Owner pass; Editor and
        // Viewer are 403 "insufficient role".
        let mk = |roles: Vec<Role>| Principal {
            id: Uuid::new_v4(),
            roles,
        };
        assert!(require_admin(&mk(vec![Role::Admin])).is_ok());
        assert!(require_admin(&mk(vec![Role::Owner])).is_ok());
        let err = require_admin(&mk(vec![Role::Editor])).unwrap_err();
        assert!(matches!(err, Error::Forbidden(ref m) if m == "insufficient role"));
        assert_eq!(err.status_code(), 403);
        assert!(require_admin(&mk(vec![Role::Viewer])).is_err());
        // Multi-role identity with any admin-ranked role passes.
        assert!(require_admin(&mk(vec![Role::Viewer, Role::Admin])).is_ok());
    }

    #[test]
    fn cookie_value_parses_first_match_and_quotes() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            "other=1; <app>_access_token=\"tok.val\"; x=2"
                .parse()
                .expect("header value"),
        );
        assert_eq!(
            cookie_value(&h, "<app>_access_token").as_deref(),
            Some("tok.val")
        );
        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            "a=1".parse().expect("header value"),
        );
        assert_eq!(cookie_value(&h, "<app>_access_token"), None);
    }

    #[test]
    fn get_current_user_reads_the_principal_extension() {
        // go IdentityFrom parity: the principal parked by require_auth.
        let principal = Principal {
            id: Uuid::new_v4(),
            roles: vec![Role::Viewer],
        };
        let (mut parts, ()) = axum::http::Request::<()>::new(()).into_parts();
        assert!(get_current_user(&parts).is_err(), "absent = 401");
        parts.extensions.insert(principal.clone());
        assert_eq!(get_current_user(&parts).expect("present").id, principal.id);
    }

    #[tokio::test]
    async fn extractor_rejects_expired_token() {
        let res = extractor_app(test_state())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header(
                        "authorization",
                        format!(
                            "Bearer {}",
                            // exp-iat must stay <= MAX_ACCESS_TTL_SECS in mint,
                            // so shift the whole window into the past.
                            codec()
                                .mint(
                                    Uuid::new_v4(),
                                    vec![Role::Viewer],
                                    now() - 1_000,
                                    now() - 500
                                )
                                .expect("mint expired")
                        ),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "expired = 401");
    }
}
