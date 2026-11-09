# PLAYBOOK_BACKEND.md — backend architecture contract (Python)

The rules that keep `<app>` hexagonal and vertically sliced, written against
the code that actually ships: one uv project, Python 3.12, FastAPI 0.115 +
pydantic v2, SQLAlchemy 2.0 async + asyncpg, Alembic, an email outbox, an
in-process realtime hub.

Premise for actors, roles, tables and invariants: `MEMORY.md`. This file is
the deep contract; `AGENTS.md` is the short one. When they disagree with the
code, the code wins and this file gets fixed in the same PR.

---

## 1. The model in one screen

| Layer            | Directory                        | Knows about                    | Never imports                      |
| ---------------- | -------------------------------- | ------------------------------ | ---------------------------------- |
| Driving adapters | `src/app/features/<slice>/http.py` | FastAPI, pydantic, own use case | other slices, `postgres.py`      |
| Use case         | `usecase.py`                     | `domain.py`, `ports.py`        | FastAPI, SQLAlchemy, other slices  |
| Ports            | `ports.py`                       | `domain.py`                    | adapters                           |
| Domain           | `domain.py`                      | stdlib, `app.shared.*`         | everything else — **zero I/O**     |
| Driven adapters  | `postgres.py`, `src/app/platform/*` | SQLAlchemy/asyncpg or the outside world | other slices       |
| Wiring           | `module.py`                      | the slice's adapter + service  | —                                  |
| Composition root | `src/app/bin/*`                  | every slice and adapter        | — (nothing imports it)             |

Dependency direction is enforced by the three import-linter contracts in
`pyproject.toml`, not by the import system. Python tolerates import cycles
at runtime; the linter is the law.

The three non-negotiable properties:

1. **The domain is pure.** `domain.py` has frozen dataclasses, invariants and
   error codes, no I/O, no framework import.
2. **Ports are consumer-defined.** The use case writes the `typing.Protocol`
   it needs, narrow (1–3 methods), in its own `ports.py`. The adapter
   satisfies it structurally.
3. **Only the root wires concretions.** A slice's `register(app, service)`
   receives a constructed service and returns nothing else.
   `tests/conftest.py` is the second wiring and the proof.

---

## 2. Slice roles and the allowed edges

```text
src/app/bin/api.py ──► every slice register() ──► Deps (concrete adapters)
      │                                              │
      └──► src/app/platform/* ◄── slices may import platform
                                     │
                              src/app/shared/* (kernel)
```

Rules, exact:

| From                                                | May import                                                                |
| --------------------------------------------------- | ------------------------------------------------------------------------- |
| `src/app/bin/**`                                    | anything                                                                  |
| `http.py`, `postgres.py`, `module.py` of a slice     | own slice, `app.platform.*`, `app.shared.*`                                |
| `usecase.py`                                        | `domain.py`, `ports.py`, `app.platform.*`, `app.shared.*`                  |
| `ports.py`                                          | `domain.py` (+ platform types where the port is platform-shaped)          |
| `domain.py`                                         | stdlib, `app.shared.*`                                                    |
| `src/app/platform/*`                                | `app.shared.*`, stdlib, SQLAlchemy — never `app.features.*`               |
| `src/app/shared/*`                                  | stdlib only                                                               |
| anything                                            | `src/app/bin/**` — forbidden                                              |
| any slice                                           | another slice — forbidden                                                 |

Imports are absolute everywhere, own-slice included
(`from app.features.widgets.domain import Widget`). Cross-layer imports use
the `app` import package: `app.shared.*`, `app.platform.*`,
`app.features.*`.

Need another slice's behavior? Three lawful moves, in order:

1. **Promote down** to `app.shared` (pure rule) or `app.platform`
   (infrastructure with no domain).
2. **Declare a port** in the consumer and satisfy it from the root with the
   other slice's adapter — the consumer never sees the producer's module.
3. **Duplicate the small capability** in the consumer when the two copies are
   genuinely independent (the signup and password slices both enqueue mail;
   neither imports the other).

Never weaken an import-linter contract. A lint failure here is an
architecture decision made in private; make it in the open instead.

---

## 3. The slice shape

Every slice has the same seven files. "Where does this live?" is answered by
file kind, never by search.

```text
src/app/features/<slice>/
├── domain.py        # pure types, invariants, error codes
├── ports.py         # the Protocols the use case declares
├── usecase.py       # orchestration: rules first, I/O last
├── queries.py       # parameterized text() SQL — the only SQL home
├── postgres.py      # SQLAlchemy adapter over queries.py
├── http.py          # FastAPI routes, pydantic models, the envelope
└── module.py        # register(app, service)
```

Unit tests live in `tests/unit/` (`test_services.py`) with in-memory fakes
from `tests/fakes.py`; the behavioral contract lives in
`specs/features/*.feature`.

The `realtime` slice is the one exception: it ships `module.py` only (the
WebSocket endpoint), because the hub itself is platform infrastructure.

### 3.1 Domain

```python
# src/app/features/widgets/domain.py
from dataclasses import dataclass
from datetime import datetime
from typing import Literal

WidgetKind = Literal["sprocket", "cog", "flange"]
WIDGET_KINDS: tuple[str, ...] = ("sprocket", "cog", "flange")


@dataclass(frozen=True)
class Widget:
    """One owned row."""

    id: str
    name: str
    kind: WidgetKind
    owner_user_id: str
    created_at: datetime
```

- No import from FastAPI/SQLAlchemy/asyncpg/another slice. The import-linter
  contract "Domain and kernel are I/O-free" names every slice's `domain.py`
  and `app.shared` as source modules and forbids `fastapi`, `sqlalchemy`,
  `asyncpg`, `alembic`, `httpx`, `uvicorn`.
- Invalid states are unrepresentable: `Literal` for closed sets, `StrEnum`
  for codes and roles, `@dataclass(frozen=True)` for data a callee must not
  mutate.
- Errors are values: `Code` is a `StrEnum` in `src/app/shared/apperr.py`.
  Raise `E(code, detail, status)`; never raise a bare string and never
  invent a code inline in `http.py`.

### 3.2 Ports

```python
# src/app/features/widgets/ports.py
from typing import Any, Protocol

from app.features.widgets.domain import Widget, WidgetKind


class WidgetStore(Protocol):
    """Persistence, always filtered by owner."""

    async def insert(self, widget: Widget) -> None:
        """Insert one row."""
        ...

    async def list(self, owner_id: str, limit: int, offset: int) -> list[Widget]:
        """One page of the owner's widgets."""
        ...

    async def get(self, widget_id: str, owner_id: str) -> Widget | None:
        """Owner-scoped fetch."""
        ...


class Publisher(Protocol):
    """The realtime surface the use case needs."""

    async def publish(self, room: str, event: dict[str, Any]) -> None:
        """Fan out one event to a room."""
        ...
```

- Keep it to the methods this use case calls. A five-method port with one
  production implementation is usually two features sharing a directory.
- Methods return domain values or raise an `AppError`; adapters translate
  driver failures inward (`IntegrityError` → `AUTH_EMAIL_TAKEN` is the
  reference).
- No `AsyncSession`, no `AsyncEngine`, no `Request` in a port signature. The
  port is the seam the tests fake.

### 3.3 Use case

```python
# src/app/features/widgets/usecase.py
from typing import Any, cast

from app.features.widgets.domain import WIDGET_KINDS, Widget, WidgetKind
from app.features.widgets.ports import Publisher, WidgetStore
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource


class WidgetService:
    """Reads and writes are filtered by the authenticated owner."""

    def __init__(
        self,
        store: WidgetStore,
        publisher: Publisher | None,
        ids: IdSource,
        clock: Clock,
    ) -> None:
        self._store = store
        self._publisher = publisher
        self._ids = ids
        self._clock = clock

    async def create(self, owner_id: str, name: str, kind: str) -> Widget:
        """Insert and announce."""
        if kind not in WIDGET_KINDS:
            msg = "unknown kind"
            raise E(Code.VALIDATION_FAILED, msg, 422)
        widget = Widget(
            id=self._ids.new_id(),
            name=name,
            kind=cast(WidgetKind, kind),
            owner_user_id=owner_id,
            created_at=self._clock.now(),
        )
        await self._store.insert(widget)
        await self._publish(owner_id, {"type": "widget.created", "widget": name})
        return widget
```

