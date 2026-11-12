# PLAYBOOK_TESTS.md — Testing & compliance-evidence playbook

Testing contract for Heimdall. It follows the same **vertical slicing** as
`PLAYBOOK_BACKEND.md` and `PLAYBOOK_FRONTEND.md`, uses the route and
permission model of `PLAYBOOK_URLS.md`, and is designed so that the test
suite doubles as **audit evidence for SOC 2 and ISO/IEC 27001**.

The rules that carry the whole file:

1. **Tests are organized by slice first, then by side (`backend` / `frontend`),
   then by test type.** The folder tells you what is covered and who owns it.
2. **Test where the risk is.** Business rules at the cheapest level (unit),
   wiring against real infrastructure (integration), agreements between
   parties (contract), and only the critical journeys end to end.
3. **Authorization, tenant isolation, idempotency and privacy are first-class
   test targets**, not afterthoughts. They are what this product can least
   afford to get wrong.
4. **Every control-relevant test is tagged with the control it supports** and
   produces a retained, tamper-evident artifact. A control that is not tested
   automatically is a control that is not evidenced.
5. **Tests never use production data and never weaken the pipeline.** A red
   gate blocks the release; there is no silent skip.

> Scope note: a passing suite produces *evidence*; it does not by itself make
> the company compliant. SOC 2 and ISO 27001 also require policies, reviews,
> training and operational processes. The control mappings below are a working
> starting point and must be confirmed with your auditor and your own
> Statement of Applicability / system description.

## 1. Repository layout

```text
tests/
  _shared/                         # no tests here, only reusable support code
    builders/                      # test data builders (Lead, Tenant, User, Run)
    fakes/                         # in-memory fakes of ports (Clock, Repo, Queue, Notifier)
    fixtures/                      # synthetic datasets, OpenAPI snapshots, event samples
    containers/                    # Postgres, Dragonfly/Redis, vector DB, mock providers
    auth/                          # token factories, role/permission personas
    msw/                           # network mocks for frontend tests
  _platform/                       # cross-slice and cross-cutting suites (section 8)
    architecture/
    authz-matrix/
    tenant-isolation/
    security/                      # sast, sca, secrets, dast, iac, container, sbom
    privacy/
    audit-log/
    resilience/
    performance/
    disaster-recovery/
    observability/
    compliance/                    # control map, evidence collectors, traceability report
  _journeys/                       # cross-slice end-to-end journeys (section 7.3)
  funnels/                         # one folder per slice, same names as src/slices
    backend/
      unit/
        domain/
        application/
      integration/
        persistence/               # ORM, SQL read models, RLS
        vector/
        queue/                     # Dragonfly/Redis locks, streams, dedup
        http/                      # controllers + real use cases + real DB
        jobs/
        events/                    # outbox, inbound handlers
      contract/
        http/                      # provider verification against OpenAPI
        events/                    # event schemas, consumer-driven
        slice/                     # contracts/ package between slices
        external/                  # Apify, payment, messaging providers
      concurrency/                 # duplicate delivery, two workers, crash mid-job
      migration/
      security/                    # authz, IDOR, input handling for this slice
      performance/
      e2e/                         # API-level critical flows
    frontend/
      unit/
        viewmodels/
        sdk/
        core/                      # route builders, permission helpers, mappers
      component/                   # Views and presentational components
      integration/                 # View + ViewModel + SDK against MSW
      contract/                    # SDK vs OpenAPI, extension message schemas
      e2e/
        web/
        native/
        extension/
      a11y/
      visual/
      security/                    # XSS, token storage, CSP, extension manifest
      performance/                 # bundle budgets, web vitals
  extraction/ ...
  identity/ ...
  tenants/ ...
```

Rules:

- Folder names under `tests/` match slice names in `src/slices/` and the URL
  `{module}`; an unmatched folder fails the architecture test.
- `unit/` mirrors the source path of what it tests
  (`tests/funnels/backend/unit/domain/model/lead.unit.<ext>` ↔
  `src/slices/funnels/domain/model/lead.<ext>`).
- A slice with no frontend omits `frontend/`; do not create empty trees.
- `_shared` contains support code only. It is not a place for cross-slice
  business assertions.
- Each slice folder has a `CODEOWNERS` entry; the slice team owns its tests,
  `_platform` is co-owned by Engineering, Security and QA.

### 1.1 Naming

