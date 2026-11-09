# PLAYBOOK_BACKEND.md — backend architecture contract (Rust)

Architecture contract for the `<app>` Rust backend. The one rule that carries
the whole file: **dependencies point inward, and every capability is a crate**.
A change that violates a rule here is a defect, not a style preference.

Premise for actors and crates: `MEMORY.md`.

---

## 1. The model in one screen

`<app>` is a Cargo workspace, edition 2024, MSRV 1.88, toolchain pinned in
`rust-toolchain.toml` (1.93.1):

- **hexagonal**: `<app>-core` (pure domain) → `<app>-ports` (traits) →
  slices (use cases) → `<app>-api` (composition root);
- **vertical slices**: each capability is its own crate — `widgets`, `auth`,
  `adminusers` — owning one flow end-to-end;
- **infrastructure crates** at the edge: `warden` (auth/crypto), `mailer`
  (email), `loom` (orchestration, realtime, workers);
- **clients**: `sdk` (typed HTTP client), `cli` (thin CLI over the SDK).

```text
                 ┌──────────────── <app>-api (composition root) ───────────────┐
                 │ config · state · router · error mapping · metrics · main   │
                 └───────┬───────────────┬───────────────┬────────────┬───────┘
                         │               │               │            │
              <app>-loom │      <app>-auth │   <app>-adminusers │  <app>-widgets
           (orchestrator)│      (slice)     │      (slice)       │    (slice)
                         └───────┬───────┴───────┬───────┴────────────┘
                                 │            <app>-warden · <app>-mailer
                                 │            (infra adapters)
                          <app>-ports (traits + Pg adapters + mocks)
                                 │
                          <app>-core (pure domain, zero I/O)
```

---

## 2. Crate roles and the allowed edges

| Crate | Role | May depend on |
|---|---|---|
| `<app>-core` | Domain types, enums, `Error`, RBAC `can()` | **nothing in the workspace** |
| `<app>-ports` | Repository traits (`domain/`), Postgres adapters (`adapters/`), mocks (`mock/`) | core |
| `<app>-widgets` | Example vertical slice (use case) | core, ports |
| `<app>-auth` | Signup, session, password, OAuth use cases | core, ports, warden |
| `<app>-adminusers` | Admin roster, rank gate, audit write | core, ports |
| `<app>-warden` | HS256 JWT codec, argon2, opaque tokens, cookies, OAuth clients, auth middleware | core |
| `<app>-mailer` | `Mailer` trait + Noop/Provider/Console/SMTP senders, templates | core |
| `<app>-loom` | Orchestrator: realtime hub, outbox drain, janitor, worker binary | core, ports, slices, mailer, warden |
| `<app>-api` | Composition root: config, `AppState`, router, routes, error mapping, telemetry, metrics | any crate |
| `<app>-sdk` | Typed HTTP client | core |
| `<app>-cli` | CLI entrypoint | core, sdk |

**Forbidden edges**

| Edge | Why |
|---|---|
| `core` → any workspace crate | the domain is zero-I/O |
| leaf slice → leaf slice | slices stay independent |
| consumer → concrete `Pg*` adapter | consumers hold `Arc<dyn Trait>` |
| any crate → `api` | nothing depends on the root |

**Enforcement**

1. The Cargo graph itself: add an edge only through the member's
   `[dependencies]`; a cycle does not compile.
2. `Arc<dyn Trait>` at the seam: a service field is
   `Arc<dyn WidgetRepository>`, never `PgWidgetRepository`.
3. `unsafe_code = "forbid"` and clippy `all` + `pedantic` + `nursery`
   (`-D warnings` in the gate) are inherited by every member via
   `[lints] workspace = true`.
4. `cargo-deny` (`deny.toml`): yanked crates denied, wildcard deps denied,
   license allowlist, crates.io only.
5. Review catches the "depends on the trait but downcasts to the adapter"
   smell the compiler can't.

---

## 3. The slice shape

A Rust slice is a crate that owns one capability. The shipped shape:

```text
crates/<app>-<scope>/
├── Cargo.toml            # [lints] workspace = true; path deps on core/ports
└── src/
    ├── lib.rs            # <Scope>Service + business rules
    ├── fakes.rs          # optional in-crate doubles for tests
    └── tests.rs          # optional separate unit-test module
```

### 3.1 The service

```rust
#[derive(Clone)]
pub struct WidgetService {
    repo: Arc<dyn WidgetRepository>,
}

impl WidgetService {
    #[must_use]
    pub fn new(repo: Arc<dyn WidgetRepository>) -> Self {
        Self { repo }
    }

    pub async fn create(&self, name: &str, kind: WidgetKind) -> Result<Widget> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("name is required".into()));
        }
        self.repo.upsert(Widget::new(name, kind)).await
    }
}
```

Rules:

- The service takes its collaborators through the constructor, as
  `Arc<dyn Trait>`; it never reads env, never opens a pool, never knows axum.
- Every public method returns `core::Result<T>`; foreign errors are mapped at
  the adapter, never surfaced.
- Validation that is a business rule lives here; validation that is a wire
  concern (JSON shape, size limits) lives at the handler.
- Unit tests use the mock adapters from `<app>-ports` (feature `mock`), not a
  live database.

### 3.2 Ports and adapters

| Location | Content |
|---|---|
| `crates/<app>-ports/src/domain/<name>_repository.rs` | trait: pure-domain signature, `#[async_trait]`, `Send + Sync + 'static` |
| `crates/<app>-ports/src/adapters/<name>_pg.rs` | the **only** place SQL lives; `FromRow` row + `into_domain()` mapper; maps `sqlx::Error` → `core::Error` |
| `crates/<app>-ports/src/mock/<name>_mock.rs` | hand-written in-memory double behind `feature = "mock"` |
| `crates/<app>-ports/src/lib.rs` | re-exports so consumers import from one place |

```rust
#[async_trait]
pub trait WidgetRepository: Send + Sync + 'static {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>>;
    async fn list(&self) -> Result<Vec<Widget>>;
    async fn upsert(&self, widget: Widget) -> Result<Widget>;
    async fn delete(&self, id: WidgetId) -> Result<bool>;
}
```

Never let `sqlx::Error`, `PgRow` or a `Pg*` type appear in a trait signature.
A new port is one trait + one adapter + one mock, added in the same PR.

### 3.3 HTTP adapters

Handlers live in `<app>-api/src/routes/`, not in the slice crate. This keeps
the slice transport-free and lets the composition root own the route table.

```rust
#[utoipa::path(
    post, path = "/api/v1/widgets",
    request_body = CreateWidgetRequest,
    responses((status = 201, body = WidgetResponse), (status = 400, body = ErrorEnvelope)),
)]
pub async fn create_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Json(req): Json<CreateWidgetRequest>,
) -> Result<(StatusCode, Json<WidgetResponse>), ApiError> {
    authorize(&principal, Action::Create)?;
    let widget = state.widgets.create(&req.name, req.kind.into()).await?;
    Ok((StatusCode::CREATED, Json(widget.into())))
}
```

Rules:

- Extractor order is part of the contract: body-consuming extractors
  (`Json`) last; `State` and `Extension` first.
- A handler is thin: validate the wire shape, delegate, map the response.
  No business rules, no SQL.
- Every route carries a `#[utoipa::path]` annotation; `openapi.json` is
  generated from code and checked by `just gen-check`.
- Request bodies use `#[serde(deny_unknown_fields)]`.

---

## 4. Composition root

`crates/<app>-api/src/state.rs` and `router.rs` are the only files that know
every concrete adapter:

```rust
// state.rs
let pool = PgPoolOptions::new()
    .max_connections(cfg.database_max_connections)
    .connect(&cfg.database_url)
    .await?;

let widgets = Arc::new(WidgetService::new(Arc::new(PgWidgetRepository::new(pool.clone()))));
let state  = AppState::new(cfg, pool, widgets, /* … */);
```

```rust
// router.rs
let widget_routes = Router::new()
    .route("/api/v1/widgets", get(list_widgets).post(create_widget))
    .route("/api/v1/widgets/{id}", get(get_widget).put(update_widget).delete(delete_widget))
    .with_state(state.clone())
    .layer(middleware::from_fn_with_state(jwt_codec, require_auth));
```

Rules:

- One constructor per collaborator, called once at boot.
- `AppState` is `Clone` and `Send + Sync`; it holds `Arc<dyn Trait>` services,
  the pool, and `Arc<Config>` — nothing request-scoped.
- No `Arc<Mutex<T>>` outside the state/loom internals. If one appears, the
  data model is wrong: extract a port.
- Route tables are declared per surface (public / protected / admin) and
  merged; authorization is a layer plus a handler-side check, never only a
  path prefix.
- Migrations run at boot (`sqlx::migrate!`) **and** via the one-shot `migrate`
  subcommand; compose runs the one-shot service before api/worker.

---

## 5. Error model

One error enum in `<app>-core`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found")] NotFound,
    #[error("invalid input: {0}")] Invalid(String),
    #[error("unauthorized")] Unauthorized,
    #[error("unauthenticated: {0}")] Unauthenticated(String),
    #[error("forbidden: {0}")] Forbidden(String),
    #[error("conflict: {0}")] Conflict(String),
    #[error("rate limited")] RateLimited { retry_after_secs: u64 },
    #[error("upstream {service}: {detail}")] Upstream { service: &'static str, detail: String },
    #[error("internal: {0}")] Internal(String),
}
```

- `status_code()` lives with the enum; `ApiError` (`api/src/error.rs`) is the
  only `IntoResponse`, and the warden re-exports the same envelope types so
  the auth middleware and the API mapper cannot drift.
- Envelope:

```json
{ "error": { "code": "not_found", "title": "Resource not found",
             "detail": "…", "request_id": "req_…", "extra": {} } }
```

- 5xx detail is redacted to `"An internal error occurred."`; the real error
  goes to the log with the request id. 4xx detail is the error's own message.
- `429` carries `Retry-After` (and `extra.retry_after_secs`); `QuotaExceeded`
  carries `extra` counters.
- A caller never sees `sqlx::Error`, `reqwest::Error` or a panic message.

---

## 6. Persistence

### 6.1 Query style (actual, not aspirational)

The shipped adapters use **runtime** `sqlx::query`, `sqlx::query_as::<_, Row>`
and `sqlx::query_scalar`. Consequences:

- builds need no `DATABASE_URL` and no `.sqlx/` cache;
- SQL is **not** checked at compile time — the database is the check, so
  adapter-level integration tests (or a staged `migrate` run) are mandatory
  before a release that touches SQL;
- `docs/08-persistence-and-migrations.md` describes the compile-time
  `query_as!` + committed `.sqlx/` variant. Adopting it is a project decision:
  switch the adapter, add a `just sqlx-prepare` recipe, commit `.sqlx/`, and
  keep the build hermetic.

### 6.2 Data mapping

- A private `*Row` struct derives `FromRow`; `into_domain()` maps it. The DB
  shape never leaks into `<app>-core`.
- Enum columns are `TEXT + CHECK` in SQL and closed enums in Rust, with
  `as_str` / `from_str` pairs tested against the same table.
- Atomicity comes from single statements: CAS updates
  (`UPDATE … WHERE old_token = $1`), `ON CONFLICT`, and the outbox CTE. Add a
  transaction only when a write genuinely spans statements.

### 6.3 Migrations

- Paired files: `NNNN_<name>.up.sql` + `NNNN_<name>.down.sql`; four-digit
  sequence, never renumbered after merge.
- `just migrate-add <name>` scaffolds the next pair.
- The `.down.sql` reverses the `.up.sql` in dependency order — a deploy must
  be rollback-safe.
- Schema changes are expand/contract: add nullable, backfill, switch reads,
  drop later. Never a destructive change in the same release as the code that
  depends on it.
- Migrations are embedded (`sqlx::migrate!("../../migrations")`) and applied
  by the `migrate` subcommand and the compose `migrations` service; the API
  also applies them at boot for a single-node dev boot.

---

## 7. Mailer and the outbox

- `Mailer` is a trait in `<app>-mailer`; implementations are `NoopMailer`,
  `ProviderMailer`, `ConsoleSender`, `SmtpSender` (STARTTLS when offered).
- Selection is a plain function, not a new trait:
  `mailer_from_config(offline, endpoint)` — offline wins over everything;
  `Some(endpoint)` → provider; otherwise noop. `AppState::from_env` is the
  only call site.
- Requests enqueue an `email_jobs` row through the outbox port; the worker
  renders the localized template and sends. **Shipped deviation:** `enqueue`
  still executes on the pool, not on a caller-supplied transaction, so the
  job insert is not atomic with the domain write — a crash between them can
  lose the mail. Do not claim atomicity before the adapter takes the tx.
- Retries use `2^n` minutes with deterministic ±10% jitter; exhausted jobs
  become `failed` (dead-letter), never silently dropped.
- Templates are `en` + `es` data; an unknown placeholder key is an error.

**INV-T7 offline guard**: `<APP>_OFFLINE=true` selects `NoopMailer`
unconditionally — `<APP>_MAILER_ENDPOINT` is never read and no
`reqwest::Client` is constructed. OAuth clients refuse before any socket.
`config` validation rejects `OFFLINE` combined with any SMTP credential.

---

## 8. Realtime

- `<app>-loom/src/hub.rs` owns the in-process `Hub`: per-room subscriber
  map, per-subscriber **bounded** channel.
- `publish` uses `try_send` and **drops** for a slow consumer; it never blocks
  the publisher. A dropped event is a delivery loss, not a stall.
- Rooms: `user:<id>` for private streams; a broadcast room when a slice needs
  one.
- SSE: `GET /api/v1/widgets/events`, authenticated by the normal bearer/cookie
  path, `event: <type>` + `data: <json>`, `retry: 2000`.
- WebSocket: `GET /api/v1/ws`, upgrade first, then verify the raw `?token=`
  query; failure closes with code `4001` / `authentication_failed`; ping every
  30s, 10s timeout; client frames are discarded.
- The hub is single-process. Multi-replica fan-out needs a pub/sub adapter
  behind the same `EventBus` seam — a deliberate stub, not a hidden gap.

---

## 9. Workers

`<app>-loom/src/workers.rs` ships two loops:

| Loop | Job | Cadence |
|---|---|---|
| outbox drain | claim a batch (`FOR UPDATE SKIP LOCKED` CTE), render, send, mark sent/failed | `<APP>_OUTBOX_POLL` (2s), batch `<APP>_OUTBOX_BATCH` (10) |
| janitor | delete expired sessions and expired verification/reset rows | `<APP>_JANITOR_POLL` (1h) |

- The worker binary is `<app>-worker` (`loom/src/bin/worker.rs`); the API can
  also run the loops in-process. Running both simultaneously is safe because
  the outbox claim is atomic and the janitor deletes are idempotent.
- Every loop takes a `CancellationToken` / shutdown signal and exits on
  `ctx.Done()` equivalent; no detached task outlives the runtime.

---

## 10. Configuration

- One typed `Config` struct in `<app>-api/src/config.rs`, loaded once at boot
  from `<APP>_`-prefixed env (``__` nests), validated immediately.
