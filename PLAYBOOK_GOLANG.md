# PLAYBOOK_GOLANG.md — Backend architecture playbook (Go)

Architecture contract for a Go backend built from this kit: **hexagonal
architecture (ports & adapters) organized by vertical slices**. The rules are
expressed as folders, dependency directions and contracts; the bundled
skeleton (`cmd/`, `internal/`, `migrations/`, `.claude/`) is the reference
implementation. Companion of `PLAYBOOK_URLS.md` (routes and authorization) and
`PLAYBOOK_TESTS.md` (the proof).

The rules that carry the whole file:

1. **A slice owns a business capability end to end** (routes → use cases →
   domain → tables). Code is grouped by capability, never by technical type.
2. **Dependencies point inward**: adapters → application → domain. The domain
   imports nothing but the standard library and `internal/shared`.
3. **Slices talk only through published contracts**, never through each
   other's tables, sqlc models or internals. `internal/features/**` never
   imports another slice.
4. **Postgres is the ledger; the outbox and the in-process hub are the
   couriers.** Correctness comes from idempotency and database constraints,
   not from a queue.
5. **Extract a slice into a service only when measurements demand it**, and
   the hexagon makes that a swap of adapters, not a rewrite.

## 1. Vertical slicing

### 1.1 What is a slice

A slice is a cohesive business capability with its own language, data and
lifecycle. Every slice lives at `internal/features/<name>/`.

The skeleton ships seven slices:

| Slice | Responsibility |
|---|---|
| `authsignup` | Signup, email verification code, resend |
| `authpassword` | Forgot / validate / reset / change password |
| `authsession` | Login, refresh rotation, logout, me/update-me, session listing and revocation |
| `authoauth` | Google One-Tap, GitHub OAuth login/callback/connect |
| `adminusers` | Admin user list, detail, partial update, audit trail, upgrade-to-admin |
| `widgets` | Reference owner-scoped CRUD slice; publishes lifecycle events (replace with your domain) |
| `realtime` | WebSocket endpoint: authenticate via `?token=`, stream hub events |

Slice naming follows the URL module of `PLAYBOOK_URLS.md`: the folder, the
route prefix and the table prefix align (`adminusers` → `/api/v1/admin/users`
→ `users`). A new product slice (`orders`, `billing`, …) follows the same
rule and starts from the `widgets` example.

### 1.2 Slice boundaries — how to decide

- One slice = one aggregate root family that changes together.
- If two things must be consistent in the same transaction, they belong to the
  same slice.
- If a feature needs data from another slice, it declares a port and the
  composition root supplies an adapter (section 7); it never imports the other
  slice and never joins its tables. `slice-isolation` enforces the import half.
- Splitting too early is worse than splitting late. Start with one slice and
  split when the capabilities start changing independently.
- The shared kernel (`internal/shared/`) exists but stays tiny (section 2.3).

## 2. Repository layout

One flat folder per slice with a fixed six-file (plus `queries.sql`) shape:

```text
<app>/
  cmd/
    api/              # composition root: env → pool → adapters → Register → serve
    worker/           # email outbox (FOR UPDATE SKIP LOCKED) + janitor loops
    migrate/          # goose up/down; the API never migrates itself
  internal/
    features/
      <slice>/
        domain.go     # pure types, invariants, typed errors — stdlib + shared only
        ports.go      # the interfaces the consumer declares
        usecase.go    # plain funcs / small structs over ports
        http.go       # ServeMux adapters + request/response encoding
        postgres.go   # adapter over internal/gen/db (sqlc)
        module.go     # Register(mux, deps) — the only exported wiring function
        queries.sql   # sqlc input for this slice's queries
        *_test.go     # colocated tests (fakes in the package)
    platform/         # authx, config, email, httpx, oauthx, pgx, realtime
    shared/           # apperr, authz, clockx, idgen, rolex
    gen/              # committed generated code: db (sqlc), api (oapi)
  migrations/         # goose SQL, shared file sequence, prefixed tables per slice
  specs/              # openapi.yaml + godog .feature files
  test/
    acceptance/       # godog over the real mux with fakes
    integration/      # testcontainers Postgres (build tag `integration`)
    testutil/
  evals/              # separate Go module: contract evals
  .claude/            # agents, skills, commands, schemas, settings
  scripts/            # rename.sh, verify-template.sh, lint-*.sh, hooks/
```

