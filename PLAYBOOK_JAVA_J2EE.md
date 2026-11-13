# PLAYBOOK_JAVA_J2EE.md — Java enterprise backend playbook (J2EE · Jakarta EE 11)

Architecture contract for a Java enterprise backend — the stack historically
called **J2EE**, standardized today as **Jakarta EE** — built as a Maven
multi-module reactor with **hexagonal architecture (ports & adapters)
organized by vertical slices**. The rules are expressed as modules, dependency
directions, annotations and contracts. The application server is a deployment
detail; the domain is plain Java.

It is the language-specific specialization of `PLAYBOOK_BACKEND.md` and must
stay consistent with `PLAYBOOK_URLS.md` (routes and authorization),
`PLAYBOOK_TESTS.md` (the proof), `PLAYBOOK_SECURITY.md` and
`PLAYBOOK_AUDIT.md`.

The rules that carry the whole file:

1. **A slice owns a business capability end to end** (resource → use case →
   domain → tables → events). Code is grouped by capability, never by
   technical type.
2. **Dependencies point inward**: adapters → application → domain. The domain
   is plain Java: no `jakarta.*`, no vendor API, no framework annotation.
3. **Slices talk through published contracts and ports**, never through each
   other's entities, tables or repositories.
4. **The use case is the transaction boundary.** One local JTA transaction per
   use case; no XA and no distributed transaction. Cross-slice and async work
   goes through the outbox.
5. **CDI is the composition root.** Constructor injection, explicit scopes; no
   service locator, no static singleton, no JNDI lookup in business code.
6. **JPA entities never cross the API boundary.** DTOs at the edge; mapping
   lives in the adapter.
7. **The application server is a detail.** Business code targets Jakarta EE
   APIs, not WildFly/Payara/Open Liberty vendor APIs.
8. **Authorization is server-side, in the use case.** Identity + active tenant
   + permission + ownership; roles and the URL shape only shape the UX.
9. **Every async job is idempotent.** JMS is at-least-once; the transactional
   outbox is the courier; the database constraint is the last line of defense.
10. **Extract a slice into a service only when measurements demand it**, and
    the hexagon makes that a swap of adapters, not a rewrite.

> Verification note: the Jakarta EE landscape moves. As of November 2026:
> **Jakarta EE 11** is the current platform release, **Java 21 LTS** is the
> safest application-server baseline and **Java 25 LTS** is supported by the
> newest servers, **Hibernate 7** implements JPA 3.2, and WildFly, Payara and
> Open Liberty ship Jakarta EE 11-compatible releases. Items marked
> **(verify)** depend on the exact server, library or vendor support and must
> be re-checked before adoption. Version numbers in this file are a baseline,
> not a promise.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Java version | **Java 21 LTS** by default; Java 25 LTS once the server, agent and libraries certify it **(verify)** | Broad app-server support, virtual threads where the server enables them, records and pattern matching |
| D2 | Platform | **Jakarta EE 11**, `jakarta.*` namespace; Web Profile unless the product needs Full (Batch, JMS, Connectors) | J2EE/Java EE 8 used `javax.*`; the namespace change is not a rename you can mix |
| D3 | Runtime | One portable target: **WildFly**, **Payara** or **Open Liberty**, in a container; a bootable jar / micro distribution for small services **(verify)** | The server is replaceable; vendor APIs are not the contract |
| D4 | Build | **Maven multi-module reactor** with `dependencyManagement`, Enforcer and versions plugins; Gradle only with an ADR | The enterprise norm, reproducible and auditable; Enforcer enforces the module graph |
| D5 | Persistence | **JPA 3.2 + Hibernate 7**, PostgreSQL primary, **Flyway** migrations; `hbm2ddl.auto=validate` in every deployed profile | One transaction model; explicit migrations; no schema surprises |
| D6 | Transactions | **JTA `@Transactional`** on the use case; local transactions only; the outbox replaces 2PC | Clear boundary, no XA, testable |
| D7 | Messaging | **Jakarta Messaging 3.1 (JMS) with ActiveMQ Artemis**; MDB or a CDI listener adapter; a **transactional outbox** for reliability | At-least-once delivery plus idempotency; the outbox survives a broker outage |
| D8 | Presentation | **JAX-RS 4 + Jackson or JSON-B + Bean Validation + MicroProfile OpenAPI** | JSON API for the SPA/native clients; no JSF for new products |
| D9 | Security | **Jakarta Security + MicroProfile JWT/OIDC**; tenant and ownership in the use case; container realms are a deployment option | Portable, testable, not tied to a server realm |
| D10 | Config | **MicroProfile Config**: system properties > environment variables > `microprofile-config.properties` (default ordinals); secrets from a vault/Kubernetes Secret | The spec-defined precedence; an env-first policy needs a custom `ConfigSource` with a higher ordinal |
| D11 | Observability | **MicroProfile Health, Metrics and OpenTelemetry**; correlation IDs in logs/MDC | Readiness/liveness, traces, evidence |
| D12 | Testing | **JUnit 5, Weld SE for CDI, Testcontainers (PostgreSQL/Artemis), REST Assured, ArchUnit** | Fast unit tests, real adapters, enforced layers |
| D13 | UI | **No JSF/Faces for new SPA-backed products**; legacy Faces screens only behind an ADR | The product family serves JSON to React/native clients |
| D14 | Deployment | **WAR/EAR on a server** or a bootable jar/container; migrations run as a deploy step, never on app start | Zero-downtime rolling and rollback |

