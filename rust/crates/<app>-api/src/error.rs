//! HTTP error envelope. Maps `<app>-core::Error` to a JSON envelope:
//!
//! ```json
//! {
//!   "error": {
//!     "code": "not_found",
//!     "title": "Resource not found",
//!     "detail": "...",
//!     "request_id": "req_..."
//!   }
//! }
//! ```
//!
//! 5xx detail is redacted to a generic message (the real error stays
//! server-side in logs); 4xx detail is the error's own message. Rate-limited
//! responses carry a `Retry-After` header.

use axum::{
    Json,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use <app>_core::Error;
use serde_json::Value;
use tracing::error;

// The envelope structs live in `<app>-warden` (the HTTP/auth boundary crate)
// so the warden's auth middleware and this API error mapper share ONE shape
// without a circular import. Re-exported here so existing `crate::error::`
// references (router openapi schemas, handlers) resolve unchanged.
pub use <app>_warden::{ErrorBody, ErrorEnvelope};

/// Client-facing detail for any 5xx. The real error (DB internals, upstream
/// response shape) stays in the server-side log only — never leaked to clients.
const GENERIC_5XX_DETAIL: &str = "An internal error occurred.";

/// Newtype so we can implement `IntoResponse` on it without orphan rules.
/// Wraps a domain `Error` with an optional request-id (set by middleware).
#[derive(Debug)]
pub struct ApiError {
    pub inner: Error,
    pub request_id: Option<String>,
}

impl ApiError {
    #[must_use]
    pub const fn new(inner: Error) -> Self {
        Self {
            inner,
            request_id: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.inner.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // 5xx are logged at error level; 4xx at debug (caller's problem).
        // The full internal/upstream detail stays server-side (logs only);
        // the client gets a generic message so we never leak DB/upstream internals.
        let detail = if status.is_server_error() {
            error!(error = ?self.inner, "request failed");
            GENERIC_5XX_DETAIL.to_string()
        } else {
            tracing::debug!(error = ?self.inner, "client error");
            self.inner.to_string()
        };

        let envelope = ErrorEnvelope {
            error: ErrorBody {
                code: error_code(&self.inner).to_string(),
                title: error_title(&self.inner).to_string(),
                detail,
                request_id: self.request_id,
                extra: extra_for(&self.inner),
            },
        };
        let mut resp = (status, Json(envelope)).into_response();

        // Rate-limited: surface Retry-After so clients back off.
        if let Error::RateLimited { retry_after_secs } = &self.inner
            && let Ok(val) = HeaderValue::from_str(&retry_after_secs.to_string())
        {
            resp.headers_mut().insert("retry-after", val);
        }
        resp
    }
}

const fn error_code(err: &Error) -> &'static str {
    match err {
        Error::NotFound => "not_found",
        Error::Invalid(_) => "invalid_request",
        Error::Unauthorized => "unauthorized",
        Error::Unauthenticated(_) => "unauthenticated",
        Error::Forbidden(_) => "forbidden",
        Error::Conflict(_) => "conflict",
        Error::RateLimited { .. } => "rate_limited",
        Error::QuotaExceeded { .. } => "quota_exceeded",
        Error::Upstream { .. } => "upstream_failure",
        Error::Internal(_) => "internal_error",
    }
}

const fn error_title(err: &Error) -> &'static str {
    match err {
        Error::NotFound => "Resource not found",
        Error::Invalid(_) => "Invalid request",
        Error::Unauthorized | Error::Unauthenticated(_) => "Authentication required",
        Error::Forbidden(_) => "Forbidden",
        Error::Conflict(_) => "Conflict",
        Error::RateLimited { .. } => "Rate limited",
        Error::QuotaExceeded { .. } => "Quota exceeded",
        Error::Upstream { .. } => "Upstream service failed",
        Error::Internal(_) => "Internal server error",
    }
}

fn extra_for(err: &Error) -> Option<Value> {
    match err {
        Error::RateLimited { retry_after_secs } => {
            Some(serde_json::json!({ "retry_after_secs": retry_after_secs }))
        }
        Error::QuotaExceeded {
            quota,
            used,
            reset_at,
        } => Some(serde_json::json!({
            "quota": quota,
            "used": used,
            "reset_at": reset_at,
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let resp = ApiError::from(Error::NotFound).into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn rate_limited_sets_retry_after() {
        let resp = ApiError::from(Error::RateLimited {
            retry_after_secs: 30,
        })
        .into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            resp.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
            Some("30")
        );
    }

    #[tokio::test]
    async fn internal_error_redacts_detail() {
        let resp = ApiError::from(Error::Internal("DB pool exhausted".into())).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let detail = json["error"]["detail"].as_str().unwrap();
        assert!(!detail.contains("DB pool"), "5xx detail must be redacted");
        assert_eq!(detail, GENERIC_5XX_DETAIL);
    }

    #[tokio::test]
    async fn client_error_exposes_detail() {
        let resp =
            ApiError::from(Error::Conflict("email already registered".into())).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let detail = json["error"]["detail"].as_str().unwrap();
        assert!(
            detail.contains("email already registered"),
            "4xx detail should be shown"
        );
    }

    #[tokio::test]
    async fn quota_exceeded_carries_extra() {
        let resp = ApiError::from(Error::QuotaExceeded {
            quota: 1000,
            used: 1000,
            reset_at: "2026-08-01T00:00:00Z".into(),
        })
        .into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["extra"]["quota"], 1000);
        assert_eq!(json["error"]["extra"]["used"], 1000);
        assert_eq!(json["error"]["extra"]["reset_at"], "2026-08-01T00:00:00Z");
    }
}