- File suffix declares the type: `*.unit`, `*.integration`, `*.contract`,
  `*.e2e`, `*.security`, `*.perf`, `*.a11y` (with the language's usual
  extension).
- Test names describe behavior, not implementation:
  `rejects_lead_update_when_user_lacks_funnels_leads_update`.
- Every test that supports a control carries a tag (section 10.2).
- Test IDs are stable (`TST-FUN-BE-INT-0042`) so audit evidence can reference
  them across releases.

## 2. Test types and the pyramid

| Type | Proves | Real vs. fake | Speed | Share of suite |
|---|---|---|---|---|
| **Unit** | Business rules and presentation logic | Everything external faked | ms | Most |
| **Integration** | Adapters work against real infrastructure | Real DB, Dragonfly/Redis, vector DB (containers); external APIs mocked | seconds | Many |
| **Contract** | Two parties agree on an interface | Each side tested alone against a shared contract | seconds | Moderate |
| **End-to-end (E2E)** | A critical user journey works across the deployed stack | Everything real except third-party providers (sandboxed) | minutes | Few |
| **Security** | Authorization, isolation, input handling, supply chain | Mixed | seconds–minutes | Targeted and continuous |
| **Non-functional** | Performance, resilience, accessibility, recovery | Prod-like environment | minutes–hours | Scheduled |

Placement rule: **push each assertion to the lowest level that can prove it.**
A permission rule is a unit test of the policy; that the route enforces it is
one integration test; that the real token flows through the stack is one E2E
check. Do not repeat the same assertion at three levels.

## 3. Backend tests

Maps one-to-one to the hexagon in `PLAYBOOK_BACKEND.md`.

### 3.1 Unit

| Folder | Scope | Rules |
|---|---|---|
| `unit/domain` | Entities, value objects, invariants, domain events, domain services | No mocks, no I/O, no framework; deterministic (`Clock`, `IdGenerator` injected) |
| `unit/application` | Use cases with in-memory fakes of every outbound port | Covers: happy path, validation, **permission denied**, **wrong tenant**, **not the owner**, conflict/version mismatch, event emitted to outbox |

Mutation testing runs on `domain` and `application` (nightly) to prove the
tests detect changes, not merely execute lines.

### 3.2 Integration

Run against real containers (`_shared/containers`), never against mocks of the
database.

| Folder | What it proves |
|---|---|
| `integration/persistence` | Repository ↔ ORM mapping, hand-written SQL read models, optimistic locking, `tenant_id` filtering, **row-level security** blocks cross-tenant reads even when the app filter is omitted |
| `integration/vector` | `LeadSimilarityIndex` upsert/query, tenant scoping is impossible to bypass, idempotent upsert by `leadId + contentHash`, rebuild from the database |
| `integration/queue` | Dragonfly/Redis: lease lock acquire/renew/expire, release only by token holder, stream consumer groups, `jobId` dedup, reclaim of stuck messages, DLQ after max attempts, per-tenant rate limit |
| `integration/http` | Controller + real use case + real DB through an in-process server: auth → `RequestContext` → use case → error envelope; status codes; pagination; `Idempotency-Key` behavior |
| `integration/jobs` | Job adapters call use cases as a system actor with minimum permissions and carry `tenantId` |
| `integration/events` | Outbox written in the same transaction as the state change, relay publishes, inbound handlers are idempotent |

### 3.3 Contract

| Folder | Provider / consumer | Mechanism |
|---|---|---|
| `contract/http` | Backend ↔ SDK/clients | Provider verification of responses against the OpenAPI spec; breaking-change detection (spec diff) fails the build unless a new version is created |
| `contract/events` | Producer slice ↔ consumer slices | Schema registry/JSON Schema tests; consumer-driven expectations; backward compatibility checks (additive only) |
| `contract/slice` | Slice `contracts/` package ↔ its callers | Callers compile and run against the published contract; no deep imports (also enforced by architecture tests) |
| `contract/external` | Heimdall ↔ Apify, payment, messaging providers | Recorded/consumer contracts for PR runs; **scheduled live smoke** against sandboxes to detect provider drift; 429/5xx/timeout behavior simulated |

Contract tests of a slice must pass **before** that slice is extracted into a
service (readiness checklist in `PLAYBOOK_BACKEND.md`).

### 3.4 Concurrency and idempotency (`concurrency/`)

Required for every slice with batch work. These prove "many workers, one
effect":