A slice may add files when the capability demands it. The six core files stay;
extras are named for what they do, never `helpers.go` or `utils.go`.

### 2.1 Dependency rules (enforced, not suggested)

| From | May import | Must not import |
|---|---|---|
| `domain.go` | stdlib, `internal/shared` | `net/http`, `database/sql`, `pgx`, `internal/gen`, `internal/platform`, other slices |
| `ports.go` / `usecase.go` | own `domain`, `internal/shared` | transport (`net/http`), `pgx`, `database/sql`, `internal/gen`, `internal/platform` |
| `http.go` | own `usecase`/`domain`/`ports`, `internal/platform/httpx`, `internal/platform/authx` | another slice |
| `postgres.go` | own `ports`, `internal/gen/db`, `internal/platform/pgx` | another slice |
| `internal/platform/**` | stdlib, `internal/shared` | `internal/features/**` |
| `internal/features/**` | own slice, `internal/shared`, `internal/platform` | another `internal/features/**` |

These are `depguard` rules in `.golangci.yml` (`domain-pure`,
`usecase-transport-free`, `slice-isolation`, `platform-no-features`).
`make ci` fails on a violation. No `//nolint:depguard`; fix the boundary
instead.

### 2.2 Composition root

`cmd/api/main.go` is the only place that constructs adapters and calls
`Register`:

```go
pool := pgx.Connect(ctx, cfg.DatabaseURL)
users := adminusers.New(pgxadapter.New(pool), auditLog)
adminusers.Register(mux, users)   // module.go owns the route table
```

Business code never instantiates infrastructure, and a slice never imports
another slice's `module.go`.

### 2.3 Shared kernel rules

`internal/shared/` holds exactly: `apperr` (typed errors + stable codes),
`authz` (role-hierarchy checks), `clockx` (injectable clock), `idgen` (UUIDs /
opaque tokens), `rolex` (the role vocabulary `ADMIN > MODERATOR > USER >
GUEST`). Forbidden: business entities, helpers "used by two slices", anything
that would make `shared` depend on `features`. Duplicate a small piece of code
before sharing a domain concept.

## 3. Data layer (outbound adapters)

The data layer is `postgres.go` in each slice, over `internal/gen/db` (sqlc).
The domain and use case never see a SQL string, a pgx type or a sqlc model.

### 3.1 Ownership

- **Each slice owns its tables**, in one Postgres database and one `public`
  schema. Ownership is carried by the table prefix: `email_verifications`,
  `sessions`, `password_resets`, `audit_logs`, `widgets`.
- **No cross-slice joins.** Reference another slice by ID. The one shared root
  is `users(id)`: several slices carry `REFERENCES users`, and that is the
  only cross-slice foreign key allowed. Adding a second one requires removing
  the ownership rule, so it is an architecture decision, not a PR detail.
- There is **no tenant column**. Ownership is `user_id`, checked by the backend
  on every call (`authz.CanManageSelf` / `CanManageSubordinate`). Do not add a
  `tenant_id` because a future feature might need one.
- Standard columns: `id` (uuid), `created_at`, `updated_at`, plus `version` or
  `status` where the row has a lifecycle. Soft delete only where the product
  needs an audit trail, not by default.

### 3.2 sqlc, not an ORM

| Use | For |
|---|---|
| **sqlc queries** (`queries.sql` in the slice) | Every read and write. `make generate` produces `internal/gen/db`; `make gen-check` fails if the committed output is stale |
| **Hand-written SQL inside `queries.sql`** | Read models, reports, aggregates, window functions — still via sqlc so the result is typed |

- `postgres.go` implements the slice's ports over the generated `db.Queries`
  and maps generated rows ↔ domain types. Generated types never leave the
  adapter.
- SQL is always parameterized by sqlc; no string concatenation, no
  `fmt.Sprintf` into a query.
- One transaction boundary per use case. A multi-statement use case takes a
  `pgx.Tx` through the port; the use case decides the boundary, `postgres.go`
  executes inside it. **Shipped deviation:** the auth slices write state and
  then enqueue on the outbox's own pool connection, so the two are not one
  transaction — a crash between them can lose the mail. The upgrade is a
  tx-capable port that takes the caller's `pgx.Tx`; do not claim atomicity
  before the code has it.
- `context.Context` is the first parameter of every query. No `context.TODO()`
  in request paths (`contextcheck` and `noctx` enforce this).

### 3.3 Vector data

