# Glossary

Every term this handbook leans on, one paragraph each. Cross-references in
brackets.

**Adapter** — A struct that *implements* a port against one mechanism: the
pgx-backed `postgres.go`, the SMTP `Sender` in `platform/email`. Adapters are
wired nowhere but the composition root. [01]

**Acceptance test** — A godog (Cucumber) scenario run against the real HTTP
mux wired with in-memory fakes (`test/acceptance/`). Proves behavior
end-to-end without a database. [11]

**Composition root** — `cmd/api/main.go` (HTTP) / `cmd/worker/main.go`
(outbox) / each slice's `Register(mux, Deps)`. The only code that knows
concrete types; nothing imports it. [01]

**Consumer-defined port** — An interface declared in the consuming slice's
`ports.go`, shaped to what the use case needs (1–2 methods), satisfied
implicitly by adapters. The template's keystone pattern. [01, 05]

**Depguard** — A golangci-lint rule engine; here the file-scoped law for
`domain.go` purity (no `net`, no `pgx`) and slice isolation (no feature→feature
imports). Configured in `.golangci.yml`, enforced red by `make ci`. [04]

**Deps** — The wire-in struct handed to a slice's `Register`: concrete
adapters + platform handles. The whole wiring contract between root and
slice. [01, 13]

**Domain** — `domain.go` in a slice: types, invariants, sentinel errors.
Imports stdlib + `internal/shared` only; zero I/O. [00, 04]

**Envelope** — The error body `{"detail": string, "code": STABLE_CODE}` on
every non-2xx. Codes are `internal/shared/apperr` constants; `httpx` maps
code→status in one place. [10]

**EventBus / Hub** — `platform/realtime`'s in-process pub/sub: `Publish`
(non-blocking drop) + `Subscribe(room) (<-chan, cancel)`. Redis pub/sub is
the documented multi-replica evolution behind the same interface. [09]

**Fake** — A hand-written in-memory implementation of a port in a `*_test.go`.
Preferred over mock frameworks everywhere; the standard unit-test double. [11]

**Functional options** — Go's constructor-with-defaults idiom:
`NewHub(WithBuffer(128))`. Used sparingly; three-plus options justify the
ceremony. [05]

**Goose** — Migration runner, single SQL files with `-- +goose Up` / `-- +goose Down` sections, invoked via
`go tool goose` / `make migrate-up`; `make migrate-add NAME=` stamps annotated files. Migrations never run at startup — the
application *checks* (readyz), it does not *apply*. [08]

**Hub room** — A string topic on the EventBus (`user:<id>`, `widgets`).
Subscription is identity-checked at subscribe time, never only at connect. [09]

**Identity** — `authx.Identity{ID, Email, Role}` carried in the request
context after `RequireAuth`. Use cases receive it as data, never as a
request. [06, 07]

**Kernel (shared)** — `internal/shared`: `apperr` (stable
codes) and `rolex` (the single role definition). May import stdlib only.
The "shared horizontal" that slices legitimately share. [03, 04]

**JWT (access token)** — HS256, claims `sub/email/role/exp/jti`, TTL 15m,
**never stored**. Minted by `authx.Minter`, parsed by `HS256Verifier`. [06]

**Middleware chain** — `func(http.Handler) http.Handler` wrappers composed by
`httpx.Chain` (request id, logging, recover, auth, role). Stdlib-only
composition. [05, 07]

**Outbox** — `email_jobs` + the worker: use cases enqueue (`Message{To,
TemplateKey, Vars}`); `RunWorker` claims rows with `SELECT ... FOR UPDATE
SKIP LOCKED`, sends via the configured `Sender` (Console|SMTP), retries with
jittered backoff, dead-letters after `max_retries`. [06, 08]

**Pepper** — Optional server-side secret pre-hashed with the password
(HMAC-SHA256) before bcrypt; empty = disabled. Lives in env config, so a DB
dump alone cannot verify passwords. [06]

**Port** — An interface at the hexagon's edge (driving: HTTP handler; driven:
stores, queues, clocks). Declared by the consumer, implemented by adapters,
faked in tests. [01]

**Readyz / Healthz** — `GET /healthz` (process) vs `GET /readyz` (`SELECT 1`
via the pool; 503 degraded on failure). The deploy signal — including schema
drift, since startup never migrates. [10]

**Refresh token (rotation)** — Opaque base64url 48B secret stored in
`sessions` (TTL 720h). Every refresh looks up + deactivates the old token and
mints a new pair (rotation); reset revokes *all* sessions, change-password
*others*. [06]

**Role hierarchy** — `ADMIN ⊃ MODERATOR ⊃ USER ⊃ GUEST`, defined once in
`internal/shared/rolex` with `AtLeast`; DB enum checks echo it. `RequireRole`
uses `AtLeast`; admins manage roles via the admin surface. [06]

**Slice** — `internal/features/<slugs>/`: one bounded feature owning its
domain, ports, use case, adapters and wiring contract in six files; slices
never import each other. The shipped six: `authsignup`, `authsession`,
`authpassword`, `adminusers`, `widgets` (example), `realtime`. [02]

**sqlc** — Generates committed typed query code (`internal/gen/db`, package
`db`) from each slice's `queries.sql` with `migrations/` as the schema
(`make generate`). Adapters are its only consumers. [08]

**Testcontainers** — Go library booting real Postgres 17 containers for the
`-tags=integration` suite (`make test-int`); per-suite lifecycle with
migrate-up/down exercised.
[08, 11]

**Vertical slicing** — Each feature owns its full stack in one directory
("tall but narrow"), over shared kernel/platform horizontals. The
organizing idea layered onto hexagonal rings. [02]

**Wire types** — `internal/gen/api` models generated from
`specs/openapi.yaml` (oapi-codegen, models only). Decode/encode happens only
in `http.go`; domain types travel inward. [03, 08]

**WS close 4001** — The WebSocket close code sent when the `?token=` JWT
fails verification: `4001 authentication_failed`, before upgrade. [09]