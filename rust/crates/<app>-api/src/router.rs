//! axum router. The composition root mounts routes over `AppState` and wraps
//! them in the request-lifecycle middleware stack:
//!
//! `request_id_layer` (outer) → `TraceLayer` → routes
//!
//! The request-id layer runs first so the [`RequestId`] extension is available
//! to `TraceLayer`'s `make_span_with` before the span is created.
//!
//! Swagger UI is served at `/docs`; the raw spec at `/docs/openapi.json`.
//! `just gen-openapi` emits the committed `openapi.json`; `just gen-check`
//! fails on drift.

use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::require_auth_cookie;
use crate::error::ErrorEnvelope;
use crate::request_id::{RequestId, request_id_layer};
use crate::routes::admin_users::{get_user, list_users, update_user, upgrade_to_admin};
use crate::routes::oauth::{github_callback, github_connect_url, github_login, google};
use crate::routes::password::{change_password, forgot_password, reset_password, validate_code};
use crate::routes::realtime::ws;
use crate::routes::session::{
    admin_anomalies, admin_list_sessions, admin_revoke_all_sessions, auth_me, auth_update_me,
    current_session, list_sessions, login, logout, refresh, revoke_all_sessions,
    revoke_current_session,
};
use crate::routes::signup::{resend, signup, verify};
use crate::routes::widgets::{
    create_widget, delete_widget, event_stream, get_widget, list_widgets, update_widget,
};
use crate::state::AppState;
use <app>_warden::{Principal, require_auth};

/// Typed health payload.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Authenticated caller identity returned by `GET /me`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub roles: Vec<<app>_core::Role>,
}

const SERVICE_NAME: &str = "<app>-api";

/// `OpenAPI` document. `paths(...)` and `components(schemas(...))` are the single
/// source of truth for the generated spec — adding a route without registering
/// it here means it won't appear in `/docs` or the emitted `openapi.json`.
#[derive(Debug, OpenApi)]
#[openapi(
    info(
        title = "<app> API",
        version = "0.1.0",
        description = "<app> · example vertical slice (widget CRUD + auth seam).",
    ),
    paths(
        healthz,
        readyz,
        me,
        crate::routes::widgets::list_widgets,
        crate::routes::widgets::get_widget,
        crate::routes::widgets::create_widget,
        crate::routes::widgets::update_widget,
        crate::routes::widgets::delete_widget,
        crate::routes::widgets::event_stream,
        // identity: authsignup
        crate::routes::signup::signup,
        crate::routes::signup::verify,
        crate::routes::signup::resend,
        // identity: authsession
        crate::routes::session::login,
        crate::routes::session::refresh,
        crate::routes::session::logout,
        crate::routes::session::auth_me,
        crate::routes::session::auth_update_me,
        crate::routes::session::list_sessions,
        crate::routes::session::current_session,
        crate::routes::session::revoke_current_session,
        crate::routes::session::revoke_all_sessions,
        crate::routes::session::admin_list_sessions,
        crate::routes::session::admin_revoke_all_sessions,
        crate::routes::session::admin_anomalies,
        // identity: authpassword
        crate::routes::password::forgot_password,
        crate::routes::password::validate_code,
        crate::routes::password::reset_password,
        crate::routes::password::change_password,
        // identity: authoauth
        crate::routes::oauth::google,
        crate::routes::oauth::github_login,
        crate::routes::oauth::github_callback,
        crate::routes::oauth::github_connect_url,
        // identity: adminusers
        crate::routes::admin_users::list_users,
        crate::routes::admin_users::get_user,
        crate::routes::admin_users::update_user,
        crate::routes::admin_users::upgrade_to_admin,
        // realtime
        crate::routes::realtime::ws,
    ),
    components(schemas(
        HealthResponse,
        MeResponse,
        ErrorEnvelope,
        crate::error::ErrorBody,
        crate::routes::widgets::CreateWidgetRequest,
        crate::routes::widgets::UpdateWidgetRequest,
        <app>_core::Widget,
        <app>_core::WidgetId,
        <app>_core::WidgetKind,
        <app>_core::Role,
        // identity: signup
        crate::routes::signup::SignupRequest,
        crate::routes::signup::VerifyEmailRequest,
        // identity: session
        crate::routes::session::AuthLoginRequest,
        crate::routes::session::AuthRefreshRequest,
        crate::routes::session::UpdateMeRequest,
        crate::routes::session::MeProfileResponse,
        // identity: password
        crate::routes::password::ForgotPasswordRequest,
        crate::routes::password::ValidateCodeRequest,
        crate::routes::password::ResetPasswordRequest,
        crate::routes::password::ChangePasswordRequest,
        // identity: oauth
        crate::routes::oauth::GoogleAuthRequest,
        crate::routes::oauth::GithubConnectURLResponse,
        // identity: adminusers
        crate::routes::admin_users::AdminUser,
        crate::routes::admin_users::AdminUserList,
        crate::routes::admin_users::AdminUpdateUser,
        crate::routes::admin_users::AuthUpgradeResponse,
        // shared wire types
        crate::routes::MessageResponse,
        crate::routes::EmailRequest,
        crate::routes::AuthTokens,
        crate::routes::SessionItem,
        crate::routes::SessionRevoked,
        crate::routes::SessionRevokedCount,
        crate::routes::AnomalyItem,
    ))
)]
pub struct ApiDoc;

/// Build the `OpenAPI` document. Used by the `openapi` subcommand in `main.rs`
/// and by the `/docs` swagger-ui route.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn build(state: AppState) -> Router {
    // Clone the codec before `state` is moved into `with_state`.
    let jwt_codec = state.jwt_codec.clone();

    // Auth-protected sub-router. `require_auth` gets `JwtCodec` via
    // `from_fn_with_state`; the `me` handler reads `Principal` from extensions.
    // This sub-router is `Router<()>` — merged after the main router's
    // `.with_state(state)` so both are `Router<()>`.
    let me_routes =
        Router::new()
            .route("/me", get(me))
            .layer(axum::middleware::from_fn_with_state(
                jwt_codec.clone(),
                require_auth,
            ));

    // C1: widget CRUD is mounted BEHIND `require_auth` — mutating routes must
    // NOT live on the unauthenticated router. Each handler additionally
    // checks `can(role, action)` for its Action (Read/Create/Update/Delete).
    // The sub-router carries `AppState` (the handlers need it); `with_state`
    // turns it into `Router<()>` before merge. `state` is cloned because the
    // main router also needs it for `readyz`.
    let widget_routes = Router::new()
        .route("/widgets", get(list_widgets).post(create_widget))
        .route(
            "/widgets/{id}",
            get(get_widget).put(update_widget).delete(delete_widget),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            jwt_codec,
            require_auth,
        ));

    // ── Identity (go-parity surface, module.go 1:1) ──────────────────────
    // Unauthenticated identity routes (the full list + `/ws` contract live in
    // [`identity_public_routes`]).
    let identity_public_routes = identity_public_routes(&state);

    // Authenticated identity routes (go RequireAuth parity): the bearer/cookie
    // middleware sets `Principal`; handlers read it from the extensions.
    // Logout doubles as a cookie-clear (the handler clears via the config).
    let identity_protected_routes = Router::new()
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(auth_me).put(auth_update_me))
        .route("/api/v1/auth/sessions", get(list_sessions))
        .route("/api/v1/auth/sessions/current", get(current_session))
        .route(
            "/api/v1/auth/sessions/revoke-current",
            post(revoke_current_session),
        )
        .route(
            "/api/v1/auth/sessions/revoke-all",
            post(revoke_all_sessions),
        )
        .route("/api/v1/auth/change-password", post(change_password))
        .route("/api/v1/auth/github/connect-url", get(github_connect_url))
        .route("/api/v1/widgets/events", get(event_stream))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth_cookie,
        ));

    // Admin routes (go RequireAuth + RequireRole(Admin) parity): the same
    // bearer/cookie middleware, then the handler-side rank gate
    // (`require_admin`) — 403 "insufficient role" for below-admin callers.
    let admin_routes = Router::new()
        .route(
            "/api/v1/admin/auth-session/users/{id}/sessions",
            get(admin_list_sessions),
        )
        .route(
            "/api/v1/admin/auth-session/users/{id}/revoke-all",
            post(admin_revoke_all_sessions),
        )
        .route("/api/v1/admin/auth-session/anomalies", get(admin_anomalies))
        .route("/api/v1/admin/users", get(list_users))
        .route("/api/v1/admin/users/{id}", get(get_user).put(update_user))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth_cookie,
        ));

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
        .merge(widget_routes)
        .merge(me_routes)
        .merge(identity_public_routes)
        .merge(identity_protected_routes)
        .merge(admin_routes);

    // Metrics plane (feature-gated). The `/metrics` route + duration middleware
    // are mounted only when the `metrics` feature is on. The recorder itself is
    // runtime-gated by `cfg.metrics_enabled` (see `metrics::init`).
    //
    // M5: `/metrics` is NOT behind `require_auth` here. Before production
    // exposure, either (a) restrict it to an internal listener (bind a
    // separate port reachable only inside the cluster), or (b) mount it behind
    // `require_auth` + an Admin action check. Exposing raw Prometheus metrics
    // to the public internet leaks internal cardinality + traffic shapes.
    #[cfg(feature = "metrics")]
    let router = router
        .route("/metrics", get(crate::metrics::metrics_handler))
        .layer(axum::middleware::from_fn(
            crate::metrics::http_metrics_layer,
        ));

    router
        // L7: Swagger UI + spec endpoint. Mounted after `.with_state` so the
        // stateless SwaggerUi router merges into `Router<()>`. DISABLE in
        // production (feature-gate `utoipa-swagger-ui` out) OR mount behind
        // `require_auth` + Admin before public exposure — the spec reveals the
        // full surface area to an attacker.
        .merge(SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi()))
        .layer(
            // TraceLayer sits inside the request-id layer so the span can carry
            // the correlation id. `make_span_with` pulls the `RequestId`
            // extension set by `request_id_layer`.
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<axum::body::Body>| {
                let request_id = req
                    .extensions()
                    .get::<RequestId>()
                    .map_or("-", RequestId::as_str);
                tracing::info_span!(
                    "http.request",
                    request_id = %request_id,
                    method = %req.method(),
                    uri = %req.uri(),
                )
            }),
        )
        .layer(axum::middleware::from_fn(request_id_layer))
}