- Two workers receive the same message: effect happens once.
- Duplicate delivery after worker crash mid-job: resumes from checkpoint,
  no duplicate rows (unique constraint holds).
- Lease expires while a worker is paused: the stale worker's write is rejected
  by the fencing token/version check.
- Redis flushed or failed over mid-run: no corruption; sweeper re-enqueues
  from the database.
- Scheduler leader loss: exactly one new leader, no double enqueue.
- Concurrent updates to the same aggregate: optimistic lock conflict
  surfaces as `409`.

### 3.5 Migration (`migration/`)

- Apply all slice migrations **from scratch** on an empty schema.
- Apply the new migrations on a database at the **previous release** with
  representative data.
- Expand/contract: old code still works after the expand step; new code works
  before the contract step.
- Backfill jobs are resumable and idempotent.
- No migration touches another slice's schema (checked by the architecture
  suite).

### 3.6 Backend E2E (`e2e/`)

API-level flows against the deployed stack (ephemeral or staging), with
third-party providers on sandbox/stub:

- Launch campaign → `202` → run progresses → leads appear in funnels → usage
  metered.
- Admin suspends a user → their next request fails → existing sessions are
  revoked.
- Same flows executed under the `/app` and `/admin` routes with the correct
  persona each.

Keep this folder small (critical journeys only).

## 4. Frontend tests

Maps to MVVM + SDK in `PLAYBOOK_FRONTEND.md`.

### 4.1 Unit

| Folder | Scope |
|---|---|
| `unit/viewmodels` | State, derived values and commands with a fake SDK and fake ports; **every status** (`loading`, `ready`, `empty`, `forbidden`, `not-found`, `offline`, `error`); runs without DOM, native runtime or `chrome` APIs (if it needs them, a platform concern leaked) |
| `unit/sdk` | Path built per `{userType}/{module}/{resource}`, headers (`Authorization`, `X-Tenant-Id`, `X-Correlation-Id`, `Idempotency-Key`), 401 single-flight refresh, retry/backoff rules, error envelope → `SdkError` mapping |
| `unit/core` | Route builders reproduce the URL playbook exactly; permission helpers; mappers |

### 4.2 Component and integration

- `component/`: each screen View renders **every ViewModel status** from a
  stubbed ViewModel; presentational components by props. Guards
  (`RequireUserType`, `RequirePermission`) redirect to `/login`, `/403` or the
  allowed dashboard.
- `integration/`: View + real ViewModel + real SDK against MSW handlers
  generated from the OpenAPI spec (so mocks cannot drift from the contract).

### 4.3 Contract

- SDK request/response types validated against the backend OpenAPI spec in
  CI; a spec change that breaks the SDK fails the build.
- Extension message schemas (`{ type, version, payload, correlationId }`)
  between popup/side panel/content script and the service worker, including
  rejection of unknown types and untrusted senders.

### 4.4 End-to-end (`e2e/{web,native,extension}`)

| Client | Tooling (examples) | Notes |
|---|---|---|
| Web | Playwright | Per `{userType}` projects (`app`, `admin`, `partner`, `public`); verify layout, guard redirects and that a forged `/admin` URL with an `/app` token is denied by the **backend**, not only hidden |
| Native | Maestro or Detox | Deep links (`heimdall://app/funnels/leads/{leadId}`) resolve to the right screen; background/foreground token refresh |
| Extension | Playwright with the unpacked extension in a persistent context | Content-script capture → service worker → SDK; tokens never visible to the content script; MV3 service-worker restart resumes work |

### 4.5 Quality gates

- **Accessibility** (`a11y/`): automated checks (axe or equivalent) on every
  route and key native screens against WCAG 2.2 AA; keyboard/focus tests on
  critical flows.
- **Visual** (`visual/`): snapshot comparison for layouts per `{userType}` and
  the design-system components; reviewed diffs only.
- **Security** (`security/`): no tokens in `localStorage`; refresh token not
  readable from JS; CSP present; unsanitized lead HTML does not execute
  (XSS payload suite); extension manifest has only approved permissions and
  host patterns, no remote code, strict CSP; provider API keys are write-only
  in the UI.
- **Performance** (`performance/`): bundle-size budgets per `{userType}`
  (admin code not in the `/app` bundle), web vitals thresholds, list
  virtualization on large datasets.
- **i18n**: no literal user-visible strings; all message keys resolve in each
  supported locale.

## 5. Security tests (per slice and platform-wide)

Per-slice (`<slice>/backend/security`, `<slice>/frontend/security`):

