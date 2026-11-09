//! Tracing bootstrap. `EnvFilter` from `RUST_LOG` (with config fallback),
//! JSON layer in production / compact in dev, an OTLP exporter seam, and an
//! RAII guard whose `Drop` flushes on shutdown.

use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::Config;

/// RAII guard. Keep alive for the lifetime of the process; drop on shutdown
/// to flush exporters cleanly.
pub struct TelemetryGuard {
    // ponytail: hold `SdkTracerProvider` here when the OTLP exporter is wired
    // (see init). `Drop` should call `provider.shutdown()` to flush the batch
    // exporter. Ceiling: clean OTLP flush on process exit. Until then the guard
    // is a no-op drop — the fmt layer needs no explicit flush.
}

impl std::fmt::Debug for TelemetryGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryGuard").finish()
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // ponytail: `provider.shutdown()` here when the OTLP exporter is wired.
        // Ceiling: clean shutdown of batch span exporters on process exit.
    }
}

/// Install the global tracing subscriber. Returns a guard that should be held
/// for the lifetime of the process.
///
/// # Errors
/// Currently always returns `Ok`; the guard is returned even if a global
/// subscriber was already installed (downgraded to a warning). The `Result`
/// return type is retained so future OTLP wiring can propagate exporter-build
/// errors without changing the call site.
pub fn init(cfg: &Config) -> Result<TelemetryGuard> {
    // RUST_LOG is the canonical source; <APP>_LOG_LEVEL is the fallback.
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if cfg.is_production() {
        fmt::layer().json().with_target(true).boxed()
    } else {
        fmt::layer().compact().with_target(true).boxed()
    };

    // ponytail: OTLP exporter seam. To wire:
    // 1. Add `opentelemetry-otlp` and `opentelemetry_sdk` to workspace deps.
    // 2. When `cfg.otel_exporter_otlp_endpoint` is `Some(endpoint)`:
    //    build a `SpanExporter` with `.with_tonic().with_endpoint(endpoint)`,
    //    create an `SdkTracerProvider` with a `Resource` (service.name =
    //    "<app>-api"), install it via `opentelemetry::global::set_tracer_provider`,
    //    and add `tracing_opentelemetry::layer().with_tracer(tracer)` to the
    //    subscriber below. Store the provider in `TelemetryGuard` for shutdown.
    // 3. Empty / `None` endpoint -> no exporter, no socket (offline-guarded).
    // Ceiling: full OTLP trace export to a collector.
    if cfg.otel_exporter_otlp_endpoint.is_some() {
        eprintln!(
            "warning: OTLP endpoint configured but exporter not yet wired \
             (ponytail: see telemetry::init)"
        );
    }

    // `try_init` fails only if a global default is already installed (e.g.
    // a test fixture or a second init call). That is benign — the existing
    // subscriber stays active — so we downgrade it to a warning.
    if tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .is_err()
    {
        eprintln!("warning: tracing subscriber already set; keeping existing");
    }

    Ok(TelemetryGuard {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_returns_guard() {
        let cfg = Config::test_default();
        let guard = init(&cfg);
        assert!(guard.is_ok(), "init should return a guard");
    }
}