/// Unauthenticated identity routes: signup trio (authsignup), the session
/// mint pair (login/refresh), the password flows (authpassword), the
/// google/github login+callback pair (authoauth), the whitelisted
/// upgrade-to-admin bootstrap, and the `/ws` upgrade (the loom
/// authenticates AFTER the upgrade — closes 4001 on failure).
fn identity_public_routes(state: &AppState) -> Router<()> {
    Router::new()
        .route("/api/v1/auth/signup", post(signup))
        .route("/api/v1/auth/verify", post(verify))
        .route("/api/v1/auth/verify/resend", post(resend))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route("/api/v1/auth/validate-code", post(validate_code))
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route("/api/v1/auth/google", post(google))
        .route("/api/v1/auth/github/login", get(github_login))
        .route("/api/v1/auth/github/callback", get(github_callback))
        .route("/api/v1/auth/upgrade-to-admin", post(upgrade_to_admin))
        .route("/api/v1/ws", get(ws))
        .with_state(state.clone())
}

/// Liveness — always succeeds, never touches the pool. Cheap for orchestrators
/// (k8s liveness probe) hammering it every few seconds.
#[utoipa::path(
    get,
    path = "/healthz",
    tag = "system",
    responses((status = 200, body = HealthResponse, description = "Process is live"))
)]
pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        service: SERVICE_NAME.into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Readiness — actually probes the database (`SELECT 1`). Returns `200` when
/// the pool can serve, `503` with `status: "degraded"` when it cannot. This is
/// the probe that gates traffic: a failing DB means we are not ready.
#[utoipa::path(
    get,
    path = "/readyz",
    tag = "system",
    responses(
        (status = 200, body = HealthResponse, description = "Database reachable — ready for traffic"),
        (status = 503, description = "Database unreachable — not ready"),
    )
)]
pub async fn readyz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Response {
    let probe = sqlx::query("SELECT 1").execute(&state.pool).await;
    let (status_code, health_status) = if probe.is_ok() {
        (axum::http::StatusCode::OK, "ok")
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "degraded")
    };

    let mut resp = Json(HealthResponse {
        status: health_status.into(),
        service: SERVICE_NAME.into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response();
    *resp.status_mut() = status_code;
    resp
}

/// Authenticated identity — returns the caller's id and roles. Protected by
/// `require_auth`; the `Principal` is set by the middleware from the verified
/// JWT. ponytail: a full security scheme (`bearer_auth`) should be registered on
/// this path when the login flow is built.
#[utoipa::path(
    get,
    path = "/me",
    tag = "auth",
    responses(
        (status = 200, description = "Caller identity", body = MeResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorEnvelope),
    )
)]
pub async fn me(axum::Extension(principal): axum::Extension<Principal>) -> Json<MeResponse> {
    Json(MeResponse {
        id: principal.id,
        roles: principal.roles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::StatusCode;
    use <app>_core::Role;
    use <app>_loom::Hub;
    use <app>_mailer::NoopMailer;
    use <app>_ports::mock::{
        MockAuditLogRepository, MockEmailVerificationRepository, MockOutboxRepository,
        MockPasswordResetRepository, MockSessionRepository, MockUserRepository,
        MockWidgetRepository,
    };
    use <app>_warden::JwtCodec;
    use http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    /// Build a state with a lazy pool pointing at a (likely absent) DB — enough
    /// for structural tests. The pool is never actually queried except by
    /// `readyz`, which degrades gracefully to 503.
    fn test_state() -> AppState {
        identity_state(|_| {}).0
    }

    /// Full-mock identity state: the caller mutates the knobs it exercises
    /// (`github_enabled`, `frontend_success`, …) and holds the seeded repos.
    fn identity_state(
        knobs: impl FnOnce(&mut crate::config::Config),
    ) -> (
        AppState,
        Arc<MockUserRepository>,
        Arc<MockSessionRepository>,
    ) {
        let mut cfg = crate::config::Config::test_default();
        knobs(&mut cfg);
        let users = Arc::new(MockUserRepository::default());
        let sessions = Arc::new(MockSessionRepository::default());
        let state = AppState::with_repos(
            dead_pool(),
            users.clone(),
            sessions.clone(),
            Arc::new(MockEmailVerificationRepository::default()),
            Arc::new(MockPasswordResetRepository::default()),
            Arc::new(MockOutboxRepository::default()),
            Arc::new(MockAuditLogRepository::default()),
            Arc::new(MockWidgetRepository::default()),
            Arc::new(Hub::new(16)),
            Arc::new(NoopMailer),
            JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!"),
            cfg,
        );
        (state, users, sessions)
    }

    fn dead_pool() -> sqlx::PgPool {
        // Point at a deliberately unreachable DB: these are unit tests using
        // the mock repos, so the pool is only exercised by `readyz`
        // (which must degrade to 503). Using DATABASE_URL would connect to the
        // gate's live Postgres and break `readyz_returns_degraded_without_db`.
        // The 1s acquire_timeout keeps the failure fast (default is 30s).
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_lazy("postgres://u:p@127.0.0.1:1/db")
            .expect("lazy pool")
    }

    /// Unix seconds for "now" — avoids pulling chrono into dev-deps.
    #[allow(clippy::cast_possible_wrap)]
    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs() as i64
    }

    /// Mint a test token for `roles` and format it as a `Bearer` header value.
    fn bearer(codec: &JwtCodec, roles: Vec<Role>) -> String {
        let t = now();
        let token = codec
            .mint(uuid::Uuid::new_v4(), roles, t, t + 900)
            .expect("mint");
        format!("Bearer {token}")
    }

    #[tokio::test]
    async fn sse_events_rejects_unauthenticated() {
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/widgets/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    /// The SSE surface: with a valid bearer, the route answers 200 and the
    /// SSE headers — never draining the body (an open-ended stream would hang
    /// `to_bytes`).
    #[tokio::test]
    async fn sse_events_streams_event_stream_headers() {
        let (state, _users, _sessions) = identity_state(|_| {});
        let codec = state.jwt_codec.clone();
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/widgets/events")
                    .header("authorization", bearer(&codec, vec![Role::Viewer]))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()["content-type"],
            "text/event-stream",
            "SSE content-type (browser EventSource contract)"
        );
        assert_eq!(res.headers()["cache-control"], "no-cache");
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], SERVICE_NAME);
    }

    #[tokio::test]
    async fn healthz_echoes_request_id_header() {
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .header("x-request-id", "test-trace-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
            Some("test-trace-1")
        );
    }

    #[tokio::test]
    async fn healthz_generates_request_id_when_absent() {
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = res
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .expect("server must generate an id");
        assert!(
            id.starts_with("req_"),
            "generated id must be prefixed: {id}"
        );
    }

    #[tokio::test]
    async fn readyz_returns_degraded_without_db() {
        // No real DB in the test env → SELECT 1 fails → 503 + degraded.
        // This verifies the probe path is actually exercised (not a stub).
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "readyz must 503 when the pool is unreachable"
        );
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "degraded");
        assert_eq!(json["service"], SERVICE_NAME);
        assert!(json["version"].is_string(), "version must be present");
    }

    #[tokio::test]
    async fn widget_crud_roundtrips_with_admin_token() {
        // C1: the full CRUD roundtrip now requires an Admin token. Uses
        // MockWidgetRepository (shared via Arc inside AppState) so all requests
        // against the cloned router see the same in-memory store.
        let state = test_state();
        let codec = state.jwt_codec.clone();
        let auth = bearer(&codec, vec![Role::Admin]);
        let app = build(state);

        // POST /widgets — create (Admin can Create)
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/widgets")
                    .header("authorization", &auth)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"cog-1","kind":"cog"}"#.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let widget: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = widget["id"].as_str().expect("id is a uuid string");

        // GET /widgets/{id} — read back (Admin can Read)
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/widgets/{id}"))
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(got["name"], "cog-1");

        // GET /widgets — list
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/widgets")
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            list.as_array().expect("list is an array").len(),
            1,
            "one widget was created"
        );

        // DELETE /widgets/{id} (Admin can Delete)
        let res = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/widgets/{id}"))
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn openapi_spec_is_non_empty() {
        let doc = openapi();
        let json = serde_json::to_value(&doc).unwrap();
        // At minimum, the spec must list the health + auth + widget paths.
        let paths = json["paths"].as_object().expect("paths is an object");
        assert!(paths.contains_key("/healthz"), "healthz must be in spec");
        assert!(paths.contains_key("/me"), "me must be in spec");
        assert!(paths.contains_key("/widgets"), "widgets must be in spec");
        assert!(
            paths.contains_key("/widgets/{id}"),
            "widget detail must be in spec"
        );
        // Role schema must be registered (used by MeResponse).
        let schemas = json["components"]["schemas"]
            .as_object()
            .expect("schemas is an object");
        assert!(schemas.contains_key("Role"), "Role must be in schemas");
    }

    #[tokio::test]
    async fn me_returns_principal_with_valid_token() {
        let state = test_state();
        let codec = state.jwt_codec.clone();
        let sub = uuid::Uuid::new_v4();
        let t = now();
        let token = codec
            .mint(sub, vec![Role::Owner], t, t + 900)
            .expect("mint");

        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], sub.to_string());
        assert_eq!(json["roles"][0], "owner");
    }

    #[tokio::test]
    async fn me_rejects_missing_token() {
        let app = build(test_state());
        let res = app
            .oneshot(Request::builder().uri("/me").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn me_rejects_invalid_token() {
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header("authorization", "Bearer garbage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // ── C1: auth + RBAC on widget CRUD ──────────────────────────────────

    #[tokio::test]
    async fn post_widgets_without_auth_returns_401() {
        // C1(a): POST /widgets with NO Authorization header → 401 (the
        // require_auth middleware rejects before the handler runs).
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/widgets")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"cog-1","kind":"cog"}"#.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn delete_widgets_with_viewer_token_returns_403() {
        // C1(b): DELETE /widgets/{id} with a Viewer-role token → 403 (Viewer
        // cannot Delete). The middleware lets it through (valid token), the
        // handler's require_action denies it.
        let state = test_state();
        let codec = state.jwt_codec.clone();
        // Create a widget first (Admin) so the 404 vs 403 distinction is clear.
        let auth_admin = bearer(&codec, vec![Role::Admin]);
        let app = build(state);
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/widgets")
                    .header("authorization", &auth_admin)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"cog-1","kind":"cog"}"#.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let widget: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = widget["id"].as_str().expect("id is a uuid string");

        // Now attempt DELETE with a Viewer token → 403.
        let auth_viewer = bearer(&codec, vec![Role::Viewer]);
        let res = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/widgets/{id}"))
                    .header("authorization", &auth_viewer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::FORBIDDEN,
            "Viewer must not Delete (403, not 401 or 404)"
        );
    }

    #[tokio::test]
    async fn post_widgets_with_editor_token_returns_201() {
        // C1(c): POST /widgets with an Editor token → 201 (Editor can Create).
        let state = test_state();
        let codec = state.jwt_codec.clone();
        let auth = bearer(&codec, vec![Role::Editor]);
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/widgets")
                    .header("authorization", &auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"cog-editor","kind":"cog"}"#.to_owned(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn post_widgets_with_admin_token_returns_201() {
        // C1(c): POST /widgets with an Admin token → 201 (Admin can Create).
        let state = test_state();
        let codec = state.jwt_codec.clone();
        let auth = bearer(&codec, vec![Role::Admin]);
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/widgets")
                    .header("authorization", &auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"cog-admin","kind":"cog"}"#.to_owned(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn get_widgets_with_viewer_token_returns_200() {
        // C1(d): GET /widgets with a Viewer token → 200 (Viewer can Read).
        let state = test_state();
        let codec = state.jwt_codec.clone();
        let auth = bearer(&codec, vec![Role::Viewer]);
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/widgets")
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "Viewer must Read (200)");
    }

    #[tokio::test]
    async fn get_widgets_without_auth_returns_401() {
        // C1: GET /widgets with NO Authorization header → 401.
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/widgets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Identity wiring (go surface parity) ─────────────────────────────

    /// A verified `core::User` for the identity seeds (argon2id hash, standard
    /// USER flavor). email must be lowercase (the lookups are exact-match).
    fn test_user(
        id: uuid::Uuid,
        email: &str,
        password: &str,
        role: <app>_core::Role,
    ) -> <app>_core::User {
        let now = chrono::Utc::now();
        <app>_core::User {
            id: <app>_core::UserId(id),
            email: email.to_owned(),
            first_name: "Ada".to_owned(),
            last_name: "Lovelace".to_owned(),
            role,
            user_type: <app>_core::UserType::User,
            is_active: true,
            is_blocked: false,
            is_verified: true,
            retry_count: 0,
            password: <app>_warden::hash_password(password, "").expect("hash"),
            language: "en".to_owned(),
            profile_picture: None,
            google_id: None,
            github_id: None,
            last_login: None,
            created_at: now,
            updated_at: now,
        }
    }

    async fn post_json(app: &Router, uri: &str, body: &str) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .expect("request"),
            )
            .await
            .expect("infallible response")
    }

    /// (status, body JSON) for a POST with a JSON body.
    async fn post_json_checked(
        app: &Router,
        uri: &str,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let res = post_json(app, uri, body).await;
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 16_384).await.unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// Seed + build the router for the login flow (both repo sides need the
    /// user's row — the JOIN materializes inside the session mock).
    fn login_app() -> Router {
        let (state, users, sessions) = identity_state(|_| {});
        let user = test_user(
            uuid::Uuid::new_v4(),
            "ada@example.com",
            "password-secret-1",
            <app>_core::Role::Viewer,
        );
        users.seed(user.clone());
        sessions.seed_user(user);
        build(state)
    }

    #[tokio::test]
    async fn signup_returns_201_with_the_go_message() {
        let app = build(test_state());
        let (status, json) = post_json_checked(
            &app,
            "/api/v1/auth/signup",
            r#"{"email":"grace@example.com","password":"password123!",
                 "first_name":"Grace","user_type":"user"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["message"], "signup successful");
    }

    #[tokio::test]
    async fn login_returns_the_bearer_token_pair() {
        let app = login_app();
        let (status, json) = post_json_checked(
            &app,
            "/api/v1/auth/login",
            r#"{"email":"ada@example.com","password":"password-secret-1"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(json["access_token"].as_str().is_some_and(|t| !t.is_empty()));
        assert!(
            json["refresh_token"]
                .as_str()
                .is_some_and(|t| !t.is_empty())
        );
        assert_eq!(json["token_type"], "bearer");
        assert!(json["expires_in"].is_i64());
    }

    #[tokio::test]
    async fn refresh_rotates_the_refresh_token() {
        let app = login_app();
        let (_, first) = post_json_checked(
            &app,
            "/api/v1/auth/login",
            r#"{"email":"ada@example.com","password":"password-secret-1"}"#,
        )
        .await;
        let refresh = first["refresh_token"].as_str().expect("refresh token");
        let (status, json) = post_json_checked(
            &app,
            "/api/v1/auth/refresh",
            &format!(r#"{{"refresh_token":"{refresh}"}}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_ne!(
            json["refresh_token"].as_str().unwrap(),
            refresh,
            "rotation is in place (new token)"
        );
    }

    #[tokio::test]
    async fn forgot_password_is_always_a_200() {
        let app = build(test_state());
        let (status, json) = post_json_checked(
            &app,
            "/api/v1/auth/forgot-password",
            r#"{"email":"nobody@example.com"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "no existence leak");
        assert!(json["message"].as_str().unwrap().contains("reset code"));
    }

    #[tokio::test]
    async fn admin_users_reject_a_non_admin_bearer() {
        let state = test_state();
        let auth = bearer(&state.jwt_codec, vec![Role::Viewer]);
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/users")
                    .header("authorization", &auth)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let bytes = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], "forbidden");
    }

    #[tokio::test]
    async fn github_login_redirects_to_the_authorize_url_when_configured() {
        let (state, _users, _sessions) = identity_state(|cfg| {
            cfg.github_enabled = true;
            cfg.github_client_id = Some("client-id".to_owned());
            cfg.github_client_secret = Some("client-secret".repeat(2));
        });
        let app = build(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/github/login?state=xyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FOUND, "go http.Redirect parity");
        let location = res
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .expect("redirect target");
        assert!(location.contains("github.com"), "{location}");
    }

    #[tokio::test]
    async fn ws_is_reachable_without_a_token() {
        // The /ws upgrade is UNAUTHENTICATED at the HTTP layer: without the
        // upgrade headers the extractor rejects with 400 — the route exists
        // and no auth middleware intercepted (401 would be the middleware's).
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/ws")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        // The close-frame vocabulary the pump uses after a failed ?token=.
        assert_eq!(<app>_loom::hub::AUTH_FAILED_CLOSE_CODE, 4001);
        assert_eq!(
            <app>_loom::hub::AUTH_FAILED_CLOSE_REASON,
            "authentication_failed"
        );
    }

    #[tokio::test]
    async fn identity_me_requires_a_token() {
        // Authenticated identity routes are behind require_auth_cookie.
        let app = build(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Route parity: go spec ⊆ router table (static, no file reads) ────

    /// The go template's identity/realtime routes (its `specs/openapi.yaml`
    /// shape), 1:1 — each path's declared methods expanded to pairs.
    const GO_METHOD_PATHS: &[(&str, &str)] = &[
        ("GET", "/healthz"),
        ("GET", "/readyz"),
        ("POST", "/api/v1/auth/signup"),
        ("POST", "/api/v1/auth/verify"),
        ("POST", "/api/v1/auth/verify/resend"),
        ("POST", "/api/v1/auth/login"),
        ("POST", "/api/v1/auth/refresh"),
        ("POST", "/api/v1/auth/logout"),
        ("GET", "/api/v1/auth/me"),
        ("PUT", "/api/v1/auth/me"),
        ("GET", "/api/v1/auth/sessions"),
        ("GET", "/api/v1/auth/sessions/current"),
        ("POST", "/api/v1/auth/sessions/revoke-current"),
        ("POST", "/api/v1/auth/sessions/revoke-all"),
        ("GET", "/api/v1/admin/auth-session/users/{id}/sessions"),
        ("POST", "/api/v1/admin/auth-session/users/{id}/revoke-all"),
        ("GET", "/api/v1/admin/auth-session/anomalies"),
        ("POST", "/api/v1/auth/change-password"),
        ("POST", "/api/v1/auth/forgot-password"),
        ("POST", "/api/v1/auth/validate-code"),
        ("POST", "/api/v1/auth/reset-password"),
        ("POST", "/api/v1/auth/google"),
        ("GET", "/api/v1/auth/github/login"),
        ("GET", "/api/v1/auth/github/callback"),
        ("GET", "/api/v1/auth/github/connect-url"),
        ("POST", "/api/v1/auth/upgrade-to-admin"),
        ("GET", "/api/v1/admin/users"),
        ("GET", "/api/v1/admin/users/{id}"),
        ("PUT", "/api/v1/admin/users/{id}"),
        ("GET", "/api/v1/ws"),
        ("GET", "/api/v1/widgets"),
        ("GET", "/api/v1/widgets/events"),
        ("POST", "/api/v1/widgets"),
        ("GET", "/api/v1/widgets/{id}"),
        ("PUT", "/api/v1/widgets/{id}"),
        ("DELETE", "/api/v1/widgets/{id}"),
    ];

    /// Every route the router mounts: (module, method, path). Health + the
    /// legacy `me` seam live in `router`; the widget CRUD carried its
    /// unprefixed path (the go `/api/v1` prefix maps via the strip in
    /// [`go_route_table_parity`]); the identity modules contribute the rest.
    const ROUTE_TABLE: &[(&str, &str, &str)] = &[
        ("router", "GET", "/healthz"),
        ("router", "GET", "/readyz"),
        ("router", "GET", "/me"),
        ("widgets", "GET", "/widgets"),
        ("widgets", "POST", "/widgets"),
        ("widgets", "GET", "/widgets/{id}"),
        ("widgets", "PUT", "/widgets/{id}"),
        ("widgets", "DELETE", "/widgets/{id}"),
        ("widgets", "GET", "/api/v1/widgets/events"),
        ("signup", "POST", "/api/v1/auth/signup"),
        ("signup", "POST", "/api/v1/auth/verify"),
        ("signup", "POST", "/api/v1/auth/verify/resend"),
        ("session", "POST", "/api/v1/auth/login"),
        ("session", "POST", "/api/v1/auth/refresh"),
        ("session", "POST", "/api/v1/auth/logout"),
        ("session", "GET", "/api/v1/auth/me"),
        ("session", "PUT", "/api/v1/auth/me"),
        ("session", "GET", "/api/v1/auth/sessions"),
        ("session", "GET", "/api/v1/auth/sessions/current"),
        ("session", "POST", "/api/v1/auth/sessions/revoke-current"),
        ("session", "POST", "/api/v1/auth/sessions/revoke-all"),
        ("password", "POST", "/api/v1/auth/change-password"),
        ("password", "POST", "/api/v1/auth/forgot-password"),
        ("password", "POST", "/api/v1/auth/validate-code"),
        ("password", "POST", "/api/v1/auth/reset-password"),
        ("oauth", "POST", "/api/v1/auth/google"),
        ("oauth", "GET", "/api/v1/auth/github/login"),
        ("oauth", "GET", "/api/v1/auth/github/callback"),
        ("oauth", "GET", "/api/v1/auth/github/connect-url"),
        ("adminusers", "POST", "/api/v1/auth/upgrade-to-admin"),
        ("adminusers", "GET", "/api/v1/admin/users"),
        ("adminusers", "GET", "/api/v1/admin/users/{id}"),
        ("adminusers", "PUT", "/api/v1/admin/users/{id}"),
        (
            "session",
            "GET",
            "/api/v1/admin/auth-session/users/{id}/sessions",
        ),
        (
            "session",
            "POST",
            "/api/v1/admin/auth-session/users/{id}/revoke-all",
        ),
        ("session", "GET", "/api/v1/admin/auth-session/anomalies"),
        ("realtime", "GET", "/api/v1/ws"),
    ];

    #[test]
    fn go_route_table_parity() {
        // 1. every go route has a rust route: try the identical path, then
        //    the `/api/v1`-stripped shape
        //    (the widget CRUD is mounted unprefixed).
        for (method, path) in GO_METHOD_PATHS {
            let candidates = [Some(*path), path.strip_prefix("/api/v1").or(Some(*path))];
            let hit = ROUTE_TABLE
                .iter()
                .any(|(_, m, p)| m == method && candidates.contains(&Some(*p)));
            assert!(hit, "go route {method} {path} is not served by the router");
        }
        // 2. the router's own surface is fully documented: each ROUTE_TABLE
        //    entry (minus the legacy seam) appears in the ApiDoc spec.
        let doc = openapi();
        let json = serde_json::to_value(&doc).unwrap();
        for (module, method, path) in ROUTE_TABLE {
            if *module == "router" {
                continue; // legacy placeholder-era seam
            }
            let op = &json["paths"][path][&method.to_ascii_lowercase()];
            assert!(
                op.is_object(),
                "route {method} {path} is missing from ApiDoc"
            );
        }
        // 3. the six identity modules are all represented.
        for module in [
            "signup",
            "session",
            "password",
            "oauth",
            "adminusers",
            "realtime",
        ] {
            assert!(
                ROUTE_TABLE.iter().any(|(m, _, _)| *m == module),
                "module {module} must appear in the route table"
            );
        }
    }
}