- **Authorization matrix**: for each route, each `{userType}` and each role,
  the expected status. Generated from the route table so a new route without
  a matrix row fails the build.
- **Broken object-level authorization (IDOR)**: user A requests user B's
  resource id; tenant A requests tenant B's; expected `404`/`403`.
- **Prefix is not authorization**: a valid `/app` token calling the
  equivalent `/admin/...` route is denied (core rule of `PLAYBOOK_URLS.md`).
- **Permission enforcement in the use case**, not only in the controller
  (call the use case directly with a context lacking the permission).
- **Input handling**: validation, injection payloads (SQL, NoSQL, command),
  oversized bodies, unexpected content types, mass assignment.
- **Session**: expired/revoked token, suspended user, role switch, tenant
  switch invalidating cached data.
- **Secrets**: provider keys never returned by any endpoint and never present
  in logs.

Platform-wide (`_platform/security`): section 8.2.

## 6. Non-functional tests

| Suite | Location | Purpose | Cadence |
|---|---|---|---|
| Load / stress / soak | `<slice>/backend/performance`, `_platform/performance` | SLO validation per slice (latency, throughput, queue drain time); capacity evidence; the data used to decide on slice extraction | Nightly smoke, pre-release full |
| Resilience / chaos | `_platform/resilience` | Dragonfly/Redis down, DB failover, provider `429`/`5xx`, network partition between slices, worker kill; verifies degradation, backpressure, no data loss | Pre-release, quarterly game day |
| Disaster recovery | `_platform/disaster-recovery` | Backup **restore** into a clean environment, RPO/RTO measured, migrations + vector index rebuild from the database | At least semi-annual (match your policy) |
| Observability | `_platform/observability` | Every request/job log carries `correlationId`, `tenantId`, `slice`; no secrets/PII in logs; traces propagate across HTTP, queue and events; alerts fire for DLQ growth and outbox lag | Per PR (assertions) + pre-release |
| Privacy | `_platform/privacy` | PII redaction in logs/analytics, retention jobs delete on schedule, data-subject export/erasure flows, deletion cascades to vector index and caches | Per release |
| Audit log | `_platform/audit-log` | Security-relevant actions (login, role change, user suspend, key rotation, export, permission denied) produce an immutable, attributable record without sensitive values | Per PR |

## 7. End-to-end policy

### 7.1 What deserves an E2E

Only journeys where failure is business-critical or spans slices and clients:
sign-in and session, tenant onboarding, API-key configuration, launch
campaign, lead lifecycle, admin user suspension, billing close. Everything
else is covered lower in the pyramid.

### 7.2 Rules

- Deterministic: own data per test, created through the SDK/API, cleaned up
  or isolated by a unique test tenant.
- No sleeps; wait on observable conditions with a timeout.
- Third-party providers are sandboxed or stubbed; a separate scheduled suite
  checks the real sandbox.
- Flaky tests are quarantined with an owner and an expiry date (max 14 days),
  tracked as a defect, and **never** silently retried to green.

### 7.3 Cross-slice journeys (`tests/_journeys`)

Journeys that cross slices live here instead of inside one slice, for example
`launch-campaign-to-billing`. They call public APIs only (never internals) and
are owned jointly by the slices they touch.

## 8. Platform-wide suites (`tests/_platform`)

### 8.1 Architecture (`architecture/`)

Fails the build when:

- Backend layers break the dependency rules (domain imports nothing;
  application does not import adapters; no slice imports another slice's
  internals).
- A migration touches another slice's schema or a table has no `tenant_id`
  where required.
- Frontend `views` import the SDK, `viewmodels` import `react-dom`,
  `react-native` or `chrome`, or `sdk` imports React.
- A route is registered without a permission declaration and a matrix row.
- A `tests/` slice folder has no matching `src/slices` folder.

### 8.2 Security (`security/`)

| Check | Tooling (examples) | Gate |
|---|---|---|
| SAST | Semgrep, CodeQL | New high/critical findings block merge |
| SCA (dependencies) + licenses | OSV, Dependabot/Renovate, license checker | Critical/high with fix available block release; policy-defined SLAs |
| Secrets scanning | Gitleaks, TruffleHog (pre-commit **and** CI, full history periodically) | Any verified secret blocks merge |
| IaC and config | Checkov, tfsec, kube-linter | High findings block |
| Container/image | Trivy, Grype | Critical/high block release |
| DAST baseline | OWASP ZAP against staging | Weekly + pre-release; findings triaged with SLA |
| SBOM + provenance | CycloneDX/SPDX, signed build attestations | Generated and stored for every release |
| Manual penetration test | Independent third party | At least annually and after major architecture change |

