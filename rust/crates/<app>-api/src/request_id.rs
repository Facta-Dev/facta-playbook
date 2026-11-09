//! Request-id middleware. Reads `x-request-id` from the incoming request (or
//! generates a fresh `req_<uuid v4>` when absent), stores it in a request
//! extension ([`RequestId`]) so handlers and the `TraceLayer` span can pick it
//! up, and echoes it back as the `x-request-id` response header so callers can
//! correlate logs and traces.
//!
//! Runs as the outermost layer (applied after `TraceLayer`) so the id is
//! available to `make_span_with` before the span is created.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// The request/response header carrying the correlation id.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Prefix for server-generated ids so they are distinguishable from caller ids.
const PREFIX: &str = "req_";

/// Request extension holding the correlation id for this request.
/// Extracted by handlers via `Extension<RequestId>` and by `make_span_with`.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resolve the id for a request: use the caller's `x-request-id` when present
/// and non-empty, otherwise generate `req_<uuid v4>`.
fn resolve_id(headers: &axum::http::HeaderMap) -> String {
    headers
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("{PREFIX}{}", Uuid::new_v4()), str::to_owned)
}

/// Axum middleware: stamp the request id into an extension and echo it back.
/// L6: also stamps the resolved id onto the REQUEST header so inner middleware
/// (like `require_auth` in the warden crate, which cannot import this crate's
/// `RequestId` type) can read it directly from the header.
pub async fn request_id_layer(mut req: Request<Body>, next: Next) -> Response {
    let id = resolve_id(req.headers());
    req.extensions_mut().insert(RequestId(id.clone()));
    // Stamp the header on the request so inner middleware can read it without
    // depending on this crate's RequestId extension type.
    if let Ok(val) = HeaderValue::from_str(&id) {
        req.headers_mut().insert(REQUEST_ID_HEADER, val);
    }

    let mut resp = next.run(req).await;

    // Echo back so the caller always sees an id (even if they didn't send one).
    if let Ok(val) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    async fn echo_id(axum::Extension(id): axum::Extension<RequestId>) -> String {
        id.0
    }

    fn app() -> Router {
        Router::new()
            .route("/echo", get(echo_id))
            .layer(axum::middleware::from_fn(request_id_layer))
    }

    #[tokio::test]
    async fn generates_id_when_header_absent() {
        let res = app()
            .oneshot(Request::builder().uri("/echo").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let header = res
            .headers()
            .get(&REQUEST_ID_HEADER)
            .expect("response must carry x-request-id");
        let id = header.to_str().unwrap();
        assert!(
            id.starts_with(PREFIX),
            "generated id must be prefixed: {id}"
        );
    }

    #[tokio::test]
    async fn echoes_caller_provided_id() {
        let caller_id = "abc-123-trace";
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/echo")
                    .header(&REQUEST_ID_HEADER, caller_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.headers()
                .get(&REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some(caller_id),
            "caller id must be echoed back unchanged"
        );
    }

    #[tokio::test]
    async fn empty_header_is_replaced_with_generated_id() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/echo")
                    .header(&REQUEST_ID_HEADER, "")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = res
            .headers()
            .get(&REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert!(id.starts_with(PREFIX), "empty header should not be echoed");
    }

    #[tokio::test]
    async fn extension_is_available_to_handler() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/echo")
                    .header(&REQUEST_ID_HEADER, "ext-check")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(res.into_body(), 256).await.unwrap();
        assert_eq!(&body[..], b"ext-check");
    }
}