---

## 2. Vertical slicing

### 2.1 What is a slice

A slice is a cohesive business capability with its own language, data and
lifecycle. In the Maven reactor every slice lives under
`features/<name>/` and owns four layer modules plus an optional published
`contracts` module:

```text
features/<name>/
  domain/          # entities, value objects, rules, ports — plain Java
  application/     # use cases, transaction boundary, orchestration
  adapters/        # JPA, JMS, HTTP clients, cache, files — the outside world
  presentation/    # JAX-RS resources and DTOs for this slice
  contracts/       # the slice's published API/event types (when others call it)
```

A typical reactor ships:

| Slice | Responsibility |
|---|---|
| `identity` | Signup, login, session, password reset, OIDC callbacks |
| `tenants` | Tenant lifecycle, membership, roles and permissions |
| `widgets` | Reference owner-scoped CRUD slice; publishes lifecycle events |
| `billing` | Plans, usage, invoices (admin experience) |

Slice naming follows the URL module of `PLAYBOOK_URLS.md`: the folder, the
route prefix and the table prefix align (`identity` → `/api/v1/public/identity`
and `/api/v1/app/session`; `widgets` → `/api/v1/app/widgets`).

### 2.2 Slice boundaries

- One slice = one aggregate-root family that changes together.
- If two things must be consistent in the same transaction, they belong to the
  same slice.
- If a slice needs another slice's data, it declares a port and the
  composition root supplies an adapter (section 11); it never imports the
  other slice's entities or joins its tables.
- Splitting too early is worse than splitting late. Start with one slice and
  split when the capabilities change independently.
- The shared kernel (`shared/`) stays tiny: `TenantId`, `CorrelationId`,
  `Money`, `Result`, `AppError`, `Clock`. No business rules there.

---

## 3. Repository layout and module rules

```text
<app>/
  pom.xml                    # reactor: dependencyManagement, Enforcer, plugins
  shared/                    # tiny shared kernel (pure Java)
  features/
    identity/{domain,application,adapters,presentation,contracts}/
    tenants/{...}/
    widgets/{...}/
  apps/
    api/                     # WAR: JAX-RS application, CDI wiring, resources
    worker/                  # runnable worker: outbox poller, JMS listeners, jobs
  migrations/                # Flyway SQL, one directory, ordered
  config/                    # server config, datasource templates, env examples
```

### 3.1 Dependency rules (enforced)

