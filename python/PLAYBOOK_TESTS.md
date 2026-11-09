# PLAYBOOK_TESTS.md — testing and compliance evidence (Python backend)

How `<app>` proves behavior, what the shipped suite does not prove, and how
the gates map to audit evidence. The rule above all: **a green gate means the
things it runs passed, not that the system works.**

Premise for actors, roles and invariants: `MEMORY.md`.

---

## 1. What ships

```text
tests/unit/test_services.py             unit — use cases against in-memory fakes (14 tests)
tests/fakes.py                          in-memory adapters, deterministic ids, test publisher
tests/conftest.py                       the real build_app(deps), uvicorn thread, fixtures
tests/acceptance/steps_*.py            pytest-bdd step definitions (7 modules)
tests/acceptance/support.py             per-scenario state and request helpers
tests/acceptance/test_bdd.py            binds the 7 feature files (59 executed cases)
tests/acceptance/test_http.py           14 HTTP contract checks that need no scenario
specs/features/*.feature                59 executed cases (7 features; 56 definitions)
evals/contracts_test.py                 14 kit-contract tests (pytest)
evals/run-live.sh                       operator smoke against the Claude CLI
```

### 1.1 Naming

- Unit tests: `tests/unit/test_*.py`; one file per concern, no package
  directories beyond `tests/unit` and `tests/acceptance`.
- Acceptance: `specs/features/<slice>.feature` + a `steps_<slice>.py` whose
  vocabulary matches only its own slice.
- Kit contracts: `evals/contracts_test.py`, fixtures under `evals/fixtures/`.
- A test named for the rule it protects (`test_admin_cannot_edit_a_peer`),
  never `test_1`.

---

## 2. Test types and the pyramid

| Level            | Runner                              | I/O                                            | Speed  | Catches                                                                            |
| ---------------- | ----------------------------------- | ---------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| **Unit**         | pytest + pytest-asyncio (`make test`) | none — fakes                                  | ms     | domain rules, use-case ordering, expiry windows, ownership scoping, audit writes  |
| **Acceptance**   | pytest-bdd (`make accept`)          | real FastAPI app on an ephemeral port; fake ports | fast   | route wiring, auth posture, status + envelope, cross-step flows                    |
| **HTTP contract** | pytest (`make accept`)             | real app, no feature file                      | fast   | error envelope uniformity, security headers, WebSocket handshake                   |
| **Kit contract** | pytest (`make evals`)               | filesystem only                                | ms     | `.claude` schema compliance, placeholder leaks, domain-import guards              |
| **Live smoke**   | `evals/run-live.sh`                 | the Claude CLI and an API key                   | manual | skill invocation quality, LLM-judge verdict                                       |

There is **no integration tier**. Nothing in CI executes real SQL against
Postgres. That is the single biggest evidence gap (§7.1), and it is honest to
say so in an audit rather than to call the fake tier "integration".

---

## 3. Unit tier

### 3.1 Rules and use cases

- Fakes implement the consumer's `ports.py` `Protocol` as plain classes in
  `tests/fakes.py`; no mocking library, no module monkeypatching, no
  "was it called" choreography.
- Inject the clock (`FixedClock` from `app/shared/clockx.py`) so expiry,
  attempt windows and rate limits are deterministic. Never sleep in a unit
  test.
- `asyncio_mode = "auto"` means an async test function needs no decorator;
  the event loop scope is per function.
- Parametrize when the behavior repeats (`pytest.mark.parametrize`); one case
  per boundary (empty, max, one-past-max, wrong type, wrong owner).
- Test the failure as carefully as the success: which `Code`, which status,
  whether anything was written.

Shipped coverage: `authsignup` (3), `authsession` (2), `authpassword` (4),
`adminusers` (3), `widgets` (2) — 14 tests total, all in
`tests/unit/test_services.py`.

### 3.2 What belongs at this level

- account-state gate order (unverified, inactive, blocked);
- refresh rotation and replay rejection;
- code expiry, attempt counting, and the "validate does not consume" rule;
- admin rank decisions (`can_manage_subordinate`) and the whitelist bootstrap;
- widget owner scoping and kind validation;
- the audit write that must land with the mutation.

