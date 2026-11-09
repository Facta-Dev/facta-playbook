---
name: "<app> Add Handler"
description: "Scaffold a new axum 0.8 handler in <app>: handler signature, extractor order, utoipa #[utoipa::path] annotation, route registration in crates/<app>-api/src/router.rs, and AppState wiring, as one change across the <scope> slice."
division: backend
---

# <app> Add Handler

A reusable scaffolding workflow for adding an HTTP capability to the `<app>` API. An agent (e.g. the API designer role) invokes this skill to produce the handler, the OpenAPI annotation, the route registration, and the AppState wiring together — so a route never lands without its spec and its wiring. The output is a design sketch plus a registration checklist.

## When to Activate

- A new endpoint is requested on the `<app>` API (a new verb on an existing path, or a new path).
- An existing handler needs a new extractor (Principal, Query, Path, Json) or a new error variant.
- The OpenAPI spec must register a route that already exists in code (drift fix).
- A reviewer asks for the handler + spec + wiring bundle before approving an API change.

Do not activate for non-HTTP changes (domain logic, persistence, migrations) — those belong to other skills.

## Core Principles

- The handler is an adapter, not a service. Extract, call a `crates/<app>-widgets` / `<app>-core` service, map to `ErrorEnvelope`, return. No SQL, no domain logic in the handler.
- The OpenAPI spec is generated, not hand-written. `#[utoipa::path]` is the source of truth; `openapi.json` is the artifact. `just gen-check` failing means the annotation and the spec disagree — fix the annotation, not the spec.
- The composition root stays thin. A new `AppState` field is `Arc<dyn Trait>` + wired in `state.rs`; logic lives in the service crates.
- Registration is centralized. `crates/<app>-api/src/router.rs` is the one place routes are mounted.
- Errors are typed, not stringy. Every domain error the service can return maps to a typed `ErrorEnvelope` variant with a deliberate status code.

## Rules

- Anti-hallucination: do not invent crate names, service methods, or error variants that are not in the tree. Every wiring step cites a file that exists.
- Every stat needs a source. If you cite the axum extractor order rule, cite the axum docs; if you cite a status code semantics, cite RFC 9110. No unsourced numbers.
- Do not emit the final design as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (handler / utoipa / wiring / checklist).
- Verify the cited service method exists in `crates/<app>-widgets` / `<app>-core` before wiring it; a handler that calls a method the service does not expose is a hallucination.
- Every handler must carry a matching `#[utoipa::path]` annotation; `just gen-check` must stay green.
- Routes are registered in `router.rs`, never inline in `main.rs` or a route module's `mod.rs`.
- Auth-protected handlers take `Principal` via the `require_auth` middleware; public handlers are explicitly marked and rare.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| axum 0.8 extractor order is Path, Query, Json, State, Extension | axum documentation, extractors | 2024 | <TODO: primary doc link> |
| utoipa 5 #[utoipa::path] is the source of truth for the generated spec | utoipa documentation, derive macros | 2024 | <TODO: primary doc link> |
| HTTP status code semantics (400/401/403/404/409/422/500) | RFC 9110, status codes | 2022 | <TODO: primary doc link> |
| require_auth middleware sets Principal via from_fn_with_state | axum middleware docs | 2024 | <TODO: primary doc link> |

## Output Format

A design sketch plus a registration checklist, anchored to the skeleton file layout. Plain text — not a fenced block:

```
DESIGN  POST /widgets  (crates/<app>-api/src/routes/widgets.rs)

  handler:
    async fn create_widget(
        State(state): State<AppState>,
        Json(body): Json<CreateWidgetRequest>,
    ) -> Result<(StatusCode, Json<Widget>), ApiError>

  utoipa:
    #[utoipa::path(post, path = "/widgets", tag = "widgets",
        request_body = CreateWidgetRequest, responses(
            (status = 201, body = Widget),
            (status = 422, body = ErrorEnvelope, description = "validation"),
        ))]

  wiring:
    router.rs  -> .route("/widgets", post(create_widget))
    state.rs   -> AppState.widgets: WidgetService (already wired; no new field)
    openapi    -> add to #[derive(OpenApi)] paths; run just gen-openapi + commit openapi.json

CHECKLIST
  [ ] handler is a thin adapter — no business logic; calls the service
  [ ] #[utoipa::path] present; just gen-check green (no openapi.json drift)
  [ ] route registered in router.rs (not inline)
  [ ] ErrorEnvelope variants cover the failure modes; no leaky 500s
  [ ] AppState unchanged OR new field is Arc<dyn Trait> + wired in state.rs
```

Summary line: `add-handler: N handler(s) designed, M registration step(s) — verdict: APPROVE | REVISE`.
