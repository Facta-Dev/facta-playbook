# PLAYBOOK_TESTS.md — testing and compliance evidence (Rust backend)

Testing contract for `<app>`. It follows the crate boundaries of
`PLAYBOOK_BACKEND.md`, uses the route and permission model of
`PLAYBOOK_URLS.md`, and is built so the suite doubles as **SOC 2 / ISO/IEC
27001 evidence** instead of a separate compliance exercise.

The rules that carry the whole file:

1. **Tests live in the crate they test**, as a `#[cfg(test)] mod tests` at the
   bottom of the file. No mirror `tests/` tree, no separate test crate for
   production code.
2. **Test where the risk is.** Business rules against mocks (unit), the
   transport against a built router (handler), agreements against the
   generated spec (contract). Real-database and end-to-end tiers are added by
   the product, not the template — see §7.
3. **Authorization, ownership isolation, token lifecycle and idempotency are
   first-class targets**, not afterthoughts.
4. **A control that is not tested automatically is not evidenced.** Every
   control-relevant test is labeled, and the artifact is retained.
5. **The gate never weakens.** `just ci` is fail-closed; there is no silent
   skip and no ignored test in the tree.

> Scope note: a green suite produces evidence; it does not by itself make the
> company compliant. Policies, reviews and operational processes are also
> required. Confirm the control map with your auditor and your Statement of
> Applicability.

---

## 1. Layout (what actually ships)

```text
crates/<app>-core/src/*.rs          # #[cfg(test)] mod tests — pure invariants
crates/<app>-ports/src/**           # mock-behaviour tests (the in-memory doubles)
crates/<app>-<slice>/src/*.rs       # service tests against ports mocks
crates/<app>-api/src/router.rs      # handler + route-table tests (tower::ServiceExt)
crates/<app>-api/src/error.rs       # envelope mapping tests
crates/<app>-warden/src/*.rs        # JWT, password, token, cookie tests
crates/<app>-loom/src/*.rs          # hub, outbox, janitor tests
crates/<app>-mailer/src/*.rs        # template + sender-selection tests
evals/                              # standalone package, NOT a workspace member
  src/lib.rs                        # .claude frontmatter/section validator
  tests/contracts.rs                # valid + invalid fixture pairs
  fixtures/                         # valid/ and invalid/ markdown
  run-live.sh                       # live-tier smoke (API + real provider)
```

Facts about the shipped suite (verify with
`cargo nextest list --workspace`):

| Crate | Tests | Level |
|---|---|---|
| `<app>-api` | 81 | handlers, router, error mapping, metrics, request id |
| `<app>-warden` | 63 | JWT codec, argon2, opaque tokens, cookies, OAuth |
| `<app>-auth` | 28 | signup/session/password/oauth services |
| `<app>-loom` | 21 | hub fan-out, outbox claim/retry, janitor |
| `<app>-ports` | 17 | in-memory mock semantics (CAS, leases, paging) |
| `<app>-mailer` | 14 | sender selection, templates, offline guard |
| `<app>-core` | 10 | roles, `can()`, domain invariants |
| `<app>-adminusers` | 10 | roster, rank gate, audit write |
| `<app>-widgets` | 5 | example slice |
| `<app>-sdk`, `<app>-cli` | 0 | **gap** — see §7.3 |

249 tests, all inline, no `#[ignore]`, no `tests/` directories.

### 1.1 Naming

- Rust naming: `fn rejects_update_when_caller_is_not_admin()` — snake_case,
  behavior first, no `test_` prefix.
- One assertion theme per test; a table (`for (input, expected) in [...]`)
  when the behavior is the same across cases.
- Async tests use `#[tokio::test]` (current-thread runtime is enough unless the
  test exercises concurrency across tasks; then `#[tokio::test(flavor =
  "multi_thread")]`).
- Every control-relevant test carries a label comment (see §9.2).

---

## 2. Test types and the pyramid

| Type | Proves | Real vs. fake | Speed | Shipped |
|---|---|---|---|---|
| **Unit** | domain invariants, service rules | ports mocks | ms | yes, the bulk |
| **Handler** | routing, extractors, status codes, envelope | built `Router` + mock `AppState` | ms | yes |
| **Contract** | the spec matches the code; the `.claude` layer matches its schema | generated doc + fixtures | ms | yes (`gen-check`, `evals`) |
| **Integration (DB)** | `Pg*` adapters, migrations, constraints | real Postgres | seconds | **no** — product adds it |
| **E2E** | a full journey on a deployed stack | everything real but providers | minutes | **no** — `run-live.sh` is the seam |