| Module | May depend on | Must not depend on |
|---|---|---|
| `features/*/domain` | Java SE, `shared` | `jakarta.*`, Spring, Hibernate, Jackson, the server |
| `features/*/application` | its `domain`, `contracts`, `shared`, `jakarta.transaction-api`, `jakarta.inject-api` | `jakarta.persistence`, JAX-RS, JMS, JPA entities, vendor APIs |
| `features/*/adapters` | its `application`, `domain`, `contracts`, `jakarta.persistence`, JMS/HTTP clients | `presentation`, another slice's internals |
| `features/*/presentation` | its `application`, `contracts`, `jakarta.ws.rs`, validation, JSON | `adapters`, JPA, another slice's internals |
| `apps/api` | every `presentation`, `adapters`; assembles the graph | business rules |
| `apps/worker` | `application`, `adapters`; the job entry points | JAX-RS resources |

- **Maven Enforcer** `bannedDependencies` fails a forbidden artifact on the
  classpath, and the reactor fails a module that does not declare its
  dependency.
- **ArchUnit** tests assert the package/layer rules the compiler cannot see
  (for example `domain` must not depend on `application`).
- **No cyclic module dependency**; a cycle is an architecture bug, not a build
  setting to relax.

### 3.2 Composition root

- `apps/api` and `apps/worker` are the only places that know which adapter is
  live. They build the object graph through CDI producers or a small
  `@ApplicationScoped` configuration class.
- A producer returns the port type, not the implementation type; consumers
  inject the port.
- Tests build the same graph with fakes; the use case never changes.

---

## 4. Domain and application layers

### 4.1 Domain

- Plain Java: entities with private state, constructors/factories that enforce
  invariants, no setters, no framework annotation.
- Value objects are `record`s (`WidgetId`, `TenantId`, `Money`) with validation
  in the compact constructor.
- Domain events are plain records; the application decides when to publish
  them. The domain does not know about JMS.
- Domain exceptions are for invariant violations (programmer errors). Expected
  failures are values (`Result`).

### 4.2 Use case contract

```java
public interface ArchiveWidget {
    Result<Widget> archive(TenantId tenantId, WidgetId id, Version expected, IdempotencyKey key);
}
```

- One use case = one class (or one method) with an explicit input and output.
- `@Transactional` (JTA) sits on the use-case implementation, never on a JAX-RS
  resource or a repository.
- The use case loads through ports, applies the rule, writes through ports and
  returns a `Result`. It never sees `EntityManager`, `HttpServletRequest` or a
  `SecurityContext`.
- Idempotency: a unique key per mutation; a replay returns the stored outcome,
  not a second effect.
- The use case is a CDI bean (`@ApplicationScoped`) with constructor
  injection; no field injection.

### 4.3 Scopes and state

| Scope | Use |
|---|---|
| `@ApplicationScoped` | Stateless services/use cases; thread-safe by design |
| `@RequestScoped` | Request-scoped context (correlation, tenant, security) |
| `@Dependent` | Small helpers owned by their injector |
| `@Singleton` | Only for a deliberate, thread-safe, immutable holder; never mutable request state |

No mutable state in an `@ApplicationScoped` bean. A cache is a port with
explicit concurrency; a counter is a database row or a metric, not a field.

---

## 5. Data layer (outbound adapters)

### 5.1 JPA and the persistence model

- JPA entities live in `adapters`, not in `domain`. Map domain ↔ entity at the
  repository boundary; the domain never sees `@Entity`, lazy proxies or
  `EntityManager`.
- `@PersistenceContext` in the adapter; the `EntityManager` is
  transaction-scoped. Never store it in a field of a longer-lived bean.
- `@Version` on every mutable aggregate for optimistic locking; an
  `OptimisticLockException` becomes a typed `conflict` result.
- Fetch what the use case needs: fetch joins or DTO projections; no lazy
  loading after the transaction closes, no Open Session in View.
- Pagination is cursor/keyset where the feed is large; `LIMIT/OFFSET` only for
  small admin tables.

### 5.2 Migrations

- **Flyway** (or Liquibase) is the only schema authority. `hbm2ddl.auto` is
  `validate` in every deployed profile; `create/update` is forbidden outside a
  throwaway test.
