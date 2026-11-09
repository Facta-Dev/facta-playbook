//! HTTP route handlers grouped by concern. Each submodule owns its
//! `#[utoipa::path]`-annotated handlers; the router mounts them and the
//! `OpenApi` builder references them in `paths(...)`.
//!
//! The identity/realtime stack mirrors the go slice layout 1:1:
//!
//! | module        | go slice      | paths                                           |
//! |---------------|---------------|-------------------------------------------------|
//! | [`signup`]    | authsignup    | `/api/v1/auth/{signup,verify,verify/resend}`    |
//! | [`session`]   | authsession   | login/refresh/logout/me/sessions + admin trio   |
//! | [`password`]  | authpassword  | forgot/validate/reset/change                    |
//! | [`oauth`]     | authoauth     | google/github login+callback+connect-url        |
//! | [`admin_users`]| adminusers   | `/api/v1/admin/users*` + upgrade-to-admin       |
//! | [`realtime`]  | realtime      | `GET /api/v1/ws`                                |
//!
//! Shared wire shapes (`MessageResponse`, `SessionItem`, …) sit here so
//! sibling modules render a single schema name in the spec.
pub mod admin_users;
pub mod oauth;
pub mod password;
pub mod realtime;
pub mod session;
pub mod signup;
pub mod widgets;

pub use widgets::{
    CreateWidgetRequest, UpdateWidgetRequest, create_widget, delete_widget, get_widget,
    list_widgets, update_widget,
};

use std::net::SocketAddr;

use crate::config::Config;
use axum::http::HeaderMap;
use axum::response::Response;
use chrono::{DateTime, Utc};
use <app>_auth::SessionTokens;
use <app>_core::Session;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── shared wire shapes (go generated models, one definition per name) ────

/// go `api.MessageResponse` parity — the uniform acknowledgement body.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

/// go `api.EmailRequest` parity — verify/resend, forgot-password and the
/// upgrade-to-admin bootstrap all take just an address.
#[derive(Debug, Deserialize, ToSchema)]
pub struct EmailRequest {
    pub email: String,
}

/// go `api.AuthTokens` parity — the login/refresh/oauth token pair.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Fixed wire value `bearer` (go `api.Bearer`).
    pub token_type: String,
    /// Access TTL seconds at mint (go `accessTTL.Seconds()`).
    pub expires_in: i64,
}

/// go `sessionItem` parity — one session row without its refresh token.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionItem {
    pub id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
}

impl From<&Session> for SessionItem {
    fn from(s: &Session) -> Self {
        Self {
            id: s.id.0.to_string(),
            user_id: s.user_id.0.to_string(),
            ip_address: s.ip_address.clone(),
            user_agent: s.user_agent.clone(),
            created_at: s.created_at,
            last_activity: s.last_activity,
            expires_at: s.expires_at,
            is_active: s.is_active,
        }
    }
}

/// go `handlers.tokenPair` parity: `token_type: "bearer"`,
/// `expires_in` = the access TTL in seconds (read off the freshly minted
/// expiry instant, so a minted pair always reports the full TTL).
pub(crate) fn token_pair(tokens: &SessionTokens) -> AuthTokens {
    AuthTokens {
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        token_type: "bearer".to_owned(),
        expires_in: (tokens.expires_at - Utc::now()).num_seconds().max(0),
    }
}

/// go `handlers.setCookie` parity: append the access-token `Set-Cookie`
/// only when cookie mode is on (cfg).
pub(crate) fn set_auth_cookie_on(resp: &mut Response, cfg: &Config, token: &str) {
    if let Some(name) = cfg.auth_cookie_name() {
        <app>_warden::set_auth_cookie(resp.headers_mut(), name, token, cfg.cookie_secure());
    }
}

/// go `authx.ClearAuthCookie` parity, cookie-mode gated like
/// [`set_auth_cookie_on`]; a no-op header touch otherwise.
pub(crate) fn clear_auth_cookie_on(resp: &mut Response, cfg: &Config) {
    if let Some(name) = cfg.auth_cookie_name() {
        <app>_warden::clear_auth_cookie(resp.headers_mut(), name, cfg.cookie_secure());
    }
}

/// go `revokeResponse` parity (`POST …/revoke-current`).
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionRevoked {
    pub revoked: bool,
}

/// go `revokeCountResponse` parity (`POST …/revoke-all`).
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionRevokedCount {
    pub revoked_count: i64,
}

/// go `anomalyItem` parity (admin session-hygiene report).
#[derive(Debug, Serialize, ToSchema)]
pub struct AnomalyItem {
    /// go field `Type` (wire `anomaly_type`).
    pub anomaly_type: String,
    pub count: i64,
    #[schema(value_type = Object, additional_properties = true)]
    pub details: serde_json::Value,
}

impl From<<app>_core::Anomaly> for AnomalyItem {
    fn from(a: <app>_core::Anomaly) -> Self {
        Self {
            anomaly_type: a.kind,
            count: a.count,
            details: a.details,
        }
    }
}

// ── shared request-context helpers (go httpx parity) ─────────────────────

/// go `httpx.RequestIP` parity: the RIGHTMOST `X-Forwarded-For` hop (the
/// proxy appends the real client at the edge) falling back to the remote
/// address when the header is absent — `None` without either (the pool of
/// facts the tests see, where no `ConnectInfo` is installed).
pub(crate) fn request_ip(headers: &HeaderMap, remote: Option<SocketAddr>) -> Option<String> {
    if let Some(right) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|fwd| fwd.rsplit(',').next())
    {
        return Some(right.trim().to_owned());
    }
    remote.map(|a| a.to_string())
}

/// go `r.UserAgent()` parity — `None` when the header is absent (the go
/// default is `""`; the rust row stores SQL NULL, same "unknown" meaning).
pub(crate) fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// go `url.Values.Encode()` component parity (RFC 3986 unreserved set,
/// space → `+`, uppercase `%XX`) — used to build redirect queries and the
/// token fragment without pulling in a URL crate.
pub(crate) fn form_urlencode(value: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(value.len() + 16);
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            other => {
                let _ = write!(out, "%{other:02X}"); // String write is infallible
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ip_takes_the_rightmost_xff_hop() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            "10.0.0.1:9, 203.0.113.7:443".parse().expect("value"),
        );
        assert_eq!(
            request_ip(&h, Some("127.0.0.1:1".parse().expect("addr"))),
            Some("203.0.113.7:443".to_owned())
        );
        assert_eq!(
            request_ip(
                &HeaderMap::new(),
                Some("127.0.0.1:1".parse().expect("addr"))
            ),
            Some("127.0.0.1:1".to_owned()),
            "no header: the remote address wins"
        );
        assert_eq!(request_ip(&HeaderMap::new(), None), None);
    }

    #[test]
    fn form_urlencode_matches_go_query_component_rules() {
        assert_eq!(form_urlencode("plain-token-1.2"), "plain-token-1.2");
        assert_eq!(form_urlencode("a b"), "a+b");
        assert_eq!(form_urlencode("a&b=c/d"), "a%26b%3Dc%2Fd");
        assert_eq!(form_urlencode("~*_"), "~%2A_");
    }
}