Placement rule: **push each assertion to the lowest level that can prove it.**
A rank rule is a service test; that the route enforces it is one handler test;
that a real token flows through the stack is one E2E check. Do not repeat the
same assertion at three levels.

---

## 3. Backend tests, by crate layer

### 3.1 `<app>-core` — pure invariants

- No async, no mocks, no I/O. Deterministic by construction.
- Role ordering, `can(role, action)`, the DB-role→domain mapping, value-object
  validation, error→status mapping.
- A failure here is a design failure: fix the type, not the test.

### 3.2 Slice services — rules against ports mocks

Use the mocks from `<app>-ports` (feature `mock`); never a live database and
never a hand-written double in the slice crate unless the port is local to it.

Required cases for every use case:

- happy path;
- validation rejection (blank, oversized, wrong enum);
- **role denied** — call the service directly with the wrong actor;
- **not the owner** — user A against user B's id;
- not found;
- conflict (`409`), including the duplicate-email and duplicate-widget cases;
- state-transition guards (cannot update a deleted row, cannot reuse a
  consumed code).

### 3.3 `<app>-api` — handlers and router

- Build the router with `build(test_state())` and drive it with
  `tower::ServiceExt::oneshot`; assert the status and the JSON envelope.
- Cover: missing token → 401, bad token → 401, valid-but-disallowed → 403,
  foreign id → 404, validation → 400, success → 200/201/204.
- Extractor order and `deny_unknown_fields` behavior (an unknown JSON field is
  a 400, not silently dropped).
- The route-table test (`go_route_table_parity`) asserts the Go-parity surface
  is served and every non-legacy route is in `ApiDoc`. A new route without a
  `#[utoipa::path]` entry fails it.
- `openapi_spec_is_non_empty` guards the generated document; `just gen-check`
  guards the committed file.

### 3.4 `<app>-warden` — auth and crypto

- JWT: happy verify, expired, wrong `aud`, wrong `iss`, missing `kid`, `alg:
  none` rejected, tampered signature, TTL above `MAX_ACCESS_TTL_SECS`.
- Password: argon2id parameters, pepper applied before hashing, constant-time
  comparison, malformed hash rejected.
- Opaque refresh: 64-char base64url, single-use CAS, replay rejected.
- Codes: 6 digits, rejection sampling, expiry, single use.
- Cookies: `HttpOnly`, `SameSite=Strict`, `Secure` outside dev, clear recipe.
- OAuth: state round-trip, offline mode refuses before any socket.

### 3.5 `<app>-loom` — concurrency and idempotency

These prove "many workers, one effect":

- two drains claim the same due job → the effect happens once
  (`FOR UPDATE SKIP LOCKED` + lease);
- a worker killed mid-send: the `sending` lease expires and the job is
  recovered;
- retry backoff grows and the attempt cap dead-letters to `failed`;
- the janitor deletes only expired rows and is idempotent when re-run;
- a subscriber whose bounded channel is full is dropped, never blocking the
  publisher;
- WS authentication failure closes with `4001` and never leaks data.

### 3.6 `<app>-mailer`

- Sender selection truth table: offline beats endpoint beats SMTP beats
  console.
- `OFFLINE=true` constructs no `reqwest::Client` and never reads the endpoint.
- Template rendering: both locales, every placeholder, unknown key is an error,
  no secret or token in the rendered body.

### 3.7 Migration tests

The template ships three paired migrations (`0001`–`0003`). CI applies them to
an empty Postgres before running the suite. Required on every schema change:

- apply all migrations from scratch (CI `migrate` step);
- apply the new migration on a database at the previous release;
- prove the `.down.sql` reverses the `.up.sql` (drop/recreate in a scratch
  database);
- expand/contract: old code works after expand, new code works before contract;
- constraints (unique email, unique provider id, FK cascade) actually fire.

An adapter test that runs the real adapter belongs here — it is the only check
the runtime `sqlx` calls ever get (§7.3).

---

## 4. Security tests

Per crate:

- **Role matrix**: for each admin route and each role
  (Viewer/Editor/Admin/Owner), the expected outcome. The `adminusers` tests
  assert every declared action; a new action without a backend check fails
  review.
- **Rank gate**: an Admin cannot modify an Owner (strict `>`); two Admins
  cannot modify each other if the rule is peer-protected.
- **IDOR**: user A requests user B's widget id → `404`. There are no tenants;
  ownership is `user_id`.