Vulnerability remediation SLAs (for example by severity) are set in the
vulnerability-management policy; the pipeline enforces whatever that policy
says.

### 8.3 Authorization matrix and tenant isolation

- `authz-matrix/`: the full table `route × {userType} × role × expected
  status`, generated from the route table and the permission catalog
  (`module.resource.action`).
- `tenant-isolation/`: with two seeded tenants, **every** list/detail/mutation
  endpoint, query, job, vector search and cache key is exercised from tenant A
  against tenant B data. Includes background jobs and events (a job carrying
  tenant A must never write tenant B).

## 9. Environments, data and pipeline

### 9.1 Environments

| Environment | Used for | Data |
|---|---|---|
| Local / CI ephemeral | Unit, integration, contract, component | Containers, synthetic |
| Ephemeral stack per PR (optional) | E2E, DAST | Synthetic, destroyed after |
| Staging | Pre-release E2E, load, resilience, DR rehearsal | Synthetic, prod-like volume |
| Production | Synthetic monitoring and smoke only | Dedicated, flagged test tenant; no destructive tests |

Production, staging and test environments are separated: distinct accounts or
projects, distinct credentials, no network path from test to production data.

### 9.2 Test data

- **Never use production data**, and never copy it down. Use builders and
  synthetic generators (`_shared/builders`).
- If masked production-shaped data is ever unavoidable, it needs documented
  approval, irreversible masking, restricted access and a deletion date.
- Test secrets are generated per environment, short-lived and stored in the
  secret manager; none in the repository.
- Fixtures contain no real personal data; leads are synthetic.

### 9.3 Pipeline stages

| Stage | Trigger | Runs |
|---|---|---|
| Pre-commit | Developer machine | Lint, format, secrets scan, affected unit tests |
| Pull request | Every PR | Unit, component, architecture, contract, integration for **affected slices and their dependents**, SAST, SCA, secrets, authz-matrix, a11y, bundle budgets |
| Main | Merge | Full integration, backend/frontend E2E smoke, container and IaC scan, SBOM |
| Nightly | Schedule | Full E2E, load smoke, mutation testing, tenant-isolation full sweep, external-provider live smoke, DAST baseline (staging) |
| Release candidate | Tag | Full E2E (all clients), performance regression, migration from previous release, resilience suite, evidence bundle (section 10.4) |
| Periodic | Calendar | Penetration test, DR restore test, chaos game day, access-review verification, evidence-completeness review |

Path filters decide which slices run on a PR, but a change to a slice's
`contracts/` always runs the contract tests of every dependent slice.

### 9.4 Thresholds (starting points, tune with data)

| Area | Threshold |
|---|---|
| Domain + application line/branch coverage | ≥ 90% / ≥ 85%, plus a mutation score target set per slice |
| Adapters | Covered by integration tests; coverage informational, not a gate |
| ViewModels | ≥ 85% |
| New code | Coverage must not decrease; changed lines covered |
| Flaky rate | < 1% of runs; above that the suite is paused for repair |
| PR pipeline duration | Target ≤ 15 minutes; nightly absorbs the slow suites |

Coverage is a floor for visibility, never the goal; the authorization,
isolation and idempotency suites are gated on **presence and pass**, not on
percentages.

## 10. Compliance evidence (SOC 2 and ISO/IEC 27001)

### 10.1 Control-to-test mapping (starting point)

SOC 2 references use the 2017 Trust Services Criteria; ISO references use
ISO/IEC 27001:2022 Annex A. Confirm the final mapping with your auditor.

| Control objective | Tests / evidence | SOC 2 | ISO 27001:2022 |
|---|---|---|---|
| Logical access and least privilege | Authz matrix, IDOR, prefix-is-not-authorization, role/permission tests, session revocation | CC6.1–CC6.3 | A.5.15, A.5.18, A.8.2, A.8.3 |
| Authentication | Token expiry/refresh/revocation, suspended-user denial, extension token isolation | CC6.1 | A.8.5 |
| Tenant segregation (confidentiality) | Tenant-isolation sweep, RLS tests, vector tenant scoping | CC6.1, C1.1 | A.8.3, A.8.22 |
| Cryptography and secrets | Secrets scanning, no secrets in logs/responses/bundle, secure storage tests | CC6.1, CC6.7, C1.1 | A.8.24, A.8.12 |
| Secure development and testing | This playbook's pipeline, SAST, architecture tests, peer review gates | CC8.1 | A.8.25, A.8.26, A.8.28, A.8.29 |
| Change management | Required checks on protected branches, PR-to-ticket traceability, release evidence bundle | CC8.1 | A.8.32 |
| Separation of environments | Environment isolation checks, no prod data in tests | CC6.1, CC8.1 | A.8.31 |
| Protection of test information | Synthetic-data checks, secrets in test envs | C1.1 | A.8.33 |
| Vulnerability management | SCA, container, IaC, DAST, pen test, SLA tracking | CC7.1 | A.8.8, A.8.9 |
| Logging and monitoring | Audit-log tests, observability assertions, alert tests | CC7.2, CC7.3 | A.8.15, A.8.16 |
| Incident detection/response readiness | Alert firing tests, runbook-linked game days | CC7.3–CC7.5 | A.5.24–A.5.28 |
| Availability and recovery | DR restore test, resilience/chaos, load against SLOs | A1.1–A1.3, CC9.1 | A.5.30, A.8.13, A.8.14 |
| Processing integrity | Idempotency, concurrency, outbox, migration and contract tests | PI1.1–PI1.5 | A.8.26, A.8.27 |
| Privacy and data lifecycle | PII redaction, retention deletion, export/erasure, deletion cascades | P-series, C1.2 | A.5.34, A.8.10, A.8.11 |
| Supply chain | SBOM, provenance, license and dependency policies | CC9.2 | A.5.19–A.5.22, A.8.30 |
| Accessibility (customer commitments, if stated) | a11y suite | CC2.3 (if committed) | — |

### 10.2 Tagging tests to controls

Every control-relevant test carries machine-readable tags in its metadata or
name, for example:

```text
@control SOC2:CC6.1  @control ISO27001:A.8.3
@risk tenant-isolation
@slice funnels  @side backend  @type integration
```

The `compliance/` collector reads the test report plus the tags and generates
the **traceability matrix**: control → tests → last result → artifact link. A
control without a passing, recent test appears as a gap in the report.

### 10.3 Evidence requirements

Evidence is only useful if it is complete, attributable and tamper-evident:

- Each pipeline run stores: JUnit/xUnit results, coverage, SARIF (SAST/SCA),
  DAST report, SBOM, build provenance, commit SHA, who approved, and the
  pipeline definition version.
- Stored in an **append-only / write-once** location with hashes, access
  restricted and logged, retained for at least the audit window your
  frameworks require (SOC 2 Type II observation periods are typically 3–12
  months; set retention in policy, with a margin).
- Manual or periodic activities (pen test, DR restore, access review, game
  day) are recorded as dated, signed reports in the same evidence store with
  a ticket link.
- Timestamps come from the CI system, not from the test code.
- Evidence contains no secrets or personal data.

### 10.4 Release evidence bundle

For each release the pipeline produces `evidence/{release}/`:

```text
manifest.json                 # commit, version, pipeline id, approver(s), hashes
test-results/                 # per slice, side and type
coverage/
security/                     # SARIF, DAST, container, IaC, SBOM, provenance
traceability-matrix.html/csv  # control → tests → result
change-record.link            # ticket, review, approvals
exceptions.md                 # waived findings with owner, justification, expiry
```

### 10.5 Change management gates

- Protected main branch; direct pushes disabled; at least one reviewer who is
  not the author; code-owner review for the touched slice and for `_platform`.
- Required status checks (section 9.3 PR stage) cannot be bypassed, including
  by administrators, except through the emergency path.
- **Emergency change**: allowed only with a recorded justification and
  approver; the full suite runs afterwards and the exception is logged in
  `exceptions.md`.
- Every PR links to a work item; the release links to the set of merged PRs.
- Separation of duties: the person who approves a release is not the sole
  author of its changes.

### 10.6 Exceptions and waivers

A failing security or compliance gate can be waived only with: owner,
justification, compensating control, expiry date (default ≤ 30 days) and
approver outside the owning team. Expired waivers fail the build.

### 10.7 Periodic control testing

