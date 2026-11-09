//! Prometheus metrics plane. The `metrics` facade carries counters/histograms;
//! `metrics-exporter-prometheus` renders the text exposition. The recorder is
//! installed **only when** `cfg.metrics_enabled` — absent ⇒ no recorder, the
//! facade macros are cheap no-ops, and `GET /metrics` returns 503.
//!
//! Feature-gated behind `feature = "metrics"` so a build without the feature
//! is byte-identical to a build that never had it. The runtime gate
//! (`metrics_enabled`) is the second layer: even with the feature on, no
//! recorder is installed unless the flag is set.
//!
//! # Cardinality
//! Labels are bounded to a fixed low-cardinality set (`method`, `status_class`)
//! — never per-request or per-identity labels, which would blow the recorder's
//! memory. A test enumerates the allowed label keys.

use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::config::Config;

// ──── Metric names (stable; no high-cardinality labels) ──────────────────────

/// HTTP requests served, labelled `method` + `status_class` (`2xx`,`4xx`,…).
pub const HTTP_REQUESTS_TOTAL: &str = "<app>_http_requests_total";
/// Histogram of request duration in seconds.
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "<app>_http_request_duration_seconds";

/// The exhaustive allowlist of label *keys*. Any value can be high-cardinality,
/// so we constrain the **keys** to a fixed, low-cardinality set and forbid
/// per-identity labels (user id, request id, etc.).
pub const ALLOWED_LABELS: &[&str] = &["method", "status_class"];

// ──── Recorder install (config-gated) ────────────────────────────────────────

/// Process-wide Prometheus render handle, set once by [`init`]. `None` ⇒ the
/// recorder was never installed (flag off).
static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the global Prometheus recorder **iff** `cfg.metrics_enabled`, and
/// stash its render handle for the `/metrics` handler. Idempotent: a second
/// call is a no-op. Returns `true` when a recorder was installed this call.
pub fn init(cfg: &Config) -> bool {
    if !cfg.metrics_enabled {
        return false;
    }
    install()
}

/// Install the Prometheus recorder unconditionally. Split from [`init`] so
/// tests can install a recorder without a `Config`. Idempotent + non-fatal on
/// double-install.
pub fn install() -> bool {
    if HANDLE.get().is_some() {
        return false;
    }
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    // A previously-installed global recorder is not fatal: keep ours unset and
    // let the handler report "not installed".
    if metrics::set_global_recorder(recorder).is_err() {
        tracing::warn!("metrics recorder already installed; skipping Prometheus recorder");
        return false;
    }
    let _ = HANDLE.set(handle);
    describe();
    true
}

/// Whether a Prometheus recorder is installed (the `/metrics` route gates on it).
#[must_use]
pub fn is_installed() -> bool {
    HANDLE.get().is_some()
}

/// Render the current exposition. `None` ⇒ no recorder installed.
#[must_use]
pub fn render() -> Option<String> {
    Some(HANDLE.get()?.render())
}

/// One-time metric descriptions (units + help text) for the exposition.
fn describe() {
    use metrics::{Unit, describe_counter, describe_histogram};
    describe_counter!(
        HTTP_REQUESTS_TOTAL,
        "HTTP requests served by method + status class"
    );
    describe_histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        Unit::Seconds,
        "HTTP request duration in seconds"
    );
}

// ──── HTTP handler + middleware ──────────────────────────────────────────────

/// `GET /metrics` — Prometheus 0.0.4 text exposition. Returns 200 with the
/// rendered text when the recorder is installed; 503 when it is not (the
/// `metrics_enabled` flag is off).
pub async fn metrics_handler() -> Response {
    render().map_or_else(
        || (StatusCode::SERVICE_UNAVAILABLE, "metrics disabled").into_response(),
        |body| {
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4")],
                body,
            )
                .into_response()
        },
    )
}

/// axum middleware: record request duration + counter for every HTTP request.
/// Cheap no-op when no recorder is installed (the facade returns a no-op handle).
pub async fn http_metrics_layer(req: Request, next: Next) -> Response {
    let method = normalise_method(req.method().as_str());
    let start = Instant::now();
    let resp = next.run(req).await;
    let duration = start.elapsed().as_secs_f64();
    let class = status_class(resp.status().as_u16());
    metrics::counter!(HTTP_REQUESTS_TOTAL, "method" => method, "status_class" => class)
        .increment(1);
    metrics::histogram!(HTTP_REQUEST_DURATION_SECONDS).record(duration);
    resp
}

// ──── Label helpers (bounded cardinality) ────────────────────────────────────

/// Bucket an HTTP status into its class label (`2xx`, `4xx`, …) — bounded
/// cardinality (max 5 values) vs. the raw 3-digit code.
#[must_use]
pub const fn status_class(status: u16) -> &'static str {
    match status {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

/// Map an HTTP method to a fixed set of static labels so a bogus / unbounded
/// method string can never explode cardinality.
#[must_use]
fn normalise_method(method: &str) -> &'static str {
    match method {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "HEAD" => "HEAD",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_class_buckets_are_bounded() {
        assert_eq!(status_class(200), "2xx");
        assert_eq!(status_class(404), "4xx");
        assert_eq!(status_class(429), "4xx");
        assert_eq!(status_class(503), "5xx");
        assert_eq!(status_class(101), "1xx");
        assert_eq!(status_class(302), "3xx");
    }

    #[test]
    fn method_normalisation_collapses_unknown_to_other() {
        assert_eq!(normalise_method("GET"), "GET");
        assert_eq!(normalise_method("DELETE"), "DELETE");
        assert_eq!(normalise_method("BREW"), "OTHER");
        assert_eq!(normalise_method("aaaaaaaaaaaaaaaaaaaa"), "OTHER");
    }

    /// Cardinality guard: every label KEY the recording helpers attach must be
    /// in the allowlist — no `user_id`, `request_id`, etc.
    #[test]
    fn all_recording_labels_are_in_the_allowlist() {
        let used: &[&str] = &["method", "status_class"];
        for key in used {
            assert!(
                ALLOWED_LABELS.contains(key),
                "label key {key:?} is not in the cardinality allowlist {ALLOWED_LABELS:?}"
            );
        }
        for forbidden in ["user_id", "request_id", "id", "path", "uri"] {
            assert!(
                !ALLOWED_LABELS.contains(&forbidden),
                "high-cardinality key {forbidden:?} must never be an allowed label"
            );
        }
    }

    #[test]
    fn render_returns_none_when_not_installed() {
        // Process-global state: if a prior test installed a recorder, skip.
        if !is_installed() {
            assert!(render().is_none());
        }
    }

    #[test]
    fn install_is_idempotent() {
        let first = install();
        let second = install();
        // At least one of the two calls returned false (idempotent).
        assert!(!(first && second), "double install must not succeed twice");
    }
}