- **Prefix is not authorization**: a valid non-admin token on
  `/api/v1/admin/...` is denied by the handler gate. Route-layer middleware
  only proves authentication.
- **Carrier parity**: a route reachable by a browser is tested with the cookie
  carrier, not only `Authorization`.
- **Token**: expired, revoked (`SessionStore`), suspended user, refresh replay,
  logout revoking the session.
- **Input**: oversized body, wrong content type, unknown JSON fields,
  injection payloads in every free-text field, mass assignment on `PUT`.
- **Envelope**: 5xx detail redacted; no `sqlx`/`reqwest`/panic text reaches the
  client; `request_id` present and equal to `x-request-id`.
- **Secrets**: no secret or token in a response, a log line, or an error
  message. `Config`'s redacted `Debug` is tested.

Platform-wide: `clippy -D warnings`, `cargo deny`, `cargo audit`,
`cargo machete`, `scripts/leak-guard.sh`.

---

## 5. Non-functional tests

| Suite | Location | Purpose | Cadence |
|---|---|---|---|
| Performance | product-added `benches/` + load test | absolute budgets on the hot path | pre-release |
| Resilience | deploy-level exercise | DB failover, provider 429/5xx, worker kill; no data loss | pre-release |
| Disaster recovery | deploy-level exercise | restore a backup into a clean environment, replay migrations, measure RPO/RTO | semi-annual |
| Observability | assertions inside handler/loom tests | request id on every span and envelope; no PII in labels or logs | per PR |
| Privacy | `just ci` + review | retention/janitor deletes on schedule; export/erasure flows | per release |
| Audit log | `adminusers` tests | role changes and denials produce an attributable record without secrets | per PR |

---

## 6. Commands

```sh
just ci                              # the gate: fmt-check + clippy + check + nextest + eval contracts
cargo nextest run --workspace        # the whole suite
cargo nextest run -p <app>-auth      # one crate
cargo nextest run -p <app>-auth -E 'test(login)'   # filter by name (nextest filter expression)
cargo test -p <app>-core login       # fallback with libtest
cargo test --manifest-path evals/Cargo.toml        # .claude contract evals
just gen-check                       # committed openapi.json matches the code
just audit                           # cargo-audit + cargo-deny + cargo-machete
bash scripts/lint-agents.sh          # .claude tree vs schemas
bash scripts/lint-skills.sh
bash scripts/lint-commands.sh
bash scripts/leak-guard.sh           # leftover template tokens
```

`cargo nextest` does not run doctests; `just ci` does not run them either. Use
`cargo test --doc` when a crate ships doc examples.

---

## 7. Known gaps in the shipped suite

State these honestly in a readiness review; do not cite them as controls
until they run in CI.

### 7.1 No database-backed adapter tests

Every test uses mocks or a lazy pool; no `Pg*` adapter executes a real query.
Consequences:

- a SQL typo is caught by the database at runtime, not by the gate;
- the CI Postgres service and the `migrate` step are currently unused by the
  suite (they are there for the product's first integration test);
- `docs/09-testing-strategy.md` describes testcontainers and wiremock; neither
  is a dependency of this workspace.

**First product task:** add `sqlx::test`-based adapter tests (or a
testcontainers harness) for the repositories the product actually uses. Until
then, every SQL change requires a manual run against a scratch database.

### 7.2 No end-to-end tier

`evals/run-live.sh` is the live-tier seam (API URL + real provider); it is not
part of `just ci` and does not run by default.

### 7.3 `<app>-sdk` and `<app>-cli` are untested

The typed client and the CLI have no tests. They are thin, but they are the
surface other teams consume; add at least a route-parity test (the SDK's paths
match `openapi.json`) before publishing.

### 7.4 No coverage or mutation gate

`just ci` does not measure coverage. `cargo-llvm-cov` is not installed and no
threshold is enforced. Coverage is visibility, not the goal; wire it when the
product has a stable suite.

### 7.5 No secrets scan in CI

`scripts/leak-guard.sh` catches template tokens, not credentials. Wire a
secrets scanner (gitleaks/trufflehog) before release.

### 7.6 No container or dependency-provenance scan

CI runs `cargo-deny` + `cargo-audit`; images are built but not scanned, and no
SBOM is produced. Known gap for supply-chain evidence.

---

## 8. Environments, data and pipeline

### 8.1 Environments

| Environment | Used for | Data |
|---|---|---|
| Local / CI ephemeral | unit, handler, contract | synthetic, containers |
| Ephemeral stack per PR | E2E, DAST (product-added) | synthetic, destroyed |
| Staging | pre-release E2E, load, DR rehearsal | synthetic, prod-like volume |
| Production | synthetic monitoring and smoke only | dedicated flagged account |

Environments are separated: distinct accounts, distinct credentials, no
network path from test to production data.

### 8.2 Test data

- Never production data, never copied down; builders and synthetic fixtures.
- Test secrets are generated per environment, short-lived, in the secret
  manager — not in the repo.
- Fixtures contain no real personal data.

### 8.3 Pipeline stages (actual)

| Stage | Trigger | Runs |
|---|---|---|
| `just ci` | developer | fmt-check, clippy, check, nextest, eval contracts |
| PR `ci.yml` | every PR/push | fmt, clippy+check, test (Postgres service + migrate + nextest), audit, eval — four parallel jobs |
| Push | `deploy.yml`, `docker-publish.yml` | build and publish |
| Manual | `rollback.yml` | rollback |
| Live tier | `evals-live.yml` | `run-live.sh` against a deployed API |

### 8.4 Thresholds (starting points)

| Area | Threshold |
|---|---|
| `core` + slice services | ≥ 90% line / ≥ 85% branch once coverage is wired |
| Adapters | covered by integration tests; coverage informational |
| New code | coverage must not decrease; changed lines covered |
| Flaky rate | < 1%; above that the suite is paused for repair |
| PR pipeline | target ≤ 15 min; slow suites go nightly |

The authorization, isolation and idempotency suites are gated on **presence
and pass**, not on a percentage.

---

## 9. Compliance evidence (SOC 2 / ISO/IEC 27001)

### 9.1 Control-to-test mapping (starting point)

SOC 2 references use the 2017 Trust Services Criteria; ISO references use
ISO/IEC 27001:2022 Annex A. Confirm with your auditor.

| Control objective | Tests / evidence | SOC 2 | ISO 27001:2022 |
|---|---|---|---|
| Logical access and least privilege | role matrix, rank gate, IDOR, prefix-not-authz, session revocation | CC6.1–CC6.3 | A.5.15, A.5.18, A.8.2, A.8.3 |
| Authentication | JWT expiry/audience/alg, refresh replay, suspended user | CC6.1 | A.8.5 |
| Ownership segregation | two-user isolation sweep over detail, list, mutation, stream | CC6.1, C1.1 | A.8.3, A.8.22 |
| Cryptography and secrets | pepper, argon2 params, redacted config, no secrets in logs | CC6.1, CC6.7, C1.1 | A.8.24, A.8.12 |
| Secure development and testing | this playbook, clippy/deny gates, review | CC8.1 | A.8.25, A.8.26, A.8.28, A.8.29 |
| Change management | required checks, PR→ticket traceability, release bundle | CC8.1 | A.8.32 |
| Separation of environments | environment isolation, no prod data in tests | CC6.1, CC8.1 | A.8.31 |
| Protection of test information | synthetic data, per-env secrets | C1.1 | A.8.33 |
| Vulnerability management | `cargo audit`, `cargo deny`, image/DAST (gaps in §7) | CC7.1 | A.8.8, A.8.9 |
| Logging and monitoring | request id tests, audit-log tests, no PII in labels | CC7.2, CC7.3 | A.8.15, A.8.16 |
| Incident readiness | alert tests, runbook game days | CC7.3–CC7.5 | A.5.24–A.5.28 |
| Availability and recovery | DR restore, resilience, load vs SLO | A1.1–A1.3, CC9.1 | A.5.30, A.8.13, A.8.14 |
| Processing integrity | CAS rotation, outbox idempotency, migration and contract tests | PI1.1–PI1.5 | A.8.26, A.8.27 |
| Privacy and data lifecycle | PII redaction, janitor retention, export/erasure | P-series, C1.2 | A.5.34, A.8.10, A.8.11 |
| Supply chain | `Cargo.lock`, `cargo deny`, SBOM/provenance (gap) | CC9.2 | A.5.19–A.5.22, A.8.30 |

### 9.2 Labeling tests to controls

Rust has no test tags. Use a comment on the test, greppable by the evidence
collector:

```rust
/// control: SOC2:CC6.1, ISO27001:A.8.3
/// risk: authz
/// slice: adminusers
#[tokio::test]
async fn rejects_update_when_caller_is_not_admin() { … }
```

The collector reads `cargo nextest` JUnit output plus these labels and emits
the traceability matrix: control → test → last result → artifact link. A
control with no passing, recent test shows up as a gap.

### 9.3 Evidence requirements

- Each pipeline run stores JUnit XML (`cargo nextest run --message-format
  junit`), clippy/SARIF output, `cargo-deny` report, SBOM,
  commit SHA, approver, and the workflow version.
- Append-only, write-once storage with hashes, access restricted and logged,
  retained for the audit window (SOC 2 Type II observations are typically
  3–12 months).
- Manual activities (pen test, DR restore, access review, game day) are dated,
  signed reports in the same store with a ticket link.
- Timestamps from CI, not from test code.
- No secrets or personal data in evidence.

### 9.4 Release evidence bundle

```text
evidence/{release}/
  manifest.json                 # commit, version, workflow run id, approver(s), hashes
  test-results/                 # JUnit per crate, eval contract results
  coverage/                     # when wired
  security/                     # cargo-deny, cargo-audit, SBOM, provenance
  traceability-matrix.html|csv
  change-record.link
  exceptions.md
```

### 9.5 Change-management gates

- Protected `main`; no direct pushes; one reviewer who is not the author;
  CODEOWNERS covers shared crates (`core`, `ports`, workspace `Cargo.toml`,
  `openapi.json`, `.github/`).
- Required checks cannot be bypassed by administrators except through the
  emergency path.
- Emergency change: recorded justification and approver, full suite afterwards,
  logged in `exceptions.md`.
- Every PR links to a work item; the release links to its merged PRs.
- The release approver is not the sole author.

### 9.6 Exceptions and waivers

Owner, justification, compensating control, expiry (default ≤ 30 days) and an
approver outside the owning team. Expired waivers fail the build.

### 9.7 Periodic control testing

| Activity | Frequency | Evidence |
|---|---|---|
| Backup restore / DR rehearsal | semi-annual | report with RPO/RTO |
| Independent penetration test | annual + on major change | report, tickets |
| Access-review verification | quarterly | permission export vs. role matrix |
| Chaos / game day | quarterly | report and actions |
| Full-history secrets scan | monthly | report |
| Dependency/base-image refresh | continuous, reviewed monthly | update PRs, SLA dashboard |
| Playbook and control-map review | annual | versioned change log |

---

## 10. Concrete example: admin updates a user

Feature: `PUT /api/v1/admin/users/{id}`.

```text
crates/<app>-adminusers/src/lib.rs     # rank gate, patch rules, audit write
crates/<app>-api/src/routes/admin_users.rs   # 401/403/404/400/200 + envelope
crates/<app>-api/src/router.rs         # route-table + spec registration
crates/<app>-ports/src/mock/           # user + audit doubles
evals/                                 # unaffected unless the kit contract changes
```

Representative tests:

```rust
/// control: SOC2:CC6.1, ISO27001:A.5.15
/// risk: authz
#[tokio::test]
async fn rejects_update_when_caller_is_below_admin() { … }   // service, 403

#[tokio::test]
async fn admin_cannot_modify_peer_or_higher_rank() { … }     // strict rank gate

#[tokio::test]
async fn foreign_user_patch_answers_404() { … }              // IDOR, handler

#[tokio::test]
async fn patch_writes_an_attributable_audit_row() { … }      // audit evidence
```

---

## 11. Checklist

- [ ] Tests colocate with their crate as `#[cfg(test)] mod tests`.
- [ ] Every assertion sits at the lowest level that can prove it.
- [ ] Slice services tested against `ports` mocks; no live DB in unit tests.
- [ ] Handler tests cover 401/403/404/400 and the envelope.
- [ ] Role matrix covers every declared admin action, including equal-rank
      denial.
- [ ] IDOR sweep covers detail, list, mutation and stream paths (two users).
- [ ] Token lifecycle tested: expiry, revocation, refresh replay, suspension.
- [ ] Outbox and janitor concurrency tested: one effect under duplicate claim,
      lease recovery, dead-letter.
- [ ] Offline mode tested: no socket, no provider call.
- [ ] SQL-touching change ran against a real database (until §7.1 is fixed).
- [ ] `just ci` is green; no `#[ignore]` in the tree.
- [ ] `just gen-check`, `cargo deny`, `cargo audit`, `cargo machete` green.
- [ ] `.claude` linters and `leak-guard.sh` green.
- [ ] Control-relevant tests labeled; traceability matrix generated.
- [ ] Evidence retained append-only, with no secrets or personal data.
- [ ] Protected branch, required checks, independent review and linked work
      items enforced; waivers carry an expiry.
- [ ] Flaky tests quarantined with an owner and expiry, never auto-retried.
- [ ] Control map reviewed with the auditor annually.