- Migrations are ordered, immutable once released, and reviewed like code.
- Zero-downtime changes use **expand/contract**: add the new column/table, dual
  write/backfill, switch reads, remove the old shape in a later release.
- Every migration has a test against a fixture of the previous release; a
  destructive migration needs a backup and a rollback note.

### 5.3 Cache and vector data

- The JPA second-level cache is for immutable reference data only; never cache
  tenant-scoped mutable rows without a tenant-aware key.
- An application cache (Redis/Dragonfly) is behind a port, with explicit
  invalidation and tenant-prefixed keys.
- Vector search (pgvector or a dedicated engine) is behind a port; it never
  leaks into the domain.

### 5.4 Tenant isolation in the schema

- Every table carries `tenant_id NOT NULL`; it is the first column of every
  primary/unique index and every query predicate.
- Standard columns: `id`, `tenant_id`, `created_at`, `updated_at`, `version`,
  `deleted_at`. No exceptions per slice.
- Schema per slice; no cross-slice foreign keys or joins. A cross-slice read
  goes through the provider's contract, not its tables.
- **Row-level security is the second line.** PostgreSQL RLS on every tenant
  table, with a dedicated application role that cannot bypass it; DDL and
  migrations use a separate owner role. The policy reads the tenant set on the
  transaction (`current_setting('app.tenant_id')`), set by the adapter from
  the validated token or job payload.
- The JPA adapter sets the tenant on the transaction before the first query; a
  connection borrowed from the pool never inherits another tenant's setting.
- Tests with two seeded tenants: a query without the tenant predicate returns
  zero rows; a write to another tenant's row fails; the application role
  cannot disable RLS.

---

## 6. Messaging, batch and async work

### 6.1 The transactional outbox

- The domain row and the outbox row are written in the **same JTA
  transaction**. A commit means both; a rollback means neither.
- The worker polls due rows with `FOR UPDATE SKIP LOCKED` (PostgreSQL) or an
  equivalent, sends through JMS/HTTP, and marks the row sent.
- Each job carries an idempotency key; the consumer deduplicates. JMS is
  at-least-once, not exactly-once.
- A failure backs off with jitter and lands in a dead-letter store with an
  admin-visible count.

### 6.2 JMS