No vector database and no `pgvector` extension is shipped. Introducing an
embedding index is a new architecture decision: it goes behind a port in the
owning slice, defaults to `pgvector`, is keyed by
`entityID + contentHash + modelVersion`, and is rebuildable from the database.
Do not add one speculatively.

### 3.4 Migrations

- One goose directory: `migrations/`, shared by all slices.
  Naming `YYYYMMDDHHMMSS_<description>.sql`, created with
  `make migrate-add NAME=add_field`.
- The file is `-- +goose Up` / `-- +goose Down`; every migration has a real
  down step (or an explicit comment saying why it is irreversible).
- Forward-only once merged. A migration that touches a table owned by another
  slice is rejected in review.
- **Expand → migrate → contract** for every breaking change: add the new
  column/table (compatible with old code), backfill in a worker, switch
  reads/writes, drop the old structure in a later release. Never rename or drop
  in the same release as the code change.
- Migrations run via `cmd/migrate` (`make migrate`) as a separate deploy step.
  The API never migrates itself.
- Large backfills are worker jobs with checkpoints, not migration scripts.

### 3.5 Cache

No Redis is shipped. The only cache is **in-process** and it is a decorator,
not an adapter. Rules for a new cache:

- It is a decorator over a use case in the same slice, so the use case stays
  pure and the cache can be removed without touching domain code.
- It is bounded and TTL'd; losing it never loses data or breaks correctness.
- It never caches an authorization decision, and it is invalidated by the
  slice that owns the data.
- A cross-replica cache is a new infrastructure decision: introduce it behind
  a port, document the consistency window, and keep Postgres authoritative.

## 4. Services layer (domain + use case)

### 4.1 Responsibilities

| File | Contains | Does not contain |
|---|---|---|
| `domain.go` | Entities, value objects, invariants, domain errors, pure helpers | I/O, transactions, HTTP, pgx, config |
| `ports.go` | The interfaces **the consumer declares** (`Store`, `Verifier`, `Mailer`, `Publisher`) — 1–2 methods where possible | implementations, framework types |
| `usecase.go` | The use case struct over the ports, one method per command/query, orchestration, transaction boundaries | SQL, HTTP, pgx, generated types |

### 4.2 Use case contract

Every use case:

1. Takes `context.Context` first, then explicit inputs. The caller identity is
   passed as data — `actorID string`, `actorRole rolex.Role` — or read from
   `authx.IdentityFrom(ctx)` in `http.go` and passed down. A use case never
   reads the URL to learn who is calling.
2. Authorizes in the use case, not the router: role hierarchy via
   `internal/shared/authz` (`CanManageSelf`, `CanManageSubordinate`) plus an
   ownership check on the loaded row. `http.go` may reject early with
   `authx.RequireRole`, but it is not the authority.
3. Loads through ports, runs domain behavior, persists, and returns a result
   or an `apperr` typed error. No HTTP status codes here.
4. Never names a transport. `usecase-transport-free` fails the build if
   `net/http`, `pgx` or `internal/gen` appear in the file.

The same use case serves owner and admin routes where the capability is the
same. The difference is the identity and the ownership check, not duplicated
business logic.

### 4.3 Conventions

- Commands change state and return minimal data; queries never change state.
- Errors: wrap with `%w`, match with `errors.Is` / `errors.As`, use the stable
  codes from `internal/shared/apperr`. `http.go` maps them once to
  `{detail, code}`.
- Time, randomness and IDs come from `clockx` / `idgen` so tests are
  deterministic.
- Business invariants live in `domain.go`, not in handlers or SQL.
- A use case that exceeds ~200 lines is a sign the slice owns two aggregates;
  split it before adding a `helpers.go`.

## 5. Batch and asynchronous work

Work that outlives a request (email delivery, backfills, provider sync) enters
the slice through a **worker**, not through a request handler. The worker calls
the same use case; jobs contain no business logic.

The mechanisms shipped in the skeleton:

| Need | Mechanism |
|---|---|
| Email delivery | `cmd/worker` claims `email_jobs` with `SELECT ... FOR UPDATE SKIP LOCKED` |
| Expired-row purges | `cmd/worker` janitor loops, idempotent and replica-safe |
| In-process events | `internal/platform/realtime` hub (single-process fan-out) |
| Slice work that outlives a request | A slice worker polling its own run table |
| Cross-process events | **Not shipped.** Add a bus behind a port when a second process needs the event; Postgres stays the source of truth |

### 5.1 Principles

