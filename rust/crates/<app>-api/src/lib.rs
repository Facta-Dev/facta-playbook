//! `<app>-api` — the library root re-exports `AppState`, `Config`, and `router`.
pub mod auth;
pub mod config;
pub mod error;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod request_id;
pub mod router;
pub mod routes;
pub mod state;
pub mod telemetry;

pub use config::Config;
pub use state::AppState;