### 3.3 What does not

- SQL correctness. A fake cannot prove a `WHERE` clause.
- Route wiring, status codes, and envelope shape. Those are acceptance's job.

---

## 4. Acceptance tier

`tests/conftest.py` builds the **real** app — `build_app(deps)` with the same
wiring the composition root uses — but with in-memory stores, a recorded mail
queue, a `FixedClock`, fake Google/GitHub clients, and the real `Hub`. The app
runs under uvicorn in a background thread on an ephemeral port, and the steps
talk to it with a synchronous `httpx.Client`.

Why this is stronger than it looks: if a slice's dependencies grew something a
fake cannot supply, acceptance fails, and that is an architecture signal (the
boundary leaked infrastructure into the use case).

Two mechanics worth knowing before you write a step:

- **Steps are plain `def`.** pytest-bdd 9 does not await async step
  functions, so a step that needs the server must go through the HTTP client.
- **Hashing is cheap in tests.** An autouse fixture replaces
  `app.platform.authx._hasher` with low-cost argon2 parameters; the shipped
  cost is exercised by the behavior around the KDF, not by the KDF's cost.

Rules:

- One feature file per slice. Steps are HTTP-level, not implementation-level:
  `the response code is "AUTH_EMAIL_UNVERIFIED"`, never "the user row's
  verified_at is null".
- Every scenario asserts status **and** `code` on failures.
- A scenario may not depend on another scenario's state; each test gets a
  fresh `World` from the fixture.
- Social flows run offline through `FakeGoogle` / `FakeGithub`; the suite
  never dials the network.
- A new route or a changed error code needs a scenario in the same PR.
- Acceptance is not E2E: no browser, no real Postgres, no SMTP, no
  multi-process. Do not describe it as E2E.

Shipped features (executed cases): `authsignup` (7), `authsession` (9),
`session-mgmt` (11), `authpassword` (9), `authoauth` (11), `adminusers` (8),
`widgets` (4) — 59 cases from 56 scenario definitions. The `authsession` and
`adminusers` outlines expand to 3 and 2 rows respectively.

`tests/acceptance/test_http.py` adds 14 checks that need no feature file:
`/healthz` and `/readyz` shape, the generic 500 envelope, a parametrized
failure table asserting every error body is exactly `{detail, code}` with a
known `Code`, the security headers and request-id echo, the WebSocket close
with code 4001 on a bad token, and the WebSocket `hello` frame.

---

## 5. Kit contract tier

`evals/contracts_test.py` is offline and dependency-free. It validates:

- agent/skill/command frontmatter keys against `.claude/schemas/*.md`;
- required body sections per document kind;
- the allowed division enum;
- CRLF rejection and literal-placeholder leaks;
- domain-import violations (a pure-domain import passes; a cross-slice import
  is flagged);
- a post-rename sweep for placeholder leaks across the tree (skipped while
  the tree is still the pre-rename template).

Run it with `make evals`. It **is** part of `make ci` and of the CI workflow.

The bash linters are the other half of the kit contract:
`scripts/lint-agents.sh`, `scripts/lint-skills.sh`,
`scripts/lint-commands.sh`. They parse the required fields and sections out of
the schema docs, so the linter and the schema cannot drift; `lint-agents.sh`
also checks that an agent's `division:` equals the directory it lives in.
While the tree is still the pre-rename template, `lint-skills.sh` and
`lint-commands.sh` need `ALLOW_PLACEHOLDERS=1` (their placeholder check is
fatal after the rename); `lint-agents.sh` has no placeholder check. None of
the three is wired into `make ci` or CI — `scripts/verify-template.sh`
invokes them on the renamed copy, and there their exit code is not fatal.

`scripts/leak-guard.sh` scans for placeholder residuals and high-signal
secret patterns. It is also run by `verify-template.sh` on the renamed copy,
where its exit code is likewise not fatal.

`evals/run-live.sh` is an operator tool, not a gate: it needs `EVALS=1`, the
`claude` CLI and `ANTHROPIC_API_KEY`; without them it exits 0 as a no-op
(INV-T16, never in CI). `TIER=3` adds an LLM-judge pass over a canned skill
output.

---

## 6. Commands and CI

```sh
make ci                                   # fmt-check + vet + lint + gen-check + unit + accept + evals
make test                                 # unit only (pytest tests/unit)
make accept                               # acceptance + HTTP contract checks
make evals                                # kit contract suite (pytest evals)
make generate / make gen-check            # the committed specs/openapi.yaml is not stale
make vuln                                 # pip-audit
make verify                               # staged renamed copy + factory smoke
```

`make ci` is, in order: `ruff format --check .` (including the Python fences
inside these Markdown docs), `mypy src` in strict mode, `ruff check .` plus
`lint-imports` (the three architecture contracts declared in
`pyproject.toml`), the OpenAPI freshness check, the unit tier, the acceptance
tier, and the kit contract tier.

`.github/workflows/ci.yml` runs `make ci`, then `make vuln` (`pip-audit`),
then actionlint. It does not run the kit linters, `verify-template.sh`, or any
database-backed test.

One reproducibility caveat: the template ships without `uv.lock` (INV-T3), so
CI resolves dependency ranges fresh on every run. Pin the lockfile after
instantiation if your evidence needs byte-reproducible installs.

### 6.1 Thresholds

No coverage threshold ships. A useful starting policy:

- every use case with a branch has a unit test;
- every route has at least one acceptance scenario including a failure path;
- a bug fix lands with the test that fails without the fix;
- coverage numbers are a smell detector, not the target.

Do not add a coverage gate to make a dashboard green; add it when a specific
module's regressions are the observed problem.

---

## 7. Known gaps in the shipped suite

### 7.1 No database-backed adapter tests

The SQLAlchemy Core `text()` queries in each slice's `queries.py`, the Alembic
`upgrade`/`downgrade` pairs, the `FOR UPDATE SKIP LOCKED` claim semantics in
`app/platform/email/outbox.py`, and the JSON/UUID column mapping are
unproven. The only real SQL that runs is the `migrate-up` / `migrate-down` /
`migrate-up` smoke in `scripts/verify-template.sh`, against a throwaway
Postgres 17 — and that script is not part of `make ci` or CI. Add a
real-Postgres lane deliberately (see `docs/11-testing-and-ci.md` §3); do not
call the fake tier integration.

### 7.2 Slices and platform modules lack unit tests

`authoauth` and `realtime` have no unit test at all; their behavior is
covered only by acceptance. The same is true of the platform layer:
`app/platform/authx.py` (HS256 algorithm/issuer pinning, argon2 verification),
`app/platform/config.py` (weak-secret rejection, offline-vs-SMTP conflict),
and `app/platform/email/outbox.py` (claim/settle/purge). The use cases are
where the ordering rules live; add unit tests before changing those modules.

### 7.3 No coverage or mutation gate

There is no coverage threshold and no mutation run. Both are worth adding once
the suite is stable; neither substitutes for the missing integration tier.

### 7.4 No secrets or dependency-provenance scan

CI runs `pip-audit` only. No secret scanner, SBOM, license scan, or container
image scan. `scripts/leak-guard.sh` is a template-local guard over the tree,
not a scanner, and it does not run in CI. Add gitleaks/trivy/Syft as separate
jobs; keep them advisory until the baseline is clean.

### 7.5 No end-to-end tier

No browser, no compose-driven test, no SMTP sink. `evals/run-live.sh` is a
manual smoke of the skill layer, not a repeatable gate over the API.

### 7.6 Realtime coverage is thin

The hub itself (drop-oldest on a full queue, unsubscribe idempotency, room
isolation) has no unit test. The WebSocket close with code 4001 and the
`hello` frame are covered by `tests/acceptance/test_http.py`, and SSE framing
is covered indirectly by the widgets event-stream scenario. Add a hub unit
test and a reconnect scenario before relying on realtime in production.

---

## 8. Security tests (per boundary)

These belong at the tier named; the shipped suite has partial coverage.

| Property                                     | Tier            | Status                     |
| -------------------------------------------- | --------------- | -------------------------- |
| Unverified/inactive/blocked login gates      | acceptance/unit | shipped                    |
| Refresh rotation rejects the old token       | acceptance/unit | shipped                    |
| Code attempt cap and expiry                  | acceptance/unit | partial (cap untested)     |
| Reset revokes every session                  | acceptance      | shipped                    |
| Change-password revokes every session        | acceptance      | shipped                    |
| Admin cannot manage a peer admin             | unit/acceptance | shipped (unit)             |
| Foreign row reads as 404, not 403            | acceptance      | shipped                    |
| Upgrade-to-admin follows the whitelist       | unit/acceptance | shipped                    |
| WS token absent/invalid → 4001               | acceptance      | shipped (`test_http.py`)   |
| Security headers and request-id echo         | acceptance      | shipped (`test_http.py`)   |
| Every error body is `{detail, code}`         | acceptance      | shipped (`test_http.py`)   |
| JWT algorithm/issuer/expiry pinning          | unit            | add                        |
| Config rejects weak secret and offline+SMTP  | unit            | add                        |
| Body size limit                              | acceptance      | add (not implemented)      |
| CORS rejects an unlisted origin              | acceptance      | add                        |
| Rate limit on reset codes                    | acceptance      | shipped                    |

Rule: a security fix without a regression test is not fixed; it is deferred.

---

## 9. Non-functional tests

Not shipped. When they are added, keep them in a separate lane:

- **Load**: Locust or k6 against a compose stack; report p95 and error rate.
- **Soak**: 30+ minutes to catch task, queue or subscription leaks.
- **Migration timing**: `migrate-up` and `migrate-down` on a populated copy.
- **Outbox drain**: enqueue N, kill the worker mid-batch, assert at-least-once
  and no lost row.
- **Shutdown**: SIGTERM under load; assert in-flight requests finish or fail
  cleanly and the worker loops stop.

---

## 10. Environments, data and pipeline

### 10.1 Environments

| Environment | Data                                             | Who can reach it               |
| ----------- | ------------------------------------------------ | ------------------------------ |
| Local       | disposable Postgres 17 (compose), `<APP>_OFFLINE=true` | developer                 |
| CI          | no database                                      | automation                     |
| Staging     | anonymized or synthetic; never a production dump | team                           |
| Production  | real                                             | operators, break-glass audited |

### 10.2 Test data

- Never copy production PII into a lower environment. Synthesize users,
  emails and widgets.
- Fixtures are deterministic: `SequenceIds` hands out the ids and codes, the
  clock is `FixedClock`.
- Every test gets a fresh `World`; a shared mutable fake across scenarios is
  a flakiness source.

### 10.3 Pipeline stages

1. Format + typecheck + lint + gen-check + unit + acceptance + evals
   (`make ci`).
2. Dependency audit (`make vuln`).
3. Workflow lint (actionlint).
4. (Add) kit linters as a CI job, secret scan, SBOM, image scan.
5. (Add) integration against a disposable Postgres.
6. Deploy with a rollback path (`docker-publish` → `deploy` → `rollback`).

A stage that can be skipped silently is not a gate.

---

## 11. Compliance evidence (SOC 2 and ISO/IEC 27001)

Map a control to a command and an artifact; an auditor accepts a repeatable
command, not a claim.

### 11.1 Control-to-test mapping (starting point)

| Control area             | Evidence in this repo                                                        |
| ------------------------ | ---------------------------------------------------------------------------- |
| Change management        | PR diff + `make ci` output + review approval                                 |
| Segregation of duties    | "Who edits what" ownership table in `AGENTS.md`; human-only control plane      |
| Access control           | `adminusers` unit tests + admin acceptance scenarios + `PLAYBOOK_SECURITY.md` §5 |
| Cryptographic controls   | argon2id + HS256 in `app/platform/authx.py`; `Settings` secret rules (unit tests to add) |
| Boundary validation      | `app/platform/envelope.py` + the failure table in `tests/acceptance/test_http.py` |
| Logging and monitoring   | request-id + security headers in `app/bin/middleware.py`; `/healthz`, `/readyz` |
| Vulnerability management | `pip-audit` job in `.github/workflows/ci.yml` + dependency review              |
| Backup and recovery      | Postgres backup config (outside this repo) + restore drill                    |
| Incident response        | `PLAYBOOK_SECURITY.md` §13 + runbook                                         |
| Supplier management      | dependency ranges in `pyproject.toml` + `pip-audit`                           |

### 11.2 Tagging tests to controls

When the suite is wired into an evidence system, tag the feature or test name
with the control id (`@SOC2-CC6.1`, `@ISO27001-A.9.2.3`). Keep the tag in the
spec, not in prose, so the runner can enumerate coverage.

### 11.3 Evidence requirements

For each control period, retain:

- the gate command and its output (CI log or signed artifact);
- the commit range it covered;
- reviewer identity and timestamp;
- any exception with owner, reason and expiry;
- for manual controls, the operator, the date and the artifact produced.

Record the resolved dependency versions alongside the log: without a committed
`uv.lock` the CI log is the only record of what actually ran.

### 11.4 Release evidence bundle

```text
release-<version>/
├── ci.log                 # make ci output
├── unit.log               # make test summary
├── accept.log             # make accept summary
├── evals.log              # make evals summary
├── audit.log              # pip-audit
├── sbom.json              # when added
├── image-digest.txt       # when built
└── approvals.md           # reviewers + timestamp
```

### 11.5 Change-management gates

- No direct push to `main`; a PR with a green gate.
- Schema changes carry an Alembic migration with both `upgrade` and
  `downgrade`, plus a rollback note.
- Security-sensitive changes (auth, crypto, config, CI) need a second
  reviewer.
- Emergency changes get a retrospective ticket within the next business day.

### 11.6 Exceptions and waivers

An exception is a record, not a comment: what control, why, compensating
measure, owner, expiry. Expired exceptions fail the next review by default.

---

## 12. Concrete example: updating a user as an admin

What a complete change looks like:

1. **Unit** (`tests/unit/test_services.py`): an ADMIN cannot edit another
   ADMIN (`can_manage_subordinate` is a strict rank comparison); the mutation
   writes an `admin_user.update` audit row; an email outside the whitelist
   cannot self-promote.
2. **Acceptance** (`specs/features/adminusers.feature`): `PATCH
   /api/v1/admin/users/{user_id}` by a USER → 403 `FORBIDDEN`; by an ADMIN on a
   USER → 200 and the row changed with one audit entry; a bad role → 422
   `VALIDATION_FAILED` with no audit entry; an unknown id → 404 `NOT_FOUND`
   with no audit entry.
3. **Contract**: the request/response shape stays in `specs/openapi.yaml`;
   `make generate` leaves the committed file consistent and `make gen-check`
   fails when it drifts.
4. **Audit**: the handler writes `audit_logs` (actor, action, entity) in the
   same code path as the change; an acceptance step asserts the row exists.
5. **Gate**: `make ci` green; the PR includes the logs.

If any of those is missing, the change is not done — it is merely compiling.

---

## 13. Checklist

- [ ] New behavior has a unit test at the layer that owns the rule.
- [ ] New or changed route has an acceptance scenario, including a failure.
- [ ] Failure assertions check `code`, not only status.
- [ ] Time is injected (`FixedClock`); no sleeps, no wall-clock dependence.
- [ ] No real network, Postgres, SMTP or Redis in unit or acceptance.
- [ ] A bug fix ships with the test that fails without it.
- [ ] `make ci` green; output read, not assumed.
- [ ] Kit-contract evals green after `.claude/**` changes.
- [ ] New tests are deterministic under reordering: no shared mutable fake
      between tests, no dependence on wall-clock time.
- [ ] Evidence artifacts retained for release.
