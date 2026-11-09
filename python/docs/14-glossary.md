# Glossary

Every term this handbook leans on, one paragraph each. Cross-references in
brackets.

**Adapter** — A class that _implements_ a port against one mechanism: the
SQLAlchemy-backed `postgres.py`, the SMTP `SmtpSender` in
`platform/email`, the `ConsoleSender`. Adapters are wired nowhere but the
composition root. [01]

**Acceptance test** — A pytest-bdd scenario (or a plain HTTP contract
check) run against the real FastAPI app wired with in-memory fakes
(`tests/acceptance/`), served by uvicorn on an ephemeral port. Proves
behavior end-to-end without a database. [11]

**asyncio** — Python's async runtime: the model behind `await`,
`asyncio.Queue`, `asyncio.Event` (the worker's stop signal) and
`pytest-asyncio` (`asyncio_mode = "auto"`). The single-threaded loop
removes the data-race class that other stacks need a detector for. [09]

**Composition root** — `src/app/bin/api.py` (HTTP) / `src/app/bin/worker.py`
(outbox + janitor) / each slice's `register(app, service)`. The only code
that knows concrete types; nothing imports it. [01]

**Consumer-defined port** — A `typing.Protocol` declared in the consuming
slice's `ports.py`, shaped to what the use case needs (1–2 methods),
satisfied structurally by adapters. The template's keystone pattern.
[01, 05]

**Deps** — The `Deps` dataclass in `src/app/bin/api.py`: the concrete
services, tokens and hub the app is built from. The whole wiring contract
between root and slices — and the seam the acceptance tier replaces with
fakes. [01, 13]

**Domain** — `domain.py` in a slice: frozen dataclasses, invariants,
stable-code errors. Imports the stdlib and `app.shared.*` only; zero I/O
(enforced by import-linter). [00, 04]

**Envelope** — The error body `{"detail": string, "code": STABLE_CODE}` on
every non-2xx. Codes are `Code` members in `app/shared/apperr.py`; only the
handlers in `app/platform/envelope.py` write the envelope. [10]

**Hub** — `app/platform/realtime`'s in-process pub/sub: `publish`
(non-blocking, drop-oldest) + `subscribe(room)` returning a `Subscription`
with a bounded `asyncio.Queue`. A real broker is the documented
multi-replica evolution behind the same surface. [09]

**Fake** — A hand-written in-memory implementation of a port in
`tests/fakes.py` (all of them sharing one `World` state object). Preferred
over mock frameworks everywhere; the standard unit-test double. [11]

**Hub room** — A string topic on the Hub (`user:<id>`). Subscription is
identity-checked at subscribe time, never only at connect. [09]

**Identity** — `authx.Identity {user_id, email, role}` returned by the
`require_auth` dependency and injected into the handler signature. Use cases
receive it as data, never as a request. [06, 07]

**Kernel (shared)** — `src/app/shared`: `apperr` (stable codes), `rolex`
(the single role definition), `authz` (self-or-subordinate), `clockx`
(injectable time), `idgen` (uuids + opaque tokens). Imports stdlib only.
The "shared horizontal" that slices legitimately share. [03, 04]

**OpenAPI contract** — `specs/openapi.yaml`, generated from the running app
by `scripts/export_openapi.py` and guarded by `make gen-check`. There is no
generated client in this template: the pydantic models are hand-written and
the contract is the artifact CI keeps honest. [03, 08]

**import-linter** — The tool that enforces the architecture contracts
declared in `pyproject.toml` (layered dependencies, I/O-free domains, slice
independence). `make lint` runs it next to ruff. [04]

**JWT (access token)** — HS256 (PyJWT), claims `sub/email/role/iss/iat/exp`,
TTL 15 min, **never stored**. Minted by `authx.Tokens.mint_access`,
verified by `verify_access` (issuer-tagged, HS256-only, requires
`sub/exp/iss/role`). [06]

**Middleware chain** — Starlette middleware (request id, security headers,
CORS) installed once in `bin/middleware.py`, plus FastAPI dependencies
(`require_auth`, `require_role`) declared per handler. Composed explicitly
at the mount point, never a global registry. [05, 07]

**Alembic** — Migration runner, one revision file per change with paired
`upgrade()` / `downgrade()`, invoked via `make migrate-up`;
`make migrate-add NAME=` stamps a new revision. Migrations never run at
startup — the application _checks_ (readyz), a one-shot compose service (or
the deploy step) _applies_. [08]

**Outbox** — `email_jobs` + the worker: use cases build an `EmailMessage`;
the slice adapter stages the row in the same transaction as the business
write; `outbox_loop` claims rows with `SELECT ... FOR UPDATE SKIP LOCKED`,
renders the localized template, sends via the configured `Sender`
(Console|SMTP), retries with exponential backoff, dead-letters once the
row's `max_retries` column is exhausted. [06, 08]

**Pepper** — Optional server-side secret concatenated to the password
before argon2id hashing; empty = disabled. Lives in env config, so a DB
dump alone cannot verify passwords. [06]

**Port** — A `Protocol` at the hexagon's edge (driving: HTTP handler; driven:
stores, queues, clocks). Declared by the consumer, implemented by adapters,
faked in tests. [01]

**pydantic v2** — The trust-boundary validator: request models in `http.py`
(`StringConstraints` for length, pattern, normalization) and `Settings` for
the environment. Never the domain — invariants are re-checked there. [10]

**pytest** — The test runner for every tier: unit, acceptance (with
`pytest-bdd`), and the kit contract evals. `asyncio_mode = "auto"` runs the
async tests without a decorator. [11]

**Readyz / Healthz** — `GET /healthz` (process; also the container probe)
vs `GET /readyz` (`SELECT 1` plus a read of `alembic_version`; 503
degraded envelope on failure). The deploy signal — including schema drift,
since startup never migrates. [10]

**Refresh token (rotation)** — Opaque 128-hex-character token stored in
`sessions` (TTL 720 h). Every refresh rotates in place under a
compare-and-swap guard on the old token (a replay can only win once); reset
revokes _all_ sessions, change-password _others_. [06]

**Role hierarchy** — `guest ⊂ user ⊂ moderator ⊂ admin`, defined once in
`app/shared/rolex` with `at_least`; the `users.role` column is TEXT with a
server default. `require_role` uses `at_least`; admins manage roles via
the admin surface. [06]

**Slice** — `src/app/features/<slug>/`: one bounded feature owning its
domain, ports, queries, adapter, use case, HTTP and wiring contract in the
same file set every time; slices never import each other. The shipped
seven: `authsignup`, `authsession`, `authpassword`, `authoauth`,
`adminusers`, `widgets` (example), `realtime`. [02]

**src layout** — The import package is `app`, living in `src/app/`, so
every import is absolute (`from app.features.widgets import queries`) and the
package is installed, not merely on the path. `mypy_path = "src"` and
`prepend_sys_path = src` (alembic) follow the same rule. [03, 13]

**Testcontainers** — The usual mechanism for a database-backed integration
lane (real Postgres 17 containers, migrations applied and reversed per
suite). The template ships **no** such lane and no testcontainers
dependency; [11] describes when to add one.

**uv** — The package manager: `uv sync` creates the environment, `uv run`
executes inside it. The template ships **no lockfile** (INV-T3): `uv.lock`
is git-ignored, so a fresh install resolves the newest in-range versions. [11, 13]

**Vertical slicing** — Each feature owns its full stack in one directory
("tall but narrow"), over shared kernel/platform horizontals. The
organizing idea layered onto hexagonal rings. [02]

**WS close 4001** — The WebSocket close code sent when the `?token=` JWT
fails verification: `4001 authentication_failed`. The handshake is completed
first and the socket is closed right after, so the client observes the close
code instead of an opaque handshake rejection. [09]

**mypy** — The static type checker (`strict = true`, `mypy src`); the
Python analog of `tsc --noEmit`. `ruff` is the linter and formatter
combined. [11]