- **Many workers, one effect.** Any number of replicas may run; the same unit
  of work must produce its effect once.
- **At-least-once delivery + idempotent handlers = effectively once.**
  Exactly-once delivery does not exist; design for redelivery.
- **Postgres is the source of truth; the courier is replaceable.** If a queue
  loses a message, the system may repeat or delay work, but never corrupt
  state.
- Work is scoped to a user or a run, never partitioned by a tenant (there are
  none). A heavy run must not block another run's worker loop.

### 5.2 Rules for every job

1. **Idempotent by design.** Re-running a job with the same id yields the same
   final state. The "already done?" check is a database constraint or a status
   transition, not a lock.
2. **State transitions are explicit.** A run moves `pending → running →
   succeeded|failed`; the worker claims only `pending` rows in one statement
   and never resumes a terminal one.
3. **Cancellable.** Every loop selects on `ctx.Done()`; shutdown drains
   in-flight work and leaves the run resumable.
4. **Bounded.** Retries have a maximum, a backoff, and a dead-letter state.
   Never an infinite retry loop.
5. **Observable.** Every job logs the run id, attempt and final status. No PII,
   no tokens.

### 5.3 Job types

| Type | Trigger | Example |
|---|---|---|
| Outbox worker | Row claimed from `email_jobs` | Send a verification or reset email |
| Janitor loop | `cmd/worker` tick | Purge expired sessions and stale codes |
| Slice worker | Pending run row | A domain pipeline added in a new slice |
| Event consumer | Bus subject (when a bus is added) | Realtime fan-out, reconciliation |
| Operator command | `cmd/*` | Backfill, eval run |

A scheduled job enqueues small, idempotent units, never one giant task.

## 6. Presentation layer (inbound HTTP adapters)

Lives in `http.go` of each slice. Routes follow `PLAYBOOK_URLS.md` exactly.

### 6.1 Structure

Routes are declared in each slice's `http.go` with stdlib `ServeMux` patterns:

```text
internal/features/adminusers/http.go
  mux.Handle("GET /api/v1/admin/users", listHandler)
  mux.Handle("PATCH /api/v1/admin/users/{id}", updateHandler)
  mux.Handle("POST /api/v1/auth/upgrade-to-admin", upgradeHandler)
```

The prefix encodes the audience: `/api/v1/admin/...` requires
`authx.RequireRole(rolex.Admin)`; `/api/v1/...` requires
`authx.RequireAuth` + ownership. There is no `/api/v1/{userType}` segment.

### 6.2 Handler rules

A handler does five things and nothing else:

1. Authenticate through `authx` middleware (mounted by `module.go`).
2. Resolve the actor with `authx.IdentityFrom(r.Context())` → `userID`,
   `role`. There is no tenant resolution and no `X-Tenant-Id` header.
3. Decode and validate the request body (`httpx.Decode`, limit/offset bounds).
4. Call the use case.
5. Map the result or `apperr` to JSON and to the `{detail, code}` envelope.

It contains no business rules and no SQL. A role check in the handler is an
early rejection, not the authority — the use case re-checks ownership.

### 6.3 Contract rules

- Version in the path (`/api/v1`). Additive changes stay in the same version;
  breaking changes create `v2` and a deprecation window.
- Plural resources, IDs in the path, filters/sort/pagination in the query
  string, as in the URL playbook. Actions are `POST` on a verb sub-resource
  (`.../revoke`, `.../sync`).
- List endpoints are paginated with a hard maximum page size
  (`httpx` limit/offset; the SDK clamps client-side too).
- Long operations return a run resource to poll or stream; the work is done by
  a slice worker. Do not hold HTTP connections open for pipeline work.
- Error envelope (same shape as the URL playbook):

```json
{
  "detail": "forbidden: admin role required",
  "code": "FORBIDDEN"
}
```

- Responses never leak internal details (stack traces, SQL, provider keys).
  A resource the caller does not own answers `404`, not `403`, when revealing
  existence would be a leak.

### 6.4 Other inbound protocols

The same use cases are exposed through other adapters without changing the
core: SSE (widget lifecycle events), WebSocket (`internal/platform/realtime`,
`GET /api/v1/ws`), and CLI entrypoints under `cmd/`. Streaming adapters use
the same authorization path as HTTP.

## 7. Connecting slices

`slice-isolation` makes the boundary absolute: **a slice never imports another
slice.** A slice that needs data another slice owns has two options, in this
order.

