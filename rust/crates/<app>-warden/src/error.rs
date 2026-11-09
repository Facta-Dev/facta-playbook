//! HTTP error envelope shared by the warden auth middleware and the API's
//! domain-error mapper. Lives in the warden crate so the auth layer can
//! produce a 401 envelope WITHOUT depending on the API crate (no circular
//! dep). The API re-exports these types from its own `error` module.
//!
//! 5xx detail is redacted to a generic message (the real error stays
//! server-side in logs); 4xx detail is the error's own message.

use axum::{
    Json,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use <app>_core::Error;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

/// The JSON envelope returned to clients.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

/// Auth-layer error. Wraps a domain [`Error`] (only the
/// `Unauthenticated`/`Unauthorized` variants are produced by the warden) and
/// renders the same JSON envelope shape as the API's `ApiError`, so clients
/// see one consistent error body across auth and CRUD surfaces.
///
/// L6: carries an optional `request_id` (pulled from the `x-request-id`
/// header by [`require_auth`](crate::require_auth)) so the 401 envelope
/// includes the correlation id.
#[derive(Debug)]
pub struct AuthError {
    inner: Error,
    request_id: Option<String>,
}

impl AuthError {
    #[must_use]
    pub const fn new(inner: Error) -> Self {
        Self {
            inner,
            request_id: None,
        }
    }

    /// Attach a request-id (from the `x-request-id` header/extension) to the
    /// error so it surfaces in the 401 envelope. Builder-style.
    #[must_use]
    pub fn with_request_id(mut self, id: Option<String>) -> Self {
        self.request_id = id;
        self
    }
}

impl From<Error> for AuthError {
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        // Auth errors are always 4xx (401). Detail is the error's own message
        // (generic "invalid access token") — never an upstream/internal leak.
        let status =
            StatusCode::from_u16(self.inner.status_code()).unwrap_or(StatusCode::UNAUTHORIZED);
        tracing::debug!(error = ?self.inner, "auth error");
        let envelope = ErrorEnvelope {
            error: ErrorBody {
                code: auth_code(&self.inner).to_string(),
                title: auth_title(&self.inner).to_string(),
                detail: self.inner.to_string(),
                request_id: self.request_id.clone(),
                extra: None,
            },
        };
        let mut resp = (status, Json(envelope)).into_response();
        // L5: WWW-Authenticate header on 401 — generic, no error_description
        // (avoid leakage). Tells the client the scheme is Bearer + the realm.
        if status == StatusCode::UNAUTHORIZED
            && let Ok(val) =
                HeaderValue::from_str(&format!("Bearer realm=\"{}\"", crate::ACCESS_AUDIENCE))
        {
            resp.headers_mut().insert("www-authenticate", val);
        }
        resp
    }
}

const fn auth_code(err: &Error) -> &'static str {
    match err {
        Error::Unauthorized => "unauthorized",
        _ => "unauthenticated",
    }
}

const fn auth_title(err: &Error) -> &'static str {
    match err {
        Error::Forbidden(_) => "Forbidden",
        _ => "Authentication required",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthenticated_maps_to_401() {
        let resp = AuthError::from(Error::Unauthenticated("bad token".into())).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn envelope_shape_is_consistent() {
        let resp = AuthError::from(Error::Unauthenticated("bad token".into())).into_response();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "unauthenticated");
        assert_eq!(json["error"]["title"], "Authentication required");
    }

    #[tokio::test]
    async fn www_authenticate_header_on_401() {
        // L5: 401 responses must carry a generic WWW-Authenticate: Bearer header.
        let resp = AuthError::from(Error::Unauthenticated("bad token".into())).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let www = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .expect("401 must carry WWW-Authenticate");
        assert!(
            www.starts_with("Bearer realm="),
            "must be Bearer scheme: {www}"
        );
    }

    #[tokio::test]
    async fn request_id_surfaces_in_envelope() {
        // L6: a request_id set on AuthError must appear in the JSON envelope.
        let resp = AuthError::new(Error::Unauthenticated("bad token".into()))
            .with_request_id(Some("req-test-123".into()))
            .into_response();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["request_id"], "req-test-123");
    }
}
