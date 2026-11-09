# PLAYBOOK_TESTS.md — Testing & compliance-evidence playbook (Go backend)

Testing contract for a Go backend built from this kit. It follows the same
**vertical slicing** as `PLAYBOOK_BACKEND.md`, uses the route and permission
model of `PLAYBOOK_URLS.md`, and is designed so that the test suite doubles as
**audit evidence for SOC 2 and ISO/IEC 27001**.

The rules that carry the whole file:

1. **Tests live next to the code they test.** Backend tests are `*_test.go`
   inside the slice. The path tells you what is covered without a mirror tree
   to keep in sync.
2. **Test where the risk is.** Business rules at the cheapest level (unit),
   wiring against real infrastructure (integration), agreements between
   parties (contract), and only the critical journeys end to end.
3. **Authorization, ownership isolation, idempotency and privacy are
   first-class test targets**, not afterthoughts. They are what this product
   can least afford to get wrong.
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
internal/features/<slice>/            # colocated unit + use-case tests
  domain_test.go   usecase_test.go   http_test.go
  postgres_test.go                      # build tag `integration`
  fakes_test.go                         # in-package fakes for the ports
internal/platform/<pkg>/*_test.go       # platform tests, colocated
test/
  acceptance/                           # godog over the real mux with fakes
    app.go  fakes.go  steps_*.go  census_test.go
  integration/                          # testcontainers Postgres, build tag
  testutil/                             # shared harness (no assertions)
specs/features/*.feature                # the behavioral contracts godog runs
evals/                                  # separate Go module: contract evals
  contracts_test.go  fixtures/{valid,invalid}  run-live.sh
```

Rules:

- A test file's location is its scope. There is no `tests/<slice>/...` mirror
  tree; do not create one.
- Cross-slice backend journeys live in `test/acceptance/` (godog).
- `test/testutil` and `test/acceptance/fakes.go` hold support code only.
- Slice ownership follows `internal/features/<slice>/` in CODEOWNERS.

### 1.1 Naming

- Go: `<topic>_test.go` in the package; table-driven subtests named as
  behavior (`t.Run("rejects update when role is not admin")`). In-package
  fakes in `fakes_test.go`.
- Test names describe behavior, not implementation:
  `rejects the update when the caller does not own the row`.
- Every test that supports a control carries a tag (section 10.2).
- Test IDs are stable (`TST-ADMINUSERS-BE-UC-0042`) so audit evidence can
  reference them across releases.

## 2. Test types and the pyramid

| Type | Proves | Real vs. fake | Speed | Share of suite |
|---|---|---|---|---|
| **Unit** | Business rules | Ports faked in-package | ms | Most |
| **Integration** | Adapters work against real infrastructure | Real Postgres (testcontainers); external APIs stubbed | seconds | Many |
| **Contract** | Two parties agree on an interface | Each side tested alone against a shared contract | seconds | Moderate |
| **End-to-end (E2E)** | A critical user journey works across the deployed stack | Everything real except third-party providers | minutes | Few |
| **Security** | Authorization, ownership isolation, input handling, supply chain | Mixed | seconds–minutes | Targeted and continuous |
| **Non-functional** | Performance, resilience, recovery | Prod-like environment | minutes–hours | Scheduled |

Placement rule: **push each assertion to the lowest level that can prove it.**
A role rule is a unit test of the use case; that the route enforces it is one
handler test; that the real token flows through the stack is one E2E check. Do
not repeat the same assertion at three levels.

## 3. Backend tests

Maps one-to-one to the slice contract in `PLAYBOOK_BACKEND.md`.

### 3.1 Unit

| File | Scope | Rules |
|---|---|---|
| `domain_test.go` | Invariants, value objects, pure helpers, typed errors | No mocks, no I/O; deterministic (`clockx`, `idgen` injected) |
| `usecase_test.go` | Use cases with in-package fakes of every port (`fakes_test.go`) | Covers: happy path, validation, **role denied**, **not the owner**, not found, conflict/version mismatch |
| `http_test.go` | Handler: decode, status codes, `{detail, code}` envelope | Uses `httptest`; no database |

Mutation testing runs on `domain.go` + `usecase.go` nightly to prove the tests
detect changes, not merely execute lines.

### 3.2 Integration

Run against real infrastructure, never against mocks of the database.

| Suite | What it proves |
|---|---|
| `postgres_test.go` (`-tags=integration`) | `postgres.go` ↔ sqlc mapping, real queries, migrations applied from scratch, constraints (unique, FK) |
| `test/integration/integration_test.go` | Cross-slice wiring, pool bootstrap, `readyz` |
| `test/acceptance/` | godog over the real mux with fakes: routes, envelopes, pagination, role gates |
| `concurrency` cases inside the slice | Two workers claim one run: effect once; duplicate delivery resumes from checkpoint; terminal rows are never re-claimed |

### 3.3 Contract

| Suite | Provider / consumer | Mechanism |
|---|---|---|
| `make gen-check` | `specs/openapi.yaml` ↔ `internal/gen/api` ↔ client types | Generated code must be reproduced byte-for-byte; a spec change that is not regenerated fails the build |
| `evals/contracts_test.go` | Backend ↔ client/agent contracts | Valid vs. invalid fixtures; offline |
| `specs/features/*.feature` | Route behavior ↔ product intent | godog acceptance suite (`make accept`) |
| `make vuln` | Dependency graph ↔ known CVEs | `govulncheck` |

A slice's contract suite must pass **before** that slice is extracted into a
service (readiness checklist in `PLAYBOOK_BACKEND.md`).

### 3.4 Concurrency and idempotency

Required for every slice with a worker (the email outbox in `cmd/worker`, the
janitor loops, and any slice worker added later). These prove "many workers,
one effect":

- Two workers claim the same `pending` row: the effect happens once.
- Duplicate delivery after a worker crash mid-run: resumes from the checkpoint,
  no duplicate rows (unique constraint holds).
- A stale worker whose run was re-claimed cannot overwrite newer state (status
  transition guard).
- Process restart mid-run: `running` rows are discovered from Postgres and
  resumed; nothing is lost when the in-process hub is empty.
- Concurrent updates to the same aggregate: conflict surfaces as `409`.

### 3.5 Migration

- Apply all migrations **from scratch** on an empty database (`make migrate`).
- Apply the new migration on a database at the **previous release** with
  representative data.
- Expand/contract: old code still works after the expand step; new code works
  before the contract step.
- Backfill workers are resumable and idempotent.
- `make migrate-down` works for the new migration, or the file documents why
  it is irreversible.

### 3.6 Backend E2E

API-level flows against the deployed stack (ephemeral or staging), with
third-party providers on sandbox/stub:

- Signup → verification email (console mailer) → login → refresh → logout.
- A user creates a widget → sees it in their list → a second user does not.
- Admin suspends a user → their next request fails → sessions revoked.

Keep this thin; godog acceptance is the default home for route-level behavior.

## 4. Security tests (per slice and platform-wide)

Per slice (`internal/features/<slice>/*_test.go`):

- **Role matrix**: for each admin route and each role, the expected outcome.
  The `adminusers` tests assert every action the admin surface declares; a new
  action without a backend check fails review.
- **Broken object-level authorization (IDOR)**: user A requests user B's
  resource id; expected `404`/`403`. There are no tenants; ownership is
  `user_id`.
- **Prefix is not authorization**: a valid non-admin token calling
  `/api/v1/admin/...` is denied by `authx.RequireRole`, and the use case
  re-checks.
- **Authorization in the use case**, not only in the handler (call the use case
  directly with the wrong actor).
- **Input handling**: validation, injection payloads, oversized bodies,
  unexpected content types, mass assignment.
- **Session**: expired/revoked token, suspended user, refresh-token reuse,
  logout revoking the session.
- **Secrets**: provider keys never returned by any endpoint and never present
  in logs.

Platform-wide (`make lint`, `make vuln`): section 8.2.

## 5. Non-functional tests

| Suite | Location | Purpose | Cadence |
|---|---|---|---|
| Performance (backend) | `-tags=perf`, `make test-perf` | Absolute budgets, serial on a quiet host | Pre-release |
| Resilience / chaos | Deploy-level exercise | Database failover, provider `429`/`5xx`, worker kill; verifies degradation and no data loss | Pre-release |
| Disaster recovery | Deploy-level exercise | Backup **restore** into a clean environment, RPO/RTO measured, migrations replayed | At least semi-annual |
| Observability | Log assertions in slice tests | Logs carry the actor, slice and run id; no secrets/PII in logs | Per PR |
| Privacy | `make ci` + review | PII redaction in logs, retention jobs delete on schedule, data-subject export/erasure flows | Per release |
| Audit log | `adminusers` tests | Security-relevant actions (login, role change, permission denied) produce an attributable record without sensitive values | Per PR |

## 6. End-to-end policy

### 6.1 What deserves an E2E

Only journeys where failure is business-critical: sign-in and session handoff,
signup + verification, password reset, admin user suspension, admin role
change. Everything else is covered lower in the pyramid.

### 6.2 Rules

- Deterministic: own data per test, created through the API, and isolated by a
  unique user.
- No sleeps; wait on observable conditions with a timeout.
- Third-party providers are stubbed; a separate scheduled suite checks the
  real sandbox.
- Flaky tests are quarantined with an owner and an expiry date (max 14 days),
  tracked as a defect, and **never** silently retried to green.

## 7. Platform-wide suites

There is no `tests/_platform` tree. Enforcement lives in the tools that
already run in the gates.

### 7.1 Architecture (`make lint`)

Fails the build when:

- `domain.go` imports `net/http`, `pgx`, `database/sql`, `internal/gen`,
  `internal/platform` or another slice (`domain-pure`).
- `usecase.go` / `ports.go` name a transport or adapter
  (`usecase-transport-free`).
- A slice imports another slice (`slice-isolation`), or
  `internal/platform` imports a feature (`platform-no-features`).
- `make gen-check` finds stale generated code.
- `go mod tidy` would change `go.mod`/`go.sum` (INV-T3 hygiene).

All GitHub Actions workflows live in `.github/workflows/` at the repository
root.

### 7.2 Security

| Check | Tooling | Gate |
|---|---|---|
| SAST | `golangci-lint` (standard set + depguard) | `make lint`, inside CI; a new finding blocks merge |
| SCA (dependencies) | `make vuln` (`govulncheck`) | Release step: critical/high with a fix available block release |
| Secrets scanning | Not configured in the skeleton | **Known gap:** required before release; any verified secret blocks merge once wired |
| Template/leak guard | `scripts/leak-guard.sh` flags leftover template tokens | Manual/review step; wire into CI per project |
| Container/image | Image is built (`Dockerfile`) but there is no scan stage | **Known gap:** critical/high block release once the stage is wired |
| Manual penetration test | Independent third party | At least annually and after major architecture change |

Current state (2026-11-08): the skeleton wires SAST through the `backend` job
of `.github/workflows/ci.yml`. `make vuln` is a release-step command, not part
of `make ci`. Secrets scanning, the leak guard and container scanning have no
gate yet; do not cite them as audit evidence until they run in CI (AGENTS.md
rule 3).

Vulnerability remediation SLAs are set in the vulnerability-management policy;
the pipeline enforces whatever that policy says.

### 7.3 Role matrix and ownership isolation

- The backend has a role test per admin route, and each declared admin action
  has a matching backend authorization test.
- **Ownership isolation**: with two seeded users, every list/detail/mutation
  endpoint that returns user data is exercised from user A against user B's
  ids. Includes workers and streams (a run owned by A must never be delivered
  to B). There are no tenants.

## 8. Environments, data and pipeline

### 8.1 Environments

| Environment | Used for | Data |
|---|---|---|
| Local / CI ephemeral | Unit, integration, contract | Containers, synthetic |
| Ephemeral stack per PR (optional) | E2E, DAST | Synthetic, destroyed after |
| Staging | Pre-release E2E, load, resilience, DR rehearsal | Synthetic, prod-like volume |
| Production | Synthetic monitoring and smoke only | Dedicated, flagged test account; no destructive tests |

Production, staging and test environments are separated: distinct accounts or
projects, distinct credentials, no network path from test to production data.

### 8.2 Test data

- **Never use production data**, and never copy it down. Use builders and
  synthetic fixtures (`test/testutil`).
- If masked production-shaped data is ever unavoidable, it needs documented
  approval, irreversible masking, restricted access and a deletion date.
- Test secrets are generated per environment, short-lived and stored in the
  secret manager; none in the repository.
- Fixtures contain no real personal data; seed users and widgets are
  synthetic.

### 8.3 Pipeline stages

| Stage | Trigger | Runs |
|---|---|---|
| Pre-commit | Developer machine | Lint, format, secrets scan, affected unit tests |
| Pull request | Every PR | Unit, architecture, contract, integration for **affected slices and their dependents**, SAST, SCA, secrets, role matrix |
| Main | Merge | Full integration, godog acceptance, container scan |
| Nightly | Schedule | Full E2E, perf smoke, mutation testing, ownership-isolation sweep |
| Release candidate | Tag | Full E2E, performance regression, migration from previous release, evidence bundle (section 9.4) |
| Periodic | Calendar | Penetration test, DR restore test, access-review verification, evidence-completeness review |

Path filters decide which slices run on a PR, but a change to a shared port or
to `specs/openapi.yaml` always runs the contract tests of every dependent
slice.

### 8.4 Thresholds (starting points, tune with data)

| Area | Threshold |
|---|---|
| `domain.go` + `usecase.go` line/branch coverage | ≥ 90% / ≥ 85%, plus a mutation score target set per slice |
| Adapters | Covered by integration tests; coverage informational, not a gate |
| New code | Coverage must not decrease; changed lines covered |
| Flaky rate | < 1% of runs; above that the suite is paused for repair |
| PR pipeline duration | Target ≤ 15 minutes; nightly absorbs the slow suites |

Coverage is a floor for visibility, never the goal; the authorization,
isolation and idempotency suites are gated on **presence and pass**, not on
percentages.

## 9. Compliance evidence (SOC 2 and ISO/IEC 27001)

### 9.1 Control-to-test mapping (starting point)

SOC 2 references use the 2017 Trust Services Criteria; ISO references use
ISO/IEC 27001:2022 Annex A. Confirm the final mapping with your auditor.

| Control objective | Tests / evidence | SOC 2 | ISO 27001:2022 |
|---|---|---|---|
| Logical access and least privilege | Authz matrix, IDOR, prefix-is-not-authorization, role/permission tests, session revocation | CC6.1–CC6.3 | A.5.15, A.5.18, A.8.2, A.8.3 |
| Authentication | Token expiry/refresh/revocation, suspended-user denial | CC6.1 | A.8.5 |
| Ownership segregation (confidentiality) | Two-user isolation sweep over detail, list, mutation, worker and stream paths | CC6.1, C1.1 | A.8.3, A.8.22 |
| Cryptography and secrets | Secrets scanning, no secrets in logs/responses, secure storage tests | CC6.1, CC6.7, C1.1 | A.8.24, A.8.12 |
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

### 9.2 Tagging tests to controls

Every control-relevant test carries machine-readable tags in its metadata or
name, for example:

```text
@control SOC2:CC6.1  @control ISO27001:A.8.3
@risk ownership-isolation
@slice adminusers  @type usecase
```

The `compliance/` collector reads the test report plus the tags and generates
the **traceability matrix**: control → tests → last result → artifact link. A
control without a passing, recent test appears as a gap in the report.

### 9.3 Evidence requirements

Evidence is only useful if it is complete, attributable and tamper-evident:

- Each pipeline run stores: JUnit/xUnit results, coverage, SARIF (SAST/SCA),
  SBOM, build provenance, commit SHA, who approved, and the pipeline
  definition version.
- Stored in an **append-only / write-once** location with hashes, access
  restricted and logged, retained for at least the audit window your
  frameworks require (SOC 2 Type II observation periods are typically 3–12
  months; set retention in policy, with a margin).
- Manual or periodic activities (pen test, DR restore, access review, game
  day) are recorded as dated, signed reports in the same evidence store with
  a ticket link.
- Timestamps come from the CI system, not from the test code.
- Evidence contains no secrets or personal data.

### 9.4 Release evidence bundle

For each release the pipeline produces `evidence/{release}/`:

```text
manifest.json                 # commit, version, pipeline id, approver(s), hashes
test-results/                 # per slice and type
coverage/
security/                     # SARIF, container, SBOM, provenance
traceability-matrix.html/csv  # control → tests → result
change-record.link            # ticket, review, approvals
exceptions.md                 # waived findings with owner, justification, expiry
```

### 9.5 Change management gates

- Protected main branch; direct pushes disabled; at least one reviewer who is
  not the author; code-owner review for the touched slice and for shared
  packages.
- Required status checks (section 8.3 PR stage) cannot be bypassed, including
  by administrators, except through the emergency path.
- **Emergency change**: allowed only with a recorded justification and
  approver; the full suite runs afterwards and the exception is logged in
  `exceptions.md`.
- Every PR links to a work item; the release links to the set of merged PRs.
- Separation of duties: the person who approves a release is not the sole
  author of its changes.

### 9.6 Exceptions and waivers

A failing security or compliance gate can be waived only with: owner,
justification, compensating control, expiry date (default ≤ 30 days) and
approver outside the owning team. Expired waivers fail the build.

### 9.7 Periodic control testing

| Activity | Frequency (set in policy) | Evidence |
|---|---|---|
| Backup restore and DR rehearsal | At least semi-annual | Report with measured RPO/RTO |
| Independent penetration test | Annual and on major change | Report, remediation tickets |
| Access-review verification | Quarterly | Reviewed permission export vs. authz matrix |
| Chaos / game day | Quarterly | Report and actions |
| Full-history secrets scan | Monthly | Report |
| Dependency and base-image refresh | Continuous, reviewed monthly | Update PRs, SLA dashboard |
| Review of this playbook and control map | Annual | Versioned change log |

## 10. Concrete example: updating a user as an admin

Feature: `PATCH /api/v1/admin/users/{id}` (role `ADMIN`).

```text
internal/features/adminusers/domain_test.go      # patch validation
internal/features/adminusers/usecase_test.go     # role denied, not found, audit written
internal/features/adminusers/http_test.go        # 401/403/404/200, envelope, pagination bounds
internal/features/adminusers/postgres_test.go    # -tags=integration: update + audit row
test/acceptance/steps_admin_users.go             # godog journey over the real mux
specs/features/admin-users.feature               # the behavioral contract
```

Representative tagged test:

```text
name:  rejects update when the caller is not an admin
tags:  @control SOC2:CC6.1  @control ISO27001:A.5.15  @risk authz
type:  usecase unit + http integration (403 {detail, code})
evidence: JUnit entry linked in traceability-matrix for control CC6.1
```

## 11. Checklist

- [ ] Tests colocate with the code: `*_test.go` in the slice.
- [ ] Slice directories match `internal/features/**`; CODEOWNERS set per slice.
- [ ] Every assertion sits at the lowest level that can prove it.
- [ ] Backend: domain, usecase, handler, integration (`-tags=integration`),
      acceptance (godog) and a thin API E2E exist where the slice needs them.
- [ ] Integration tests use real Postgres; no mocked database.
- [ ] Role matrix has a test per declared admin action; a new action without a
      backend check fails review.
- [ ] Ownership-isolation sweep covers detail, list, mutation, worker and
      stream paths (two users, no tenants).
- [ ] Idempotency and concurrency tests prove one effect under duplicate
      claim, crash and restart.
- [ ] depguard enforces the architecture rules; `make gen-check` is green.
- [ ] SAST, SCA and secrets scanning run on the cadence in section 8.3; pen
      test at least annually.
- [ ] DR restore and access-review verification run on schedule with dated
      evidence.
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