### 7.1 Level 1 — a port supplied by the composition root (default)

- The consuming slice declares the dependency in its own `ports.go`
  (`type Users interface { GetUser(ctx, id) (domain.User, error) }`) and
  `cmd/api/main.go` supplies an adapter over the owning slice's use case or
  HTTP route. The consumer never imports the producer's package.
- In the monolith the adapter is wired in-process; after extraction (section 9)
  it becomes a network client. The port does not change either way.
- DTOs cross the boundary, not domain entities. The adapter maps.
- Read-mostly calls only. A write that spans slices goes through an event.

### 7.2 Level 2 — asynchronous events (when a bus is added)

- Add the broker behind a port in `internal/platform`; do not let a slice talk
  to NATS/Redis directly.
- Event fields are additive: `<type, version, occurredAt, payload>`. Add
  fields; never repurpose them.
- Consumers are idempotent and keyed by event id. At-least-once, not
  exactly-once.
- Cross-slice workflows with compensation are owned by the initiating slice,
  with explicit compensating actions.
- A task ledger (leases, receipts, artifacts) lives in Postgres, in the same
  transaction as its effect. Do not add a second source of truth.

### 7.3 Level 3 — network protocols between deployed services

When a slice runs as its own service (section 9) the same port gets a
different adapter:

| Interaction | Protocol |
|---|---|
| Synchronous request/response, internal | gRPC or HTTP/JSON |
| Asynchronous, durable | The bus added in Level 2, same event envelope |
| External clients | HTTP/JSON, versioned, under `/api/v1/...` |

Network calls require timeouts, retries only for idempotent operations,
circuit breaking, and service authentication. Trust is never implied by
network position; the callee authorizes.

## 8. Deployment modes: one codebase, several processes

Before any microservice, run the same artifact in different **modes**,
selected by the entrypoint under `cmd/`:

| Mode | Runs | Scales by |
|---|---|---|
| `cmd/api` | HTTP adapters of all slices; starts the in-process slice workers | Request rate |
| `cmd/worker` | Email outbox + janitor loops | Queue depth |
| `cmd/migrate` | One-shot goose migrations | Run once per release |
| Additional `cmd/*` | Operator tools | On demand |

This isolates heavy batch load from interactive latency and keeps one image
with several entrypoints.

## 9. When to extract a slice into a microservice

Default: **modular monolith, several entrypoints**. Extract only with
evidence.

### 9.1 Triggers (any one, measured, sustained)

| Signal | Example threshold (tune with real data) |
|---|---|
| Independent scaling | The slice needs more than ~5x the replicas/CPU/memory of the rest, or a different resource profile (GPU, large memory, long-lived connections) |
| Load isolation | The slice's p95 CPU/DB load degrades other slices even with separate worker pools |
| Data profile | Its dataset or write rate needs its own engine, sharding or retention policy |
| Failure isolation | Its outages or deploys must not take down the core |
| Team / release cadence | A dedicated team ships it independently |
| Runtime fit | It benefits from a different language or runtime |

### 9.2 Anti-triggers

- "It would be cleaner." The module boundary already gives that.
- No measurements, only intuition.
- The slice is still changing together with its neighbors in most PRs.
- The team has no observability or on-call capacity for another service.

### 9.3 Readiness checklist (all required before extracting)

- [ ] The slice owns its tables; no cross-slice FKs remain (only `users(id)`).
- [ ] Every caller goes through an HTTP/event boundary, never an import.
- [ ] Cross-slice writes already go through events.
- [ ] Its workers are idempotent and its run rows carry explicit state.
- [ ] A contract test suite exists for its API and events.

### 9.4 Extraction procedure (strangler)

1. Deploy the slice as its own process against its own tables.
2. Move its tables to their own database instance (config change + data
   migration).
3. Replace in-process adapters in consumers with network adapters (gRPC or
   events). Ports and use cases stay untouched.
4. Run both paths in parallel with shadow traffic, then cut over.
5. Remove the in-process path.

### 9.5 Candidates, in likely order

Extract a long-running, network-bound slice first; keep the auth slices and
`adminusers` in the core (everything depends on them and they are cheap to
run). The `widgets` example is not a candidate — it is a template.

## 10. Cross-cutting rules

### 10.1 Authorization

- There is no tenant context. The actor is `authx.Identity{UserID, Email,
  Role, JTI}`, resolved once per request and passed to the use case as data.
- Authorization = identity + role hierarchy (`rolex`, `authz`) + resource
  ownership, enforced in the use case. The URL prefix, headers sent by the
  client and hidden UI buttons are never the source of truth.
- Admin routes use `authx.RequireRole(rolex.Admin)` at the router **and** the
  use case re-checks; defense in depth, one authority.
- Jobs run as a **system actor** with the minimum privileges needed, scoped to
  the user or run they process.
- Provider keys are encrypted at rest, decrypted only inside the adapter that
  uses them, never logged and never returned in a response.

### 10.2 Observability

- Structured logs with `userID`, `slice`, `useCase`, `runID` where relevant.
  Never log secrets or full PII.
- Per-slice metrics: use-case latency and error rate, run state counts,
  worker lag.
- Health endpoints: `readyz` (database probe via `internal/platform/pgx`),
  `livez` (process).

### 10.3 Configuration and environments

- Typed env configuration (`internal/platform/config`, `<APP>_` prefix),
  validated at startup; the process fails fast on missing or invalid values,
  including secret strength.
- `<APP>_OFFLINE=true` is the switch every network adapter must honor before
  touching a socket (OAuth provider calls, email, eval live paths); tests run
  offline.
- Same artifact in every environment.

### 10.4 Testing pyramid

| Level | Scope | Notes |
|---|---|---|
| Domain unit | `domain.go` invariants and pure helpers | No mocks, no I/O, fastest |
| Use-case | `usecase.go` with in-package fakes of the ports (`fakes_test.go`) | Covers authorization and ownership paths |
| Adapter integration | `postgres.go`, migrations, real Postgres | `-tags=integration`, testcontainers, `make test-int` |
| Acceptance | godog over the real mux with fakes | `make accept`, `specs/features/*.feature` |
| Contract evals | `evals/` (separate module) | `contracts_test.go` + fixtures |
| End-to-end | A few critical flows through HTTP | Keep thin |
| Architecture | depguard: domain purity, slice isolation, platform purity | `make lint`, every PR |
| Performance | Absolute budgets, serial and quiet host | `-tags=perf`, `make test-perf`, not in `make test` |

## 11. Concrete example: a widget lifecycle

Flow: an authenticated user creates a widget, then watches its lifecycle
events.

```text
POST /api/v1/widgets          { name }
GET  /api/v1/widgets/{id}
GET  /api/v1/widgets/events   SSE
```

1. `widgets/http.go` authenticates, reads the identity, validates the body
   (`domain.go`), calls the `Create` use case.
2. `Create` persists a `widgets` row and returns it. The handler answers `201`
   with the row.
3. The use case publishes a lifecycle event to the in-process hub;
   `realtime` streams it over the existing WebSocket, and `widgets` exposes an
   SSE adapter for the same events.
4. Reads load the row through `postgres.go` and re-check ownership before
   returning it. A foreign id answers `404`.
5. If the process restarts, state is rediscovered from Postgres — the
   database, not the hub, is the ledger.

## 12. Checklist

- [ ] Code is organized by slice: `domain.go`, `ports.go`, `usecase.go`,
      `http.go`, `postgres.go`, `module.go`, `queries.sql` — never by
      technical type at the top level.
- [ ] depguard enforces the boundary: `make ci` is green, no `nolint:depguard`.
- [ ] `domain.go` imports only stdlib + `internal/shared`.
- [ ] Each slice owns its tables (prefix) and its `queries.sql`; no cross-slice
      joins, no `tenant_id`.
- [ ] sqlc for every query; `make gen-check` is green.
- [ ] Migrations are goose files in `migrations/`, forward-only,
      expand/contract, run by `cmd/migrate`.
- [ ] Use cases authorize by role + ownership; handlers are thin.
- [ ] Every worker is idempotent, claims terminal-safe state transitions,
      respects `ctx.Done()`, and has bounded retries.
- [ ] Postgres holds truth; the outbox and hub are replaceable couriers.
- [ ] Cross-slice calls go through a port/HTTP/event boundary; cross-slice
      writes go through events.
- [ ] Long operations return a run resource and are executed by a worker.
- [ ] Deployment entrypoints (`cmd/api`, `cmd/worker`, `cmd/migrate`) exist
      before any microservice.
- [ ] A slice is extracted only with a measured trigger and the readiness
      checklist complete.
- [ ] Logs carry the actor, slice and run id; secrets never logged.
- [ ] Provider keys are encrypted at rest and never returned in a response.