- Validate the domain's rules here, once. Edge validation (shape, types,
  lengths) lives in `http.py` with pydantic; the use case validates meaning.
- I/O last: compute and check first, then call the port.
- No `datetime.now()`, no `uuid4()` — take a `Clock` and an `IdSource` when
  time or identity matters to the rule. `FixedClock` and `SequenceIds` exist
  exactly so tests never sleep and never read the wall clock.
- `asyncio` cancellation enters here when the work is long or batched:
  accept the `asyncio.Event` or `CancelledError` the caller owns and honor it
  at loop yields.

### 3.4 HTTP adapter

```python
# src/app/features/widgets/http.py
from fastapi import APIRouter, status
from pydantic import BaseModel, StringConstraints

from app.features.widgets.usecase import WidgetService
from app.platform.authx import IdentityDep


class WidgetRequest(BaseModel):
    name: Annotated[str, StringConstraints(min_length=1, max_length=120)]
    kind: Annotated[str, StringConstraints(min_length=1, max_length=32)]


def build_router(service: WidgetService) -> APIRouter:
    """Every route is authenticated and owner-scoped."""
    router = APIRouter(tags=["widgets"])

    @router.post("/widgets", status_code=status.HTTP_201_CREATED)
    async def create_widget(body: WidgetRequest, identity: IdentityDep) -> WidgetResponse:
        return _model(await service.create(identity.user_id, body.name, body.kind))

    return router
```

- The pydantic model is the only body decoder; a validation failure becomes
  a 422 `VALIDATION_FAILED` envelope through the root's handler.
- Identity comes from `IdentityDep` (`Depends(require_auth)`), which
  verifies the bearer token and returns an `Identity`. Never parse the JWT in
  a handler.
- `Depends` is used for auth guards only (`require_auth`, `require_role`);
  everything else is constructor injection.
- One handler = one job. Mapping loops and multi-step orchestration belong in
  the use case.

### 3.5 Queries and the Postgres adapter

`queries.py` is the only place SQL exists. SQLAlchemy Core `text()` builds
it; column names stay snake_case exactly as the migrations declare them.

```python
# src/app/features/widgets/queries.py
from sqlalchemy import text

GET = text(
    """
    SELECT id, name, kind, owner_user_id, created_at
    FROM widgets
    WHERE id = :widget_id AND owner_user_id = :owner_id
    """
)
```

```python
# src/app/features/widgets/postgres.py
async def get(self, widget_id: str, owner_id: str) -> Widget | None:
    async with self._db.session() as session:
        result = await session.execute(queries.GET, {"widget_id": widget_id, "owner_id": owner_id})
        row = result.mappings().first()
    return None if row is None else _widget(row)
```

- Export module-level `text()` constants, one per statement, with named
  parameters (`:widget_id`). No query builder leaks out, and there are no ORM
  entities in this template.
- `postgres.py` owns the mapping to domain values (`_widget`) and translates
  driver errors. Nothing else may import `queries.py`.
- Two statements that must agree belong in one session inside the adapter,
  not in the use case. The port says so (`register(user, code, message,
  now)`) instead of exposing a session handle.

### 3.6 Module

```python
# src/app/features/widgets/module.py
from fastapi import FastAPI

from app.features.widgets.http import build_router
from app.features.widgets.usecase import WidgetService


def register(app: FastAPI, service: WidgetService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
```

`register(app, service)` is the whole seam. It may build the router; it may
not open an engine, read the environment, or import another slice.

---

## 4. Composition root

`src/app/bin/api.py` is three functions:

1. `wire(settings)` — build the real adapters and services: one
   `Database.connect(settings)` engine, a `SystemClock`, a `SystemIds`, the
   `Hub`, a `Passwords` hasher, a `Tokens` minter/verifier, one service per
   slice, and a `Deps` dataclass holding them.
2. `build_app(deps)` — assemble the FastAPI app: the `lifespan` closes the
   engine on shutdown, `app.state.tokens` and `app.state.hub` carry the
   process-wide handles, `install_error_handlers(app)` and
   `install_middleware(app, settings)` install the envelope and the
   request-id/security-header/CORS chain, every slice's `register` is called
   with its service, and `/healthz` and `/readyz` are registered outside the
   API prefix.
3. `create_app()` / `main()` — load `Settings()`, `configure_logging`, and
   serve with uvicorn.

`src/app/bin/worker.py` is the second entrypoint on the same image: the same
`Database`, a `select_sender(settings)` sender, one `asyncio.Event` wired to
SIGINT and SIGTERM, the outbox loop **and** the janitor loop, then
`database.close()`.

Both roots validate config at boot. Neither applies migrations. The outbox
worker is a **separate process**: the API never drains the outbox.

Rules:

- Adapters are constructed at the root, never imported by a use case.
- One engine per process. `Database.session()` hands out short-lived
  sessions; the caller owns commit/rollback.
- Shutdown is explicit: the lifespan disposes the engine, the worker cancels
  its tasks and closes the engine. A task still running after SIGTERM is a
  defect.
- The root may know every slice; nothing else may know two.

---

## 5. Error model

Every non-2xx response has exactly one shape:

```json
{ "detail": "widget not found", "code": "WIDGET_NOT_FOUND" }
```

`AppError` (`src/app/shared/apperr.py`) carries `code`, `detail` and
`status`. Construction:

- `E(code, detail, status)` — the only constructor.
- `raise E(...) from exc` — wrap a driver failure with its cause; never
  match on message text.
- The exception handlers installed by `install_error_handlers(app)` are the
  only coercion point: `AppError` → its status, `RequestValidationError` →
  422 `VALIDATION_FAILED` with a fixed detail, `StarletteHTTPException` →
  its status (`NOT_FOUND` for 404, `INTERNAL` otherwise), anything else → a
  logged 500 `INTERNAL`.

Rules:

- Stable codes are the `Code` enum in `src/app/shared/apperr.py`. Add a code
  there before using it; never type a string literal in a handler.
- Map the HTTP status once, in the place that knows the failure. `401` for an
  invalid credential, `403` for an authenticated-but-forbidden action, `404`
  for a row the caller may not see (never leak existence with `403`), `409`
  for uniqueness, `422` for shape validation.
- 5xx detail is generic. Do not echo driver messages, SQL, or stack data.
- The envelope is written in exactly one place: `error_body()` in
  `src/app/platform/envelope.py`. A handler raises; the root renders.
- Adapters translate exactly once. An `IntegrityError` mapped in two places
  will drift.
- `except` never swallows: no `except Exception: pass`. Log once, where the
  context lives, and re-raise or translate.

---

## 6. Persistence

- **Postgres 17**, one async engine at the root
  (`create_async_engine(..., pool_pre_ping=True)`, `expire_on_commit=False`).
- **SQL lives in each slice's `queries.py`** and is consumed only by that
  slice's `postgres.py`. No raw SQL in `usecase.py` or `http.py`; no query
  builder in a port signature. This rule is review-enforced — the
  import-linter contracts keep `sqlalchemy` out of `domain.py` and
  `app.shared`, not out of a slice's adapter.
- **Migrations live in `migrations/versions/`** as Alembic revisions created
  by `make migrate-add NAME=…`. Every file carries `upgrade()` and
  `downgrade()`; an empty `downgrade` is a review block.
- **Migrations never run at startup.** Apply with `make migrate-up`, or the
  compose one-shot `migrations` service (`alembic upgrade head`). Replicas
  would race.
- **The migration environment is separate from the app environment.**
  `migrations/env.py` reads `<APP>_DATABASE_URL` from the environment and
  drives its own async engine with a `NullPool`; it does not import
  `Settings`, so the two cannot drift by accident.
- **Naming**: snake_case tables and columns, plural tables, `id` UUID
  columns, `created_at timestamptz` defaulted by the DB, composite indexes
  for the hot lookups (`sessions_user_active`, `email_jobs_due`).
- **No `SELECT *` into a port type**: select the columns the port promises,
  map explicitly. A schema addition should not silently widen a response.
- **Pagination**: lists take `page` (≥1) and `page_size` (1–100, default 20)
  through `pagination()` in `src/app/platform/envelope.py`. An unbounded
  fetch on a user-sized table is a defect.
- **Ownership predicates are in the SQL**, not bolted on after the read:
  `get(widget_id, owner_id)` filters both. An unowned row then reads as
  `None` → 404.
- **Destructive changes** (drop/rename column, backfill) get their own
  migration with a data plan in the file header. Never edit a migration that
  has shipped; add the next revision.

The shipped schema: `0001_init` creates `users`, `sessions`,
`email_verifications`, `password_resets`, `email_jobs`, `widgets` and
`audit_logs`; `0002_profiles` adds `users.profile_picture` plus the
`sessions_expiry` and `email_jobs_status` indexes.

---

## 7. Mail and the outbox

Slices never send mail. They enqueue intent; a worker renders and delivers.

```python
# src/app/platform/email/outbox.py
@dataclass(frozen=True)
class EmailMessage:
    """What a use case hands to the outbox."""

    to: str
    template_key: str
    variables: dict[str, Any]
    locale: str = "en"
```

- The slice declares a store port method that carries the message
  (`register(user, code, message, now)`); the adapter stages the outbox row
  on its own session.
- `enqueue_email(session, message, now)` only **stages** the `email_jobs`
  INSERT on the caller's session and never commits. The business write and
  the job therefore commit together: a crash cannot separate them.
- The worker claims due rows with `FOR UPDATE SKIP LOCKED` — safe with many
  replicas — flips them to `sending`, and recovers rows stranded there past a
  five-minute lease (at-least-once).
- Delivery success → `sent`. Failure → `retry_count + 1` with an exponential
  `2^n`-minute backoff; at the row's `max_retries` (DB default 3) the row
  dead-letters as `failed` and `last_error` keeps the cause.
- Templates are keyed (`verify_email`, `reset_password`) and localized by
  locale (`en`, `es`); an unknown locale falls back to `en`, an unknown
  template key raises `ValueError` — which fails that delivery visibly
  instead of silently dropping the mail.
- The janitor purges expired sessions and stale one-shot codes
  (`email_verifications`, `password_resets`) on `<APP>_JANITOR_POLL_SECONDS`
  (default 1 h). It does not purge `email_jobs`; add a retention purge
  deliberately.

---

## 8. Realtime

One process-wide `Hub` (`src/app/platform/realtime/hub.py`), created in
`wire()` and injected as each slice's `Publisher` port.

```python
class Hub:
    """Rooms are process-local; a second replica needs a real broker."""

    async def subscribe(self, room: str) -> Subscription: ...

    async def unsubscribe(self, subscription: Subscription) -> None: ...

    async def publish(self, room: str, event: dict[str, Any]) -> None: ...
```

- `publish` never blocks the caller: when a subscriber's queue (bounded at
  32) is full, the **oldest** event is dropped to make room. Slow consumers
  lose data; they do not apply backpressure to a request.
- `unsubscribe` is idempotent and removes the room when its last listener
  leaves, so a forgotten unsubscribe leaks a queue, not a room.
- Room vocabulary is shared by WS and SSE: `user:<id>`. A slice publishes to
  that room through its port; it never imports the realtime slice.
- **WebSocket**: upgrade at `/api/v1/ws?token=<access-jwt>`. The browser
  cannot send an `Authorization` header on the handshake, so the token is
  verified _after_ the upgrade; failure completes the handshake and then
  closes with code `4001` and reason `authentication_failed`. The server
  sends `{"type": "hello", "user_id": ...}` right after accepting, answers a
  client frame of `"ping"` with `{"type": "pong"}`, and pushes an
  application-level `{"type": "ping"}` every 30 s.
- **SSE**: `GET /api/v1/widgets/events`, `text/event-stream`, one
  `data: {"type": ...}` line per frame with compact JSON — no `event:` name
  and no `retry:` line — plus a `: keep-alive` comment every 15 s. The
  handler subscribes to the user's room and unsubscribes in a `finally:`
  block; the client disconnect ends the stream.
- A stream delivers only what the same identity could read over REST. A
  stream is not an authorization bypass.
- **Single-process by design.** Fan-out stops at the process boundary. The
  documented evolution is a Redis pub/sub adapter behind the `Hub`; the
  contract above is what must not change.

---

## 9. Workers and background work

Two backgrounds exist: the email outbox and the janitor. Both live in
`src/app/platform/email/worker.py`.

Rules for any new loop:

- It has an owner: one `asyncio.Event` per process, set by the SIGINT and
  SIGTERM handlers. No task that outlives its owner.
- It is idempotent or claims rows with `SKIP LOCKED`. Multiple replicas must
  be safe; at-least-once is the delivery guarantee.
- It never throws out of the loop. A failed tick logs and continues; a crash
  loop needs an alert, not silent retries.
- It honors the stop event during the wait, not after it
  (`asyncio.wait_for(stop.wait(), timeout=...)`).
- It uses its own session for janitor-style bulk deletes; feature writes go
  through a slice's adapter.
- Structured logging only, through `logging` with the JSON formatter. No
  `print` outside a script's own CLI output.

Adding a queue? The outbox table is the pattern: durable row, lease,
backoff, dead-letter. Do not add Celery/Redis for a second queue without
writing the swap decision down — the adapter is replaceable, the contract is
not.

---

## 10. Configuration

`src/app/platform/config.py`: one pydantic-settings `Settings` over
`<APP>_`-prefixed env, parsed once at boot. Unknown variables are ignored;
list-valued settings are JSON arrays.

- Every key is prefixed: `<APP>_DATABASE_URL`, `_JWT_SECRET`, `_JWT_ISSUER`,
  `_ACCESS_TTL_MINUTES`, `_REFRESH_TTL_HOURS`, `_PASSWORD_PEPPER`,
  `_OFFLINE`, `_CORS_ORIGINS`, `_SMTP_HOST`, `_SMTP_PORT`, `_SMTP_USER`,
  `_SMTP_PASSWORD`, `_MAIL_FROM`, `_OUTBOX_POLL_SECONDS`, `_OUTBOX_BATCH`,
  `_JANITOR_POLL_SECONDS`, `_VERIFY_CODE_TTL_HOURS`, `_VERIFY_MAX_ATTEMPTS`,
  `_RESET_TTL_MINUTES`, `_RESET_RATE_LIMIT`, `_GOOGLE_CLIENT_ID`,
  `_GITHUB_CLIENT_ID`, `_GITHUB_CLIENT_SECRET`, `_GITHUB_CALLBACK_URL`,
  `_FRONTEND_SUCCESS`, `_ALLOWED_ADMIN_EMAILS`, `_LOG_LEVEL`.
- Durations are plain numbers whose unit is in the name (`_SECONDS`,
  `_MINUTES`, `_HOURS`); booleans are `true`/`false`; lists are JSON arrays.
- **Fail closed**: `DATABASE_URL` must use the `postgresql+asyncpg://`
  driver; `JWT_SECRET` must be at least 32 characters with 8 distinct ones
  and must not be a known placeholder.
- **Offline guard (INV-T7)**: `<APP>_OFFLINE=true` forbids SMTP credentials
  at boot, forces the console sender, and makes the OAuth flows refuse
  (`AUTH_OFFLINE`, 503) instead of opening sockets. A fresh `.env` with
  dummy values plus `OFFLINE=true` must boot a working stack.
- Config is read in the roots and passed down. A slice that instantiates
  `Settings()` or reads `os.environ` is a defect; add a constructor
  argument instead.

---

## 11. Observability

- **Logs**: the stdlib `logging` module with a JSON formatter — one object
  per line on stdout (`ts`, `level`, `logger`, `msg`, `exc`). The request id
  travels in a `ContextVar`, is copied onto every record by a logging
  filter, and is echoed on every response as `x-request-id`. Never log
  passwords, tokens, authorization headers, or the WS query string.
- **Health**: `/healthz` is process liveness (no I/O) and backs the
  container probe. `/readyz` runs `SELECT 1` **and** reads
  `alembic_version`; a database that was never migrated is not ready, and
  the failure answers a 503 envelope.
- **Envelope**: the stable `code` is the machine field; `detail` is for
  humans. Counters and alerts should key on `code`, not on message text.
- **Metrics/tracing are not shipped.** OpenTelemetry is a deliberate cut.
  The seams are the middleware chain and the platform adapters; add
  instrumentation there, not in every slice.
- **Log levels**: `debug` in development, `info` in production. A per-request
  log line at `info` on a hot path is a cost decision.

---

## 12. Deployment

- One Docker image (`python:3.12-slim`, non-root `app`, `HEALTHCHECK` via
  `urllib.request` on `/healthz` — the runtime image carries no curl), two
  entrypoints: `python -m app.bin.api` and `python -m app.bin.worker`.
- `docker compose up --build` boots Postgres 17, a one-shot `migrations`
  service, the API, and the worker. The API never migrates itself.
- The outbox runs only in the worker. A deployment with no worker quietly
  stops draining `email_jobs` and stops purging expired rows; keep the
  second service.
- Scale-out is safe for outbox claims and janitor purges; realtime fan-out
  is not (see §8).
- No lockfile ships with the template (INV-T3). Decide deliberately after
  instantiation: commit `uv.lock` and remove the ignore entry if you need
  byte-reproducible installs.

---

## 13. When to add a slice

Add a slice when a capability has its own rules, its own tables or its own
route group. Do not add one for a single handler over an existing slice.

The recipe is `docs/12-playbooks.md` §1; the proof it was done right:

- `make ci` green and `make accept` green.
- `git status` shows edits only under `src/app/features/<slug>/`, the root
  wiring (`src/app/bin/api.py`), a new migration, and optionally a spec —
  nothing else.
- Deleting the slice directory plus its root registration leaves a compiling
  project (apart from the example's spec).

If adding a slice forced edits in three unrelated directories, the boundary
is wrong; fix the shape before landing it.

---

## 14. Checklist

- [ ] `domain.py` imports only stdlib and `app.shared.*`.
- [ ] Ports are consumer-defined `Protocol`s, ≤3 methods, no driver types.
- [ ] SQL appears only in `queries.py`; the adapter is the only caller.
- [ ] Errors are `AppError` values with codes from `src/app/shared/apperr.py`.
- [ ] The handler raises; the envelope renders once, in
      `src/app/platform/envelope.py`.
- [ ] Lists are paginated with a hard maximum.
- [ ] Ownership is a predicate in the SQL, not a post-read check.
- [ ] A new table has a paired upgrade/downgrade revision and no startup
      migration.
- [ ] Mail goes through the outbox row on the caller's session, never an
      inline sender.
- [ ] Realtime publishes to a room; slices never import each other.
- [ ] New loops take the process stop event and are replica-safe.
- [ ] New config is `<APP>_`-prefixed, pydantic-validated, fail-closed,
      documented in `.env.example`.
- [ ] Nothing new imports `src/app/bin/**`; nothing new imports another
      slice.

---

## 15. Known gaps in the shipped code

Not features. Fix deliberately or leave documented.

- No integration tier: adapter SQL and migration upgrade/downgrade are
  unproven by CI (`docs/11-testing-and-ci.md` §3).
- `authoauth` and `realtime` have no unit tests; the pytest-bdd acceptance
  suite covers their behavior.
- Realtime is single-process (Redis pub/sub is the evolution).
- There is no login lockout: `users.retry_count` ships as a column but
  nothing increments it, and an unknown email is checked against a fixed
  `_DUMMY_HASH` for timing parity rather than against a counter.
- No coverage threshold, mutation testing, secret scan, SBOM or image scan.
- `POST /api/v1/auth/upgrade-to-admin` is unauthenticated and gated only by
  `ALLOWED_ADMIN_EMAILS`; remove it after bootstrap.
- No `src/gen/` client: the only committed generated artifact is
  `specs/openapi.yaml`, exported from the app by `scripts/export_openapi.py`.
  The pydantic request/response models are hand-written.
- The stack deltas vs beautifulpython's published digest (SQLAlchemy Core
  `text()` and not ORM entities, DB outbox and not Celery/Redis, constructor
  injection, stdlib `logging` and not a logging library) are intentional.
  Swap an adapter; do not change the contract.
