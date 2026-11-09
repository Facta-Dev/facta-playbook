//! Widget CRUD routes. Each handler is annotated with `#[utoipa::path]` so the
//! `OpenApi` builder in `router.rs` can register it. Handlers are thin: they
//! validate at the HTTP boundary, delegate to `WidgetService`, and map domain
//! errors into `ApiError`.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use <app>_core::{Action, Error, Widget, WidgetId, WidgetKind, can};
use <app>_loom::EventBus;
use <app>_warden::Principal;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// C1: check that the caller has at least one role granting `action`. A
/// multi-role principal is allowed if ANY role permits the action. Returns
/// `Error::Forbidden` on deny so the client sees a 403 (authenticated but not
/// allowed) — distinct from the 401 the middleware returns for a bad token.
fn require_action(principal: &Principal, action: Action) -> Result<(), ApiError> {
    if principal.roles.iter().any(|r| can(*r, action)) {
        Ok(())
    } else {
        Err(Error::Forbidden(format!(
            "role(s) {:?} may not {:?} widgets",
            principal.roles, action
        ))
        .into())
    }
}

/// Request body for `POST /widgets`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWidgetRequest {
    /// Human-readable name. Must not be blank.
    pub name: String,
    /// Widget category. One of `sprocket`, `cog`, `flange`.
    pub kind: WidgetKind,
}

/// Request body for `PUT /widgets/{id}`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWidgetRequest {
    /// Human-readable name. Must not be blank.
    pub name: String,
    /// Widget category. One of `sprocket`, `cog`, `flange`.
    pub kind: WidgetKind,
}

/// List all widgets.
#[utoipa::path(
    get,
    path = "/widgets",
    tag = "widgets",
    responses(
        (status = 200, description = "List of all widgets", body = Vec<Widget>),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Authenticated but not allowed", body = crate::error::ErrorEnvelope),
        (status = 500, description = "Internal error", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn list_widgets(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<Vec<Widget>>, ApiError> {
    require_action(&principal, Action::Read)?;
    Ok(Json(state.widgets.list().await?))
}

/// Fetch a single widget by id.
#[utoipa::path(
    get,
    path = "/widgets/{id}",
    tag = "widgets",
    params(("id" = uuid::Uuid, Path, description = "Widget id")),
    responses(
        (status = 200, description = "Widget found", body = Widget),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Authenticated but not allowed", body = crate::error::ErrorEnvelope),
        (status = 404, description = "No widget with that id", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Widget>, ApiError> {
    require_action(&principal, Action::Read)?;
    let widget = state.widgets.get(WidgetId(id)).await?;
    Ok(Json(widget))
}

/// Create a new widget.
#[utoipa::path(
    post,
    path = "/widgets",
    tag = "widgets",
    request_body = CreateWidgetRequest,
    responses(
        (status = 201, description = "Widget created", body = Widget),
        (status = 400, description = "Invalid input (blank name, unknown kind)", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Authenticated but not allowed", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn create_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<CreateWidgetRequest>,
) -> Result<(StatusCode, Json<Widget>), ApiError> {
    require_action(&principal, Action::Create)?;
    let widget = state.widgets.create(&req.name, req.kind).await?;
    Ok((StatusCode::CREATED, Json(widget)))
}

/// Update an existing widget's mutable fields. `404` if the id does not exist
/// (update is NOT an upsert — it never creates a row).
#[utoipa::path(
    put,
    path = "/widgets/{id}",
    tag = "widgets",
    params(("id" = uuid::Uuid, Path, description = "Widget id")),
    request_body = UpdateWidgetRequest,
    responses(
        (status = 200, description = "Widget updated", body = Widget),
        (status = 400, description = "Invalid input (blank name, unknown kind)", body = crate::error::ErrorEnvelope),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Authenticated but not allowed", body = crate::error::ErrorEnvelope),
        (status = 404, description = "No widget with that id", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn update_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateWidgetRequest>,
) -> Result<Json<Widget>, ApiError> {
    require_action(&principal, Action::Update)?;
    let widget = state
        .widgets
        .update(WidgetId(id), &req.name, req.kind)
        .await?;
    Ok(Json(widget))
}

/// Delete a widget by id. Idempotent — `404` only if the id never existed.
#[utoipa::path(
    delete,
    path = "/widgets/{id}",
    tag = "widgets",
    params(("id" = uuid::Uuid, Path, description = "Widget id")),
    responses(
        (status = 204, description = "Widget deleted (or already absent)"),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
        (status = 403, description = "Authenticated but not allowed", body = crate::error::ErrorEnvelope),
        (status = 404, description = "No widget with that id", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn delete_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, ApiError> {
    require_action(&principal, Action::Delete)?;
    let deleted = state.widgets.delete(WidgetId(id)).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound.into())
    }
}

/// Drop-guard for the SSE subscription: a hub `CancelFn` unsubscribes by
/// being CALLED (a boxed `FnOnce` does nothing on plain drop — go parity:
/// `defer cancel()`). Dropping the guard at any stream end — natural close,
/// channel close, or a client-disconnect mid-await — unsubscribes exactly
/// once.
struct CancelGuard(Option<<app>_loom::CancelFn>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel();
        }
    }
}

/// SSE fan-out of the caller's private realtime room (go `widgets/events`
/// handler parity: `Subscribe(realtime.RoomUser(owner))`). Authenticated
/// (cookie) because browsers consume SSE via `EventSource`, which cannot set
/// headers. Yields an initial `retry: 2000`, then each hub frame as
/// `event: <type>`/`data: <json>` (`{"type","payload"}` with `payload`
/// omitted when null). Hub publishes never block (full buffer = drop), and a
/// disconnect is a normal exit — the dropped stream unsubscribes.
#[utoipa::path(
    get,
    path = "/api/v1/widgets/events",
    tag = "widgets",
    responses(
        (status = 200, description = "text/event-stream of the caller's room events; disconnect = unsubscribe"),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorEnvelope),
    )
)]
pub async fn event_stream(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
) -> impl axum::response::IntoResponse {
    use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};

    let (rx, cancel) = state
        .hub
        .subscribe(&<app>_loom::room_user(&principal.id.to_string()));
    let stream = futures_util::stream::unfold(
        (rx, CancelGuard(Some(cancel)), false),
        |(mut rx, guard, sent_retry)| async move {
            if !sent_retry {
                let item = SseEvent::default().retry(std::time::Duration::from_millis(2000));
                return Some((Ok::<_, std::convert::Infallible>(item), (rx, guard, true)));
            }
            let ev = rx.recv().await?;
            let json = <app>_loom::frame(&ev).to_string();
            let item = SseEvent::default().event(ev.r#type.as_str()).data(json);
            Some((Ok(item), (rx, guard, true)))
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_deserialises() {
        let json = r#"{"name":"cog-1","kind":"cog"}"#;
        let req: CreateWidgetRequest = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(req.name, "cog-1");
        assert_eq!(req.kind, WidgetKind::Cog);
    }

    #[test]
    fn create_request_rejects_unknown_kind() {
        let json = r#"{"name":"x","kind":"nonexistent"}"#;
        let err = serde_json::from_str::<CreateWidgetRequest>(json);
        assert!(err.is_err(), "unknown kind must fail deserialisation");
    }
}