- Jakarta Messaging 3.1 with ActiveMQ Artemis (or the server's broker).
- A listener is an adapter: MDB (`@MessageDriven`) or a CDI bean using
  `JMSContext`; the handler calls the same use case as the HTTP path.
- Queues/topics are namespaced by slice and tenant where needed; a message
  carries `correlationId`, `tenantId` and `idempotencyKey`.
- Do not put a full entity or a secret in a message; send an id and re-read
  the truth.

### 6.3 Scheduling and concurrency

- `@Schedule` or `ManagedScheduledExecutorService`; **never** `new Thread`,
  `Executors` or an unmanaged `Timer` inside a managed server.
- One scheduler per deployment mode; a job that must not overlap uses a
  database lock or a single-consumer queue.
- Bound the work: batch size, time budget, backpressure. A long job checkpoints
  and resumes; it does not run forever in one transaction.

### 6.4 Long operations (`202`)

- A command that exceeds the request budget creates a run row
  (`runId`, `status`, `progress`) and returns `202 Accepted`.
- A worker (or a batch job) executes it; the client polls or subscribes to
  realtime updates.
- The run row is the source of truth; a cancelled or failed run is visible, not
  a silent queue.

---

## 7. Presentation layer (JAX-RS)

### 7.1 Resources are thin

```java
@Path("/api/v1/app/widgets")
@Produces(MediaType.APPLICATION_JSON)
public final class WidgetResource {
    private final ArchiveWidget archiveWidget;

    @Inject
    public WidgetResource(ArchiveWidget archiveWidget) {
        this.archiveWidget = archiveWidget;
    }

    @POST
    @Path("/{id}/archive")
    public Response archive(
            @PathParam("id") WidgetId id,
            @HeaderParam("If-Match") String expected,
            @HeaderParam("Idempotency-Key") IdempotencyKey key) {
        return archiveWidget.archive(tenant(), id, Version.parse(expected), key)
                .fold(Response::ok, this::error);
    }
}
```

- A resource parses/validates the request, calls one use case and maps the
  `Result` to a response. No business rule, no `EntityManager`, no transaction.
- DTOs are records with Bean Validation; the resource never returns an entity.
- `@Valid` on the DTO; a `ConstraintViolationException` maps to `validation`.

### 7.2 Error envelope

The shape is the platform's, not the server's:

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

- One `ExceptionMapper` per error family, not a `try/catch` in every resource.
- Map the typed error to HTTP: unauthenticated 401, forbidden 403, not-found
  404, conflict 409, validation 422, rate-limited 429, server 500/503.
- Never leak a stack trace, SQL text or an internal id. The `correlationId`
  is the bridge to the logs.

### 7.3 Contracts

- JSON-B or Jackson, chosen once; no mixed serializers.
- OpenAPI generated from annotations (`@Operation`, `@Schema`) or a curated
  spec; the generated clients are the contract.
- Cursor pagination with a stable `(createdAt, id)` key; a bounded page size.
- `ETag`/`If-Match` for optimistic concurrency; `Idempotency-Key` for
  mutations; `202` for long operations.
- CORS is an allowlist; CSRF protection is mandatory for cookie sessions.

### 7.4 Legacy Faces and Servlets

- **JSF/Faces is legacy** for new products: the SPA/native clients consume
  JSON. A new Faces screen needs an ADR and a migration plan.
- Servlets and JAX-RS filters are for cross-cutting concerns (correlation,
  auth, CORS, security headers), never for business logic.

---

## 8. Security

- **Jakarta Security** for authentication/authorization primitives:
  `SecurityContext`, `@RolesAllowed`; **MicroProfile JWT** for OIDC token
  validation, or an app-managed OIDC adapter with the same port.
- The container realm is a deployment option; tenant and ownership decisions
  stay in the use case. `@RolesAllowed` alone is not multi-tenant isolation.
- Tenant is resolved from the validated token (or the job payload), never from
  a query string or an unverified header. Every repository call takes the
  `TenantId` explicitly.
- Authorization = identity + active tenant + permission
  (`module.resource.action`) + resource ownership, enforced in the use case.
- The database is the second line: `tenant_id` on every table, RLS with a
  non-bypass application role, and cross-slice access only through contracts
  (section 5.4). A missing predicate must yield an empty/404 result, never a
  cross-tenant read.
- Secrets: vault / Kubernetes Secret; datasource credentials via the server's
  JNDI configuration; never in `microprofile-config.properties`,
  `persistence.xml`, logs or the image.
- Passwords: Argon2id (or bcrypt for legacy); MFA for admin/tenant owners;
  sessions rotated and revocable.
- CSRF for cookie sessions; CORS allowlist; security headers; TLS only.
- Audit log for admin actions; no PII or token in logs; the correlation id
  links the request, the use case and the audit row.

---

## 9. Configuration and observability

### 9.1 Configuration

```text
system properties > environment variables > microprofile-config.properties > defaults
```

- These are the specification's default ordinals (400 > 300 > 100): a `-D`
  system property overrides an environment variable, and both override the
  packaged properties file. An env-first policy needs a custom `ConfigSource`
  with a higher ordinal; do not assume the shell wins.
- Every key is documented; every environment has an example, not a secret.
- Profile-specific config (dev/staging/prod) is a file or a ConfigMap, not a
  code branch.
- Managed/enterprise policy (where the product has one) is a read-only source
  that wins over user config; the app reports the effective source.

### 9.2 Health, metrics and traces

- **MicroProfile Health**: readiness (DB, broker, required dependencies) and
  liveness (the process is alive); no deep check on the liveness path.
- **Metrics**: request latency, error rate, queue depth/age, outbox/DLQ, pool
  saturation, job duration.
- **OpenTelemetry** traces with `correlationId` propagated from HTTP/JMS/outbox
  to the adapter; a span per use case.
- Structured logs with `correlationId`, `tenantId`, `userId`, `slice`,
  `useCase`; never secrets or full PII.

---

## 10. Deployment

- **WAR/EAR on WildFly, Payara or Open Liberty**, or a bootable jar/micro
  distribution in a container. The choice is per deployment, not per module.
- **Server configuration is code**: datasource, JMS, security realm, thread
  pools and JVM flags live in the image/Helm/Ansible, never in the admin
  console.
- **Migrations run as a deploy step** (init container or a job), before the
  new application version receives traffic; the app itself never migrates.
- Stateless application nodes; sessions are tokens, not server memory. If a
  session store is unavoidable, it is shared and tenant-aware.
- Connection pools and JVM heap are sized from measurements, not defaults;
  container-aware flags (`-XX:MaxRAMPercentage`, CPU limits) are explicit.
- Rolling deploy with readiness gates; expand/contract migrations; a rollback
  path for the previous artifact and schema.
- Backups and restore drills cover the database, the object store and the
  broker configuration; the audit trail is retained per policy.

---

## 11. Connecting slices and extracting services

### 11.1 Levels

1. **In-process port (default).** The consumer declares the port; the
   composition root supplies an adapter that calls the provider's use case.
   CDI events are for in-process notifications, not for durable work.
2. **Asynchronous message.** A durable event goes through the outbox and JMS;
   the consumer is idempotent and tenant-scoped.
3. **Network protocol.** Only after extraction: REST/gRPC between services,
   with the same contracts and tracing.

### 11.2 Extraction triggers and readiness

- Triggers: independent scaling, load isolation, a different data profile,
  failure isolation, a separate team/release cadence, regulated data, or a
  different runtime. Any one, measured and sustained.
- Anti-triggers: "it would be cleaner", no measurements, still changing
  together, no on-call/observability for another service.
- Readiness: the slice owns its schema, all callers use `contracts`, async
  writes already go through events, jobs are idempotent, tracing/metrics exist,
  and a contract test suite proves the API/events.
- Procedure: deploy the slice as its own worker first, move its schema, replace
  in-process adapters with network adapters, run both paths in parallel, cut
  over, remove the old path.

---

## 12. Testing

| Level | Tooling | What it proves |
|---|---|---|
| Unit (domain) | JUnit 5, no mocks | Invariants, value objects, rules |
| Use case | JUnit 5 + fake ports | Orchestration, authorization, `Result` mapping |
| CDI wiring | Weld SE | Producers, scopes, interceptors |
| Adapters | Testcontainers PostgreSQL/Artemis | JPA mapping, migrations, outbox, JMS |
| API | REST Assured + a deployed app (Testcontainers) | Status codes, envelope, validation, authz |
| Architecture | ArchUnit + Maven Enforcer | Layer and module rules |
| Contract | generated OpenAPI + consumer tests | The API did not drift |
| Performance | Gatling/JMH on a staging profile | Latency/throughput budgets |
| Smoke | the built WAR/EAR on the target server | The deployment actually starts |

- Every use case has a test for the happy path, each expected failure, and the
  permission/ownership denial.
- Every migration has a fixture from the previous release.
- Every async handler has an idempotency test (run it twice, one effect).
- Every endpoint has an authz matrix row and a tenant-isolation test with two
  tenants.

---

## 13. Concrete example: archive a widget

```java
public final class ArchiveWidgetService implements ArchiveWidget {
    private final WidgetRepository widgets;
    private final Outbox outbox;
    private final Clock clock;

    @Inject
    public ArchiveWidgetService(WidgetRepository widgets, Outbox outbox, Clock clock) {
        this.widgets = widgets;
        this.outbox = outbox;
        this.clock = clock;
    }

    @Override
    @Transactional
    public Result<Widget> archive(TenantId tenantId, WidgetId id, Version expected, IdempotencyKey key) {
        return widgets.findById(tenantId, id)
                .map(widget -> {
                    if (!widget.tenantId().equals(tenantId)) {
                        return Result.<Widget>failure(AppError.forbidden("widgets.widget.archive"));
                    }
                    if (!widget.version().equals(expected)) {
                        return Result.<Widget>failure(AppError.conflict());
                    }
                    Widget archived = widget.archive(clock.now());
                    widgets.save(tenantId, archived);
                    outbox.append(tenantId, WidgetArchived.of(archived), key);
                    return Result.success(archived);
                })
                .orElseGet(() -> Result.failure(AppError.notFound()));
    }
}
```

- The transaction covers the widget row and the outbox row.
- The permission check and the version check happen before the write.
- The outbox row is idempotent by `key`; a retry is safe.
- The resource maps `forbidden` → 403, `conflict` → 409, `notFound` → 404.

---

## 14. Anti-patterns (reject in review)

| Anti-pattern | Why it is wrong | Do instead |
|---|---|---|
| EJB session beans for new business logic | Transaction/lifecycle coupling, harder tests, legacy | CDI beans + `@Transactional` use cases |
| JSF/Faces for a new SPA-backed product | A second UI stack and a server session | JAX-RS JSON; Faces only for legacy |
| Entity returned from a resource | Lazy loading, over-posting, schema leak | DTO records + mapping |
| `@Transactional` on a resource or repository | The boundary is wrong; partial effects | Put it on the use case |
| `EntityManager` in a use case or resource | Persistence leaks into the application | Repository port + adapter |
| `hbm2ddl.auto=update` in staging/prod | Silent, unreviewed schema changes | Flyway + `validate` |
| Open Session in View | Lazy loading after commit, hidden N+1 | Fetch joins/projections inside the transaction |
| `new Thread`/`Executors` in a managed server | Unmanaged lifecycle, no context propagation | Jakarta Concurrency / JMS / outbox |
| JNDI lookup in business code | Service locator, untestable | Constructor injection |
| XA across two resources | Fragile, slow, operationally painful | Local transaction + outbox/idempotency |
| `@Singleton` mutable state | Data races and cross-tenant leakage | Database/cache with a tenant key |
| Container realm as the only authz | No tenant/ownership enforcement | Use-case authorization |
| Secrets in `microprofile-config.properties` | Leaks into the image and git | Vault/Kubernetes Secret |
| `catch (Exception) { log; return 500; }` in every resource | Hides the taxonomy, leaks details | One `ExceptionMapper` per error family |
| A chatty JPA graph for a list endpoint | N+1 and memory blowups | DTO projection + pagination |
| A migration edited after release | Every environment diverges | Add a new migration |

---

## 15. Checklist

- [ ] The slice owns its capability, schema, contracts and tests; no
      cross-slice entity or table access.
- [ ] `domain` is plain Java; `application` has no JPA/JAX-RS/JMS; Enforcer and
      ArchUnit fail a violation.
- [ ] Every mutation is a use case with `@Transactional`, a `Result` and an
      idempotency key; the resource is thin.
- [ ] Entities never cross the API; DTOs are validated; the error envelope is
      the platform shape.
- [ ] Optimistic locking is on every mutable aggregate; conflicts are 409, not
      500.
- [ ] Flyway is the only schema authority; `validate` in deployed profiles;
      migrations have previous-release fixtures.
- [ ] The outbox and the domain row commit together; the worker is idempotent;
      the DLQ is visible.
- [ ] JMS handlers, schedulers and batch jobs are bounded, tenant-scoped and
      observable.
- [ ] Tenant and permission/ownership checks are in the use case; the token is
      the only tenant source; `tenant_id` + RLS are in the schema and the
      application role cannot bypass them; two-tenant isolation tests pass.
- [ ] Secrets come from a vault/Secret; no secret in config, images or logs.
- [ ] Health, metrics and traces are wired; `correlationId` crosses HTTP, JMS
      and the outbox.
- [ ] The WAR/EAR/bootable jar starts on the target server; migrations run as a
      deploy step; rolling and rollback are exercised.
- [ ] The test pyramid and the authz matrix are green; a performance budget is
      measured on the reference profile.
- [ ] `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md` and `PLAYBOOK_AUDIT.md`
      evidence exists for the release.