- Defaults live in the struct's `default_*` functions, not scattered.
- The `Debug` impl redacts `jwt_secret`, `password_pepper`,
  `apikey_hmac_secret`, `smtp_pass`, `github_client_secret`.
- Required secrets: `<APP>_JWT_SECRET`, `<APP>_APIKEY_HMAC_SECRET` (≥32 bytes,
  placeholder blocklist rejects `.env.example` values);
  `<APP>_DATABASE_URL` non-empty.
- Durations are Go-style strings (`15m`, `720h`, compound `1h30m`) parsed at
  load; every TTL must be `> 0`.
- No env reads after startup; pass `Arc<Config>` through `AppState`.
- Production warnings (non-durable in-memory revocation store) are logged, not
  swallowed.

---

## 11. Observability

- Every request gets `req_<uuid v4>` (`request_id.rs`), echoed on
  `x-request-id` and attached to the `tracing` span.
- Logging: `RUST_LOG`, else `<APP>_LOG_LEVEL`; JSON in production, compact
  otherwise. Libraries log, never print.
- Metrics (feature `metrics`, runtime flag `<APP>_METRICS_ENABLED`):
  `<app>_http_requests_total` and `<app>_http_request_duration_seconds`, with
  labels limited to `method` and `status_class`. Never a user id or path in a
  label.
- `/metrics` and `/docs` (Swagger UI) are unauthenticated by default. Restrict
  them to an internal listener or an admin guard before production exposure.
- `/healthz` = process alive; `/readyz` = `SELECT 1` succeeds.

---

## 12. Deployment

- `Dockerfile`: `rust:1-bookworm` builder → distroless `cc-debian12:nonroot`
  runtime with `/app/server` and `/app/worker`, UID/GID 65532, no shell.
- `[profile.release]`: thin LTO, stripped symbols, `panic = "abort"`,
  `codegen-units = 1`, `opt-level = 3`.
- `docker-compose.yml`: postgres → one-shot migrations → api + worker.
- `Dockerfile.static`: same binaries against `distroless/static` for musl.
- Cargo.lock is committed post-rename (binary workspace); never commit it in
  the template.

---

## 13. When to add a crate

Add a crate when **all** of these hold:

1. The capability has its own business rules and its own tests.
2. It has at least one port or a distinct ownership boundary.
3. Two other crates would otherwise import the same code.

Do not add a crate for:

- a helper that one slice uses (put it in that slice);
- a "shared utils" grab bag (that is `core` or `ports`, chosen by whether it
  does I/O);
- a second orchestrator (only `loom` fans out across slices).

Checklist for a new crate: workspace `members` + `[workspace.dependencies]`
path pin, `[lints] workspace = true`, dependency edges that only point inward,
a mock for every new port, a paired migration if it owns storage, wiring in
`AppState`, routes in `router.rs`, and `just ci` green.

---

## 14. Checklist

- [ ] Every dependency edge is in the allowed table; no slice→slice import.
- [ ] `core` has no workspace dependency and no I/O.
- [ ] SQL exists only under `crates/<app>-ports/src/adapters/`.
- [ ] Consumers hold `Arc<dyn Trait>`; no `Pg*` type outside the composition
      root.
- [ ] Adapters map errors into `core::Error`; nothing returns `sqlx::Error`.
- [ ] New storage ships a paired migration; numbering is unchanged after
      merge.
- [ ] New HTTP route has a `#[utoipa::path]` annotation and `just gen-check`
      is green.
- [ ] New port ships a mock and a use-case test against it.
- [ ] `unsafe` is absent; suppressions use `#[expect(…, reason = "…")]`.
- [ ] Offline mode still boots: `mailer_from_config` selects Noop, no
      outbound socket is constructed.
- [ ] New background loop takes the shutdown signal and does not detach.
- [ ] `just ci` and `just audit` are green before the PR is opened.
