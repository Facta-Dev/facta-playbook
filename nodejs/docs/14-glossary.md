# Glossary

Every term this handbook leans on, one paragraph each. Cross-references in
brackets.

**Adapter** — A class that _implements_ a port against one mechanism: the
Kysely-backed `postgres.ts`, the SMTP `Sender` in `platform/email`. Adapters
are wired nowhere but the composition root. [01]

**Acceptance test** — A cucumber scenario run against the real HTTP app wired
with in-memory fakes (`test/acceptance/`), listening on an ephemeral port.
Proves behavior end-to-end without a database. [11]

**Composition root** — `src/bin/api.ts` (HTTP) / `src/bin/worker.ts` (outbox +
janitor) / each slice's `register(app, Deps)`. The only code that knows
concrete types; nothing imports it. [01]

**Consumer-defined port** — An interface declared in the consuming slice's
`ports.ts`, shaped to what the use case needs (1–2 methods), satisfied
structurally by adapters. The template's keystone pattern. [01, 05]

**Deps** — The wire-in interface handed to a slice's `register`: concrete
adapters + platform handles. The whole wiring contract between root and
slice. [01, 13]

**Domain** — `domain.ts` in a slice: types, invariants, stable-code errors.
Imports stdlib + `#shared/*` only; zero I/O. [00, 04]

**Envelope** — The error body `{"detail": string, "code": STABLE_CODE}` on
every non-2xx. Codes are `#shared/apperr` constants; only the root's error
handler writes the envelope. [10]

**EventBus / Hub** — `platform/realtime`'s in-process pub/sub: `publish`
(non-blocking drop) + `subscribe(room)` returning an `AsyncIterableIterator`
with `cancel()`. Redis pub/sub is the documented multi-replica evolution
behind the same interface. [09]

**Fake** — A hand-written in-memory implementation of a port in a
`*.test.ts`. Preferred over mock frameworks everywhere; the standard
unit-test double. [11]

**Hub room** — A string topic on the EventBus (`user:<id>`, `widgets`).
Subscription is identity-checked at subscribe time, never only at connect. [09]

**Identity** — `authx.Identity {userId, email, role, jti}` attached to the
request (`req.identity`) by `requireAuth`. Use cases receive it as data,
never as a request. [06, 07]

**Kernel (shared)** — `src/shared`: `apperr` (stable codes), `rolex` (the
single role definition), `authz` (self-or-subordinate), `clockx` (injectable
time), `idgen` (uuids + opaque tokens). Imports stdlib only. The "shared
horizontal" that slices legitimately share. [03, 04]

**Kysely** — The typed query layer: each slice owns a `queries.ts` declaring
its tables as a `Database` map and named type-checked query functions
(`make generate` plays no part — the types are hand-written, compile-checked,
the sqlc analog's documented delta). Adapters are its only consumers. [08]

**JWT (access token)** — HS256 (jose), claims `sub/email/role/exp/jti`, TTL
15m, **never stored**. Minted by `authx`'s Minter, verified by its Verifier
(issuer-tagged, HS256-only). [06]

**Middleware chain** — fastify `onRequest` hooks (request id, security
headers, CORS) + `preHandler` guards (`requireAuth`, `requireRole`), composed
explicitly at the mount point, never a global registry. [05, 07]

**node-pg-migrate** — Migration runner, one `.mjs` file per change with
paired `export const up/down` functions, invoked via `make migrate-up`;
`make migrate-add NAME=` stamps timestamped files. Migrations never run at
startup — the application _checks_ (readyz), a one-shot compose service (or
the deploy step) _applies_. [08]

**Outbox** — `email_jobs` + the worker: use cases enqueue (`Message{to,
templateKey, vars}`); `runEmailWorker` claims rows with `SELECT ... FOR
UPDATE SKIP LOCKED`, renders the localized template, sends via the configured
`Sender` (Console|SMTP), retries with jittered backoff, dead-letters after
`max_retries`. [06, 08]

**Pepper** — Optional server-side secret pre-hashed with the password
(HMAC-SHA256) before bcrypt (cost 10); empty = disabled. Lives in env config,
so a DB dump alone cannot verify passwords. [06]

**Port** — An interface at the hexagon's edge (driving: HTTP handler; driven:
stores, queues, clocks). Declared by the consumer, implemented by adapters,
faked in tests. [01]

**Readyz / Healthz** — `GET /healthz` (process; also the container probe) vs
`GET /readyz` (`SELECT 1` via the pool, 2s timeout; 503 degraded envelope on
failure). The deploy signal — including schema drift, since startup never
migrates. [10]

**Refresh token (rotation)** — Opaque base64url 64-char token stored in
`sessions` (TTL 720h). Every refresh rotates in place under a
compare-and-swap guard on the old token (a replay can only win once); reset
revokes _all_ sessions, change-password _others_. [06]

**Role hierarchy** — `ADMIN ⊃ MODERATOR ⊃ USER ⊃ GUEST`, defined once in
`#shared/rolex` with `atLeast`; the DB `CHECK` constraint echoes it.
`requireRole` uses `atLeast`; admins manage roles via the admin surface. [06]

**Slice** — `src/features/<slug>/`: one bounded feature owning its domain,
ports, queries, adapter, use case, HTTP and wiring contract in the same file
set every time; slices never import each other. The shipped seven:
`authsignup`, `authsession`, `authpassword`, `authoauth`, `adminusers`,
`widgets` (example), `realtime`. [02]

**Subpath imports** — `package.json` `"imports"`: `#shared/*`, `#platform/*`,
`#features/*`, `#gen/*`. The import-law surface the eslint rule groups
read: own slice relatively, cross-slice by subpath, `src/features/**` bans
`#features/*` outright. [04, 13]

**Testcontainers** — The usual mechanism for a database-backed integration
lane (real Postgres 17 containers, migrations applied and reversed per
suite). The template ships **no** such lane and no testcontainers
dependency; [11] describes when to add one.

**Vertical slicing** — Each feature owns its full stack in one directory
("tall but narrow"), over shared kernel/platform horizontals. The
organizing idea layered onto hexagonal rings. [02]

**Wire types** — `src/gen/api` models generated from `specs/openapi.yaml`
(openapi-typescript). Decode/encode happens only in `http.ts`; domain types
travel inward. [03, 08]

**WS close 4001** — The WebSocket close code sent when the `?token=` JWT
fails verification: `4001 authentication_failed`, after the handshake
completes (close codes are meaningless before it). [09]

**zod** — The trust-boundary validator: `httpx.decode` at the door (request
bodies) and env Load in `platform/config`. Never the domain — invariants re-
checked there. [10]
