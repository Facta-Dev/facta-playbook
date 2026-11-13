# PLAYBOOK_BACKEND.md — Backend architecture playbook

Architecture contract for Heimdall's backend: **hexagonal architecture
(ports & adapters) organized by vertical slices**. Language-agnostic: every
rule is expressed in terms of folders, dependencies and contracts, not
frameworks. Companion of `PLAYBOOK_URLS.md` (routes and authorization model).

The runnable, machine-checkable companions are
[beautifulgo](https://github.com/Facta-Dev/beautifulgo),
[beautifulnodejs](https://github.com/Facta-Dev/beautifulnodejs),
[beautifulpython](https://github.com/Facta-Dev/beautifulpython) and
[beautifulrust](https://github.com/Facta-Dev/beautifulrust): three Agent Skills
(write, review, port) and a drop-in linter config per language, enforcing this
contract.

The rules that carry the whole file:

1. **A slice owns a business capability end to end** (routes → use cases →
   domain → tables). Code is grouped by capability, never by technical type.
2. **Dependencies point inward**: adapters → application → domain. The domain
   imports nothing.
3. **Slices talk only through published contracts**, never through each
   other's tables, ORM models or internals.
4. **Redis/Dragonfly coordinates, the database decides.** Locks and queues
   prevent duplicate work; correctness is guaranteed by idempotency and
   database constraints.
5. **Extract a slice into a service only when measurements demand it**, and
   the hexagon makes that a swap of adapters, not a rewrite.

## 1. Vertical slicing

### 1.1 What is a slice

A slice is a cohesive business capability with its own language, data and
lifecycle. Heimdall's initial slices (premise in `MEMORY.md`):

| Slice | Responsibility |
|---|---|
| `identity` | Authentication, sessions, tokens, users, roles, RBAC permissions |
| `tenants` | Organizations, membership, API key configuration |
| `projects` | Projects and their configuration |
| `extraction` | Apify extraction runs, raw results, normalization |
| `funnels` | Funnels, leads, stages, per-employee funnel view |
| `campaigns` | Campaign definition, launch, calls/messages |
| `usage` | Metering of runs, tokens, calls per tenant |
| `billing` | Plans, invoices, payment provider |

Slice naming follows the `{module}` segment of `PLAYBOOK_URLS.md`: the URL
module, the slice folder and the permission prefix match
(`funnels` → `/api/v1/app/funnels/...` → `funnels.leads.read`).

### 1.2 Slice boundaries — how to decide

- One slice = one aggregate root family that changes together.
- If two things must be consistent in the same transaction, they belong to the
  same slice.
- If a feature needs data from another slice, it asks through that slice's
  contract (section 7); it never joins its tables.
- Splitting too early is worse than splitting late. Start with fewer, larger
  slices and split when the language or the change rate diverges.
- A `shared-kernel` exists but stays tiny (section 2.3).

## 2. Repository layout

```text
src/
  slices/
    funnels/
      domain/                  # pure business rules
        model/                 # entities, aggregates, value objects
        events/                # domain events
        services/              # domain services (stateless rules)
        errors/
      application/             # use cases and ports
        commands/              # write use cases (CreateLead, MoveLeadStage)
        queries/               # read use cases (ListLeads, GetLeadDetail)
        ports/
          in/                  # use-case interfaces (optional if handlers are the API)
          out/                 # LeadRepository, LeadSimilarityIndex, EventPublisher, Clock...
        policies/              # permission checks, ownership checks
      adapters/
        in/
          http/                # presentation: routes, controllers, DTOs
            app/               # /api/v1/app/funnels/...
            admin/             # /api/v1/admin/tenants/{tenantId}/funnels/...
          jobs/                # batch entrypoints (consumers, schedulers)
          events/              # inbound event handlers from other slices
        out/
          persistence/         # data layer (section 3)
            orm/               # ORM models + repository implementations
            sql/               # hand-written SQL for read models
            migrations/        # this slice's migrations only
          vector/              # vector index adapter
          cache/               # Dragonfly/Redis cache adapter
          clients/             # adapters to other slices or external APIs
      contracts/               # PUBLISHED API of the slice (section 7)
      module.<ext>             # composition: wires ports to adapters
    extraction/ ...
    identity/ ...
  shared-kernel/               # ids, money, pagination, error model, Clock, Result
  platform/                    # cross-cutting infra: config, logging, tracing,
                               # db pool, redis client, http server, outbox runner
  bootstrap/                   # process entrypoints: api, worker, scheduler
tests/
  architecture/                # dependency rule tests
```

### 2.1 Dependency rules (enforced, not suggested)

| From | May import | Must not import |
|---|---|---|
| `domain` | `shared-kernel` only | application, adapters, any framework, ORM, HTTP |
| `application` | own `domain`, `shared-kernel`, other slices' `contracts` | adapters, ORM, HTTP types, another slice's internals |
| `adapters` | own `application` + `domain`, `platform`, libraries | another slice's adapters or domain |
| `contracts` | `shared-kernel` | everything else |

Enforce in CI with an architecture test (ArchUnit, dependency-cruiser,
import-linter, depguard, Deptrac, or the equivalent for the language). A
violation fails the build. Rules that are not enforced decay within weeks.

### 2.2 Composition root

Each slice has one `module` file that binds ports to adapters. Process
entrypoints in `bootstrap/` decide which modules and which inbound adapters
are active (section 8). Business code never instantiates infrastructure.

### 2.3 Shared kernel rules

Allowed: identifiers, pagination, error envelope, `Clock`, `Result`, tenant
context type. Forbidden: business entities, helpers "used by two slices".
Duplicate a small piece of code before sharing a domain concept.

## 3. Data layer (outbound adapters)

The data layer lives in `adapters/out/persistence`, `vector`, and `cache`.
The domain and application never see an ORM model, a SQL string or a vector
client.

### 3.1 Ownership

- **Each slice owns its tables.** Use one database schema per slice
  (`funnels.leads`, `extraction.runs`) in a shared Postgres instance at the
  start, so the split into separate databases later is a configuration change.
- **No cross-slice foreign keys, no cross-slice joins.** Reference other
  slices by ID only (`lead.projectId` is an opaque value). Integrity across
  slices is eventual and handled by events (section 7.2).
- Every tenant-scoped table has `tenant_id NOT NULL`, indexed as the first
  column of composite indexes. Enable row-level security as a second line of
  defense where the database supports it; the application still filters by
  tenant explicitly.
- Standard columns: `id`, `tenant_id`, `created_at`, `updated_at`,
  `version` (optimistic locking), `deleted_at` where soft delete is needed.

### 3.2 ORM vs SQL — when to use which

| Use | For |
|---|---|
| **ORM** (repository implementations) | Aggregate loading and saving, writes, simple lookups by ID |
| **Hand-written SQL** (query adapters) | Read models, reports, dashboards, funnel aggregates, anything with joins, window functions or heavy filtering |

- Commands go through repositories (`LeadRepository.save(lead)`) and map
  ORM model ↔ domain entity inside the adapter. ORM entities never leave the
  adapter.
- Queries may bypass the aggregate and return DTOs straight from SQL
  (CQRS-lite). The query port returns a read DTO, not a domain entity.
- SQL lives in versioned files or a query builder inside `sql/`, always
  parameterized. No string concatenation. Every query has the tenant filter.
- One transaction boundary per use case, opened in the application layer via a
  `UnitOfWork` port. Adapters never commit on their own.

### 3.3 Vector database

- Hidden behind a port that speaks the domain language, for example
  `LeadSimilarityIndex.upsert(leadId, text)` /
  `findSimilar(tenantId, query, limit)`. No vendor types in signatures.
- **Default**: `pgvector` inside the slice's own schema (one fewer system,
  transactional with the rest). **Move to a dedicated engine** (Qdrant,
  Weaviate, Pinecone, Milvus) when measured recall/latency or index size
  requires it; only the adapter changes.
- Tenant isolation is mandatory: a tenant filter or a collection/namespace per
  tenant. A similarity query without a tenant scope must be impossible to
  express through the port.
- Embeddings are derived data: the relational table is the source of truth.
  Indexing is done asynchronously by a batch job (section 5), is idempotent
  (keyed by `entityId + contentHash + modelVersion`), and the index can always
  be rebuilt from the database.
- Store `embedding_model` and `dimension` with each vector; re-embedding on a
  model change is a batch job, not a deploy.

### 3.4 Migrations

- Migrations live in the slice (`adapters/out/persistence/migrations`) and run
  against that slice's schema only. A migration that touches another slice's
  table is rejected in review.
- Forward-only, immutable once merged. Naming:
  `YYYYMMDDHHMM__slice__description`.
- **Expand → migrate → contract** for every breaking change: add the new
  column/table (compatible with old code), backfill in a batch job, switch
  reads/writes, drop the old structure in a later release. Never rename or drop
  in the same release as the code change.
- Migrations run as a separate deploy step (a one-shot job), never at API
  startup in multi-replica environments.
- Large backfills are batch jobs with checkpoints, not migration scripts.

### 3.5 Cache

- Cache is an outbound adapter (`CachePort`), used for read acceleration only.
  Losing the cache must never lose data or break correctness.
- Keys follow section 5.4. Every entry has a TTL.
- Invalidate on domain events from the owning slice; never from another slice.

## 4. Services layer (application + domain)

### 4.1 Responsibilities

| Layer | Contains | Does not contain |
|---|---|---|
| `domain` | Entities, value objects, invariants, domain events, stateless domain services | I/O, transactions, permissions, framework types |
| `application` | Use cases (one class/function per command or query), orchestration, transactions, permission and ownership checks, port definitions | SQL, HTTP, ORM, queue clients |

### 4.2 Use case contract

Every use case:

1. Receives an explicit input object and a **`RequestContext`**
   (`userId`, `tenantId`, `activeRole`, `permissions`, `correlationId`)
   derived from the token and session — never from the URL segment.
2. Authorizes through a policy: permission (`funnels.leads.update`) **and**
   resource ownership/tenant match. This is where the rule "`{userType}` never
   authorizes" is enforced. Controllers may reject early, but the use case is
   the authority.
3. Loads aggregates through ports, executes domain behavior, saves, and
   publishes domain events through the outbox (section 7.2) in the **same
   transaction**.
4. Returns a result object or typed error; it never throws transport-level
   errors (no HTTP codes here).

The same use case serves `/app` and `/admin` routes. The difference is the
`RequestContext` and the policy, not duplicated business logic.

### 4.3 Conventions

- Commands change state and return minimal data (ID, version). Queries never
  change state.
- Domain errors are typed and mapped to the error envelope in the
  presentation layer (`FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `VALIDATION_FAILED`).
- Time, randomness and IDs come from ports (`Clock`, `IdGenerator`) so tests
  are deterministic.
- Business invariants live in the domain, not in validators or controllers.

## 5. Batch processes layer (inbound adapters + coordination)

Batch work (Apify runs, enrichment, embedding, campaign dispatch, usage
rollups, billing cycles, backfills) enters the slice through **inbound
adapters in `adapters/in/jobs`**, the same way HTTP does: it parses the
message, builds a `RequestContext` (a system actor with explicit permissions)
and calls a use case. Jobs contain no business logic.

### 5.1 Principles

- **Many workers, one effect.** Any number of replicas may run; the same unit
  of work must produce its effect once.
- **At-least-once delivery + idempotent handlers = effectively once.**
  Exactly-once delivery does not exist; design for redelivery.
- **Dragonfly/Redis coordinates; the database is the source of truth.** If
  Redis loses data, the system may repeat or delay work, but never corrupt it.
- Work is **partitioned by `tenantId`** (and by entity where useful) so one
  heavy tenant cannot starve the others.

### 5.2 Building blocks

| Need | Mechanism |
|---|---|
| Distributed work queue | Redis Streams with consumer groups (or a queue library built on them, such as BullMQ). One message is delivered to one consumer in the group |
| Dedup at enqueue | Deterministic `jobId` (`{jobType}:{entityId}:{windowOrVersion}`) with `SET NX`; a second enqueue of the same id is a no-op |
| Mutual exclusion on a resource | Lease lock: `SET key token NX PX ttl`; release only if the token matches (compare-and-delete script); renew while working |
| Single scheduler (cron ticks) | Leader lease: only the holder of `scheduler:leader` enqueues scheduled jobs; others stand by. Failover when the lease expires |
| Idempotency of effects | Idempotency record in the **database** (`processed_jobs` table or a unique constraint on the natural key), written in the same transaction as the effect |
| Crash recovery | Visibility timeout + reclaim of pending messages (`XAUTOCLAIM`) for jobs whose worker stopped heartbeating |
| Poison messages | Max attempts → dead-letter stream + alert; never infinite retry |
| Fairness and limits | Per-tenant concurrency cap and token-bucket rate limit (per API key for providers such as Apify) kept in Redis |
| Fallback without Redis | `SELECT ... FOR UPDATE SKIP LOCKED` on a jobs table for low-volume or critical jobs |

### 5.3 Rules for every job

1. **Idempotent by design.** Re-running a job with the same `jobId` yields the
   same final state. Effects check "already done?" through a database
   constraint, not through the lock.
2. **A lock is an optimization, not a guarantee.** A lease can expire while a
   worker is paused. Protect effects with a **fencing token** or a version
   check (`UPDATE ... WHERE version = :expected`) so a stale worker cannot
   overwrite newer state.
3. **Lease longer than the work, renewed by heartbeat**, and shorter than the
   acceptable recovery time. A worker that loses its lease stops.
4. **Chunk long work** into resumable steps with a persisted checkpoint
   (cursor, page, run id). A restart continues, it does not start over.
5. **Retries with exponential backoff and jitter**; classify errors as
   retryable (timeouts, 429, 5xx) or terminal (validation, 4xx) and send
   terminal ones to the dead-letter stream immediately.
6. **Bounded blast radius.** Timeouts on every external call, global and
   per-tenant concurrency limits, backpressure when the queue depth exceeds a
   threshold.
7. **Observable.** Every job logs `jobId`, `jobType`, `tenantId`, `attempt`,
   `correlationId`; emits metrics (queue depth, age of oldest message,
   duration, success/failure, retries, DLQ size).

### 5.4 Key naming in Dragonfly/Redis

```text
heimdall:{env}:{slice}:{purpose}:{id}

heimdall:prod:extraction:lock:run:{runId}
heimdall:prod:extraction:queue:runs                 # stream
heimdall:prod:funnels:dedup:enrich:{leadId}:{hash}
heimdall:prod:platform:scheduler:leader
heimdall:prod:extraction:ratelimit:apify:{apiKeyId}
```

- Every key carries a TTL unless it is a stream with a trimming policy.
- Where cluster mode or multi-key scripts are used, put a hash tag
  (`{tenantId}`) so related keys land on the same shard.
- A slice only touches keys under its own prefix. `platform` owns the shared
  primitives (lock, leader election, rate limiter) and exposes them as ports.
- Dragonfly is wire-compatible with Redis for streams, locks and scripts. Keep
  the code on plain Redis commands so either engine is interchangeable, and
  configure persistence and replication according to how costly a lost queue
  entry is. Remember that a failover can drop the last acknowledged writes:
  this is why idempotency lives in the database.

### 5.5 Job types

| Type | Trigger | Example |
|---|---|---|
| Queue worker | Message on a stream | Execute an Apify run, enrich a lead, embed text |
| Scheduled | Leader enqueues on a cron tick | Hourly usage rollup, nightly billing close |
| Event-driven | Domain event from another slice | `LeadCreated` → index embedding |
| One-off / backfill | Operator command | Re-embed all leads with a new model |

Scheduled jobs enqueue small, idempotent units ("rollup tenant X for hour H"),
not one giant task.

## 6. Presentation layer (inbound HTTP adapters)

Lives in `adapters/in/http`. Routes follow `PLAYBOOK_URLS.md` exactly.

### 6.1 Structure

```text
adapters/in/http/
  app/          # /api/v1/app/...      end-user experience
  admin/        # /api/v1/admin/...    platform/tenant administration
  partner/      # /api/v1/partner/...
  dto/          # request/response schemas per version
  mappers/      # DTO <-> use-case input/output
```

One folder per `{userType}` mirrors the frontend namespaces and layouts.

### 6.2 Controller rules

A controller does five things and nothing else:

1. Authenticate (verify token, resolve session) through `identity`'s contract.
2. Build the `RequestContext` (user, **tenant from the token/session or from
   `X-Tenant-Id` validated against membership**, active role, permissions).
3. Validate and map the request DTO to the use-case input.
4. Call the use case.
5. Map the result or typed error to the response and the error envelope.

It contains no business rules and no SQL. The `{userType}` path segment may
select which controller group serves the route; it must never grant anything.

### 6.3 Contract rules

- Version in the path (`/api/v1`). Additive changes stay in the same version;
  breaking changes create `v2` and a deprecation window.
- Plural resources, IDs in the path, filters/sort/pagination in the query
  string, as in the URL playbook. State transitions are `POST` on a verb
  sub-resource (`.../suspend`, `.../launch`).
- List endpoints are paginated (cursor by default) with a hard maximum page
  size.
- Mutating endpoints that can be retried accept an `Idempotency-Key` header;
  the key is stored in the database with the response.
- Long operations return `202 Accepted` with a resource to poll
  (`/api/v1/app/projects/{projectId}/runs/{runId}`), the work being done by a
  batch job. Do not hold HTTP connections open for extraction work.
- Error envelope (same shape as the URL playbook):

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "You do not have permission to manage users in this tenant.",
    "requiredPermission": "tenant.users.manage",
    "correlationId": "c0a8..."
  }
}
```

- Responses never leak internal details (stack traces, SQL, internal ids of
  other tenants). Cross-tenant access to an existing resource answers
  `404`, not `403`, when revealing existence would be a leak.

### 6.4 Other inbound protocols

The same use cases can be exposed through other adapters without changing the
core: gRPC for service-to-service calls, webhooks (Apify, payment provider)
as `adapters/in/http/webhooks` with signature verification and idempotent
processing, CLI for operations, WebSocket/SSE for live funnel updates.

## 7. Connecting slices

Three levels, in this order of preference. Use the cheapest one that meets the
need.

### 7.1 Level 1 — In-process contract (default)

A slice publishes a small, versioned API in `contracts/` (interfaces + DTOs +
event schemas). Another slice depends only on that package, through its own
outbound port:

```text
funnels/application/ports/out/ProjectDirectory      # port owned by funnels
funnels/adapters/out/clients/ProjectsInProcessAdapter  # calls projects/contracts
```

- The consuming slice defines the port in **its own language** and an adapter
  translates (anti-corruption layer). It never reuses the other slice's
  entities.
- Calls are read-mostly and cheap. Writes across slices are not made through a
  synchronous call chain; use events.
- Contracts are DTOs only: no domain entities, no ORM models, no
  framework types.

### 7.2 Level 2 — Asynchronous events (eventual consistency)

- Slices publish **domain events** (`LeadCreated`, `RunCompleted`,
  `TenantSuspended`) through a **transactional outbox**: the event is written
  in the same database transaction as the state change, and an outbox relay
  publishes it to Redis Streams (or a broker such as NATS, Kafka, RabbitMQ).
- Consumers are inbound adapters (`adapters/in/events`), idempotent, keyed by
  event id, and tolerant to duplicates and reordering within a tenant.
- Events are versioned (`type`, `version`, `occurredAt`, `tenantId`,
  `correlationId`, `payload`). Add fields; never repurpose them.
- Cross-slice workflows that need compensation (launch campaign → reserve
  usage → charge) use a saga/process manager owned by the initiating slice,
  with explicit compensating actions.

### 7.3 Level 3 — Network protocols between deployed services

When a slice runs as its own service (section 9) the same port gets a
different adapter:

| Interaction | Protocol |
|---|---|
| Synchronous request/response, internal | gRPC (or HTTP/JSON with an OpenAPI contract) |
| Asynchronous, durable | Stream/broker with the same event schemas as Level 2 |
| External or partner-facing | HTTP/JSON, versioned, under `/api/v1/{userType}/...` |

Network calls require: timeouts, retries only for idempotent operations,
circuit breaker, bulkhead, propagation of `correlationId` and tenant context,
and **service authentication** (mTLS or signed service tokens). The caller
forwards the end-user identity; the callee still authorizes with RBAC. Trust is
never implied by network position.

## 8. Deployment modes: one codebase, several processes

Before any microservice, run the same artifact in different **modes**,
selected by the entrypoint in `bootstrap/`:

| Mode | Runs | Scales by |
|---|---|---|
| `api` | HTTP inbound adapters of all slices | Request rate |
| `worker` | Queue consumers (optionally limited to chosen slices/queues) | Queue depth |
| `scheduler` | Leader-elected cron enqueuer (1 active, N standby) | Not scaled |
| `migrate` | One-shot migrations | Run once per release |

This isolates heavy batch load from interactive latency (a burst of Apify
runs cannot slow the API) and lets `worker` pools be dedicated per queue,
with no network boundary introduced and no distributed-system cost paid yet.

## 9. When to extract a slice into a microservice

Default: **modular monolith, multiple modes**. Extract only with evidence.

### 9.1 Triggers (any one, measured, sustained)

| Signal | Example threshold (tune with real data) |
|---|---|
| Independent scaling | The slice needs more than ~5x the replicas/CPU/memory of the rest, or a different resource profile (GPU, large memory, long-lived connections) |
| Load isolation | The slice's p95 CPU/DB load degrades other slices even with separate worker pools |
| Data profile | Its dataset or write rate needs its own engine, sharding or retention policy (for example the vector index, or raw extraction results) |
| Failure isolation | Its outages or deploys must not take down the core, and the core must keep working when it is down |
| Team / release cadence | A dedicated team ships it independently and the shared release train is the bottleneck |
| Security / compliance | It handles regulated data (payments, PII) that requires a separate boundary and audit scope |
| Runtime fit | It benefits from a different language or runtime (for example a data/ML pipeline) |

### 9.2 Anti-triggers

- "It would be cleaner." The module boundary already gives that.
- No measurements, only intuition.
- The slice is still changing together with its neighbors in most PRs.
- The team has no observability, tracing or on-call capacity for another
  service.

### 9.3 Readiness checklist (all required before extracting)

- [ ] The slice owns its schema; no cross-slice joins or FKs remain.
- [ ] All callers use its `contracts/` package; no deep imports.
- [ ] Cross-slice writes already go through events, not synchronous chains.
- [ ] Its jobs are idempotent and its queues are namespaced under its prefix.
- [ ] Tracing, metrics and `correlationId` propagation are in place.
- [ ] A contract test suite exists for its API and events.

### 9.4 Extraction procedure (strangler)

1. Deploy the slice as its own process using the existing code (mode
   `worker`/`api` limited to that slice) against its own schema.
2. Move its schema to its own database instance (config change plus data
   migration).
3. Replace the in-process adapters in consumers with network adapters (gRPC or
   events). Ports and use cases stay untouched.
4. Run both paths in parallel with comparison or shadow traffic, then cut over.
5. Remove the in-process path.

### 9.5 Heimdall candidates, in likely order

1. **`extraction`** — bursty, long-running, provider-bound (Apify), the
   first one to isolate as a dedicated `worker` pool, and the first
   candidate for a standalone service if load demands it.
2. **Vector indexing / similarity** (inside `funnels`) — move to a dedicated
   vector engine behind the same port before any service split.
3. **`campaigns` dispatch** — external call/message providers, strict rate
   limits.
4. **`billing`** — likely extracted for compliance reasons rather than load.

`identity`, `tenants` and `projects` stay in the core: everything depends on
them and they are cheap to run.

## 10. Cross-cutting rules

### 10.1 Multi-tenancy and authorization

- Tenant context is resolved once per request/job from the token or job
  payload and carried in `RequestContext`. Every repository and query takes
  the tenant explicitly.
- Authorization = identity + active tenant + permission
  (`module.resource.action`) + resource ownership, enforced in the use case.
  The URL prefix, headers sent by the client and hidden UI buttons are never
  the source of truth.
- Jobs run as a **system actor** with the minimum permissions needed, and
  always carry the `tenantId` of the data they process.
- Secrets (tenant API keys for Apify, providers) are stored encrypted,
  decrypted only inside the outbound adapter that needs them, and never
  logged or returned in responses.

### 10.2 Observability

- Structured logs with `correlationId`, `tenantId`, `userId`, `slice`,
  `useCase`. Never log secrets or full PII.
- Distributed tracing across HTTP, queue messages and events (propagate the
  trace context in message headers).
- Per-slice metrics: use-case latency and error rate, queue depth and age,
  DLQ size, lock contention, outbox lag.
- Health endpoints per mode: liveness (process) and readiness (database,
  Redis reachable).

### 10.3 Configuration and environments

- Twelve-factor configuration from the environment, validated at startup;
  the process fails fast on missing or invalid values.
- Per-slice feature flags for risky rollouts. Same artifact in every
  environment.

### 10.4 Testing pyramid

| Level | Scope | Notes |
|---|---|---|
| Domain unit | Entities, value objects, domain services | No mocks, no I/O, fastest |
| Use-case | Application layer with in-memory fakes of the ports | Covers permissions and ownership paths |
| Adapter integration | Repositories, SQL, vector index, Redis locks/streams against real containers | Includes migration run from scratch |
| Contract | Slice `contracts/` and event schemas, consumer-driven | Required before extraction |
| End-to-end | A few critical flows through HTTP | Keep thin |
| Architecture | Dependency rules, no cross-slice imports | Runs on every PR |
| Concurrency | Two workers on the same job, crash mid-job, duplicate delivery | Proves "no overlap, no duplicates" |

## 11. Concrete example for Heimdall

Flow: the end user launches a campaign from a project.

```text
POST /api/v1/app/projects/{projectId}/launch
```

1. `projects` HTTP adapter (`app/`) authenticates, builds `RequestContext`,
   maps the DTO, returns `202` with a `runId`.
2. `LaunchCampaign` use case (application) checks `campaigns.launch` and that
   the project belongs to the tenant, creates the run aggregate, and writes
   `RunRequested` to the outbox in the same transaction.
3. The outbox relay publishes the event to the `extraction` queue
   (`jobId = extraction:run:{runId}`, deduplicated by `SET NX`).
4. `extraction` worker (any replica) claims the message through the consumer
   group, takes the lease `extraction:lock:run:{runId}`, checks the
   per-API-key rate limit, calls Apify through its outbound adapter and
   persists a checkpoint after every page.
5. Results are written idempotently (unique key on
   `tenantId + externalResultId`) and `LeadsExtracted` is published via the
   outbox.
6. `funnels` consumes `LeadsExtracted`, creates leads in each employee's
   funnel, and enqueues an embedding job per lead; the vector adapter upserts
   by `leadId + contentHash`.
7. `usage` consumes `RunCompleted` and meters the run for `billing`.
8. The user polls `GET /api/v1/app/projects/{projectId}/runs/{runId}` and
   sees progress; an admin sees the tenant-wide view under
   `/api/v1/admin/tenants/{tenantId}/...` through a different policy, not a
   different use case.

If a worker dies at step 4, its lease expires, another worker reclaims the
message and resumes from the checkpoint; the unique key makes repeated
writes harmless. If Redis is wiped, runs in `RUNNING` state without a
heartbeat are re-enqueued by the scheduler sweeper from the database.

## 12. Checklist

- [ ] Code is organized by slice, then by layer (`domain`, `application`,
      `adapters`, `contracts`), never by technical type at the top level.
- [ ] Architecture tests enforce the dependency rules in CI.
- [ ] `domain` has no framework, ORM, HTTP or queue imports.
- [ ] Each slice owns its schema and migrations; no cross-slice joins or FKs.
- [ ] ORM for aggregates and writes; parameterized SQL for read models.
- [ ] Vector store behind a port, tenant-scoped, rebuildable from the database.
- [ ] Migrations are forward-only, expand/contract, run as a separate step.
- [ ] Use cases authorize by permission + ownership using `RequestContext`;
      controllers are thin; `{userType}` never authorizes.
- [ ] Every job is idempotent, has a deterministic `jobId`, a lease with
      heartbeat, a fencing/version check, retries with backoff and a DLQ.
- [ ] Redis/Dragonfly holds coordination only; the database holds truth.
- [ ] Redis keys follow `heimdall:{env}:{slice}:{purpose}:{id}` and have TTLs.
- [ ] Scheduled work uses leader election and enqueues small units.
- [ ] Cross-slice calls go through `contracts/`; cross-slice writes go through
      outbox events.
- [ ] Long operations return `202` and are executed by batch workers.
- [ ] Deployment modes (`api`, `worker`, `scheduler`, `migrate`) exist before
      any microservice.
- [ ] A slice is extracted only with a measured trigger and the readiness
      checklist complete.
- [ ] Logs, traces and metrics carry `correlationId`, `tenantId` and `slice`.
- [ ] Secrets are encrypted at rest and never logged.
