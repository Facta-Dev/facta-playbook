//! Realtime — go `features/realtime` parity: the single `GET /api/v1/ws`
//! upgrade. UNAUTHENTICATED at the HTTP layer on purpose: the loom's
//! `serve_ws` upgrades FIRST, then authenticates `?token=` and closes with
//! `4001 authentication_failed` on any failure — clients react at the
//! close-frame level, not the HTTP status level.
use std::sync::Arc;

use axum::extract::{RawQuery, State, WebSocketUpgrade};
use axum::response::Response;
use <app>_loom::hub::serve_ws;

use crate::state::AppState;

/// The `/ws` upgrade — handler inputs are passed through verbatim to
/// [`serve_ws`] (the shared JWT codec + the hub). Async by contract: axum
/// 0.8 implements `Handler` only for future-returning fns — no awaits here.
#[allow(clippy::unused_async)] // axum 0.8 Handler bound requires a Future
#[utoipa::path(
    get,
    path = "/api/v1/ws",
    tag = "realtime",
    params(("token" = String, Query, description = "Access token (query-raw; verified after the upgrade)")),
    responses(
        (status = 101, description = "Upgrade accepted — the pump streams hub events (or closes 4001)"),
        (status = 400, description = "Not a websocket-upgrade request (axum extractor rejection)"),
    )
)]
pub async fn ws(
    State(state): State<AppState>,
    ws_upgrade: WebSocketUpgrade,
    RawQuery(query): RawQuery,
) -> Response {
    let codec = Arc::new(state.jwt_codec.clone());
    let hub = state.hub;
    serve_ws(ws_upgrade, query, codec, hub)
}

#[cfg(test)]
mod tests {
    /// The route contract lives in the router tests (`ws is reachable …`);
    /// this row only pins the loom constants the handler relies on.
    #[test]
    fn auth_failure_close_frame_vocabulary_is_4001() {
        assert_eq!(<app>_loom::hub::AUTH_FAILED_CLOSE_CODE, 4001);
        assert_eq!(
            <app>_loom::hub::AUTH_FAILED_CLOSE_REASON,
            "authentication_failed"
        );
    }
}