| Activity | Frequency (set in policy) | Evidence |
|---|---|---|
| Backup restore and DR rehearsal | At least semi-annual | Report with measured RPO/RTO |
| Independent penetration test | Annual and on major change | Report, remediation tickets |
| Access-review verification | Quarterly | Reviewed permission export vs. authz matrix |
| Chaos / game day | Quarterly | Report and actions |
| Full-history secrets scan | Monthly | Report |
| Dependency and base-image refresh | Continuous, reviewed monthly | Update PRs, SLA dashboard |
| Review of this playbook and control map | Annual | Versioned change log |

## 11. Concrete example for Heimdall: updating a lead's stage

Feature: `PATCH /api/v1/app/funnels/leads/{leadId}` (permission
`funnels.leads.update`).

```text
tests/funnels/backend/unit/domain/model/lead.unit            # stage transition rules
tests/funnels/backend/unit/application/move-lead-stage.unit  # permission denied, wrong tenant,
                                                             # not owner, version conflict, outbox event
tests/funnels/backend/integration/persistence/lead-repo.integration   # optimistic lock, RLS
tests/funnels/backend/integration/http/lead-update.integration        # envelope, 403/404/409
tests/funnels/backend/contract/http/leads.contract                    # OpenAPI conformance
tests/funnels/backend/contract/events/lead-stage-changed.contract     # event schema v1
tests/funnels/backend/security/lead-idor.security                     # tenant A vs tenant B
tests/funnels/frontend/unit/viewmodels/use-lead-detail-vm.unit        # forbidden/offline/ready
tests/funnels/frontend/unit/sdk/leads.unit                            # path + headers + errors
tests/funnels/frontend/component/lead-detail-view.component           # every status rendered
tests/funnels/frontend/e2e/web/lead-stage.e2e                         # /app route + guard
tests/funnels/frontend/e2e/extension/capture-lead.e2e                 # content script → worker → SDK
tests/_platform/authz-matrix/                                         # row for this route
tests/_platform/tenant-isolation/                                     # sweep includes this endpoint
```

Representative tagged test:

```text
name:  rejects_stage_change_for_user_without_permission
tags:  @control SOC2:CC6.1  @control ISO27001:A.5.15  @risk authz
type:  application unit + http integration (403 with requiredPermission)
evidence: JUnit entry linked in traceability-matrix for control CC6.1
```

Campaign-launch flow adds: `queue` and `concurrency` suites in `extraction`
(duplicate delivery, lease expiry, Redis flush), a `contract/external` suite
for Apify with a nightly live-sandbox smoke, and the
`_journeys/launch-campaign-to-billing` end-to-end.

## 12. Checklist

- [ ] Tests live under `tests/{slice}/{backend|frontend}/{type}`; cross-cutting
      suites under `_platform`, cross-slice journeys under `_journeys`.
- [ ] Slice folder names match `src/slices` and the URL `{module}`; CODEOWNERS
      set per slice.
- [ ] Every assertion sits at the lowest level that can prove it.
- [ ] Backend: unit (domain, application), integration (persistence, vector,
      queue, http, jobs, events), contract (http, events, slice, external),
      concurrency, migration, security and a thin E2E exist for each slice
      that needs them.
- [ ] Frontend: unit (viewmodels, sdk, core), component, integration, contract,
      E2E per client (web, native, extension), a11y, visual, security,
      performance.
- [ ] Integration tests use real containers; no mocked database.
- [ ] Authorization matrix is generated from the route table; a route without
      a row fails the build.
- [ ] Tenant-isolation sweep covers endpoints, jobs, events, vector search and
      cache keys.
- [ ] Idempotency and concurrency tests prove one effect under duplicate
      delivery, crash and lease expiry.
- [ ] Architecture tests enforce backend and frontend dependency rules.
- [ ] SAST, SCA, secrets, IaC, container, DAST and SBOM run on the cadence in
      section 9.3; pen test at least annually.
- [ ] DR restore, chaos and access-review verification run on schedule with
      dated evidence.
- [ ] No production data in tests; environments and credentials are separated.
- [ ] Control-relevant tests are tagged; the traceability matrix is generated
      and gaps are reviewed.
- [ ] Evidence is retained in append-only storage for the required period and
      contains no secrets or personal data.
- [ ] Protected branches, required checks, independent review and linked work
      items are enforced; emergency changes and waivers are recorded with
      expiry.
- [ ] Flaky tests are quarantined with owner and expiry, never auto-retried to
      green.
- [ ] Control mapping and this playbook are reviewed with the auditor at least
      annually.