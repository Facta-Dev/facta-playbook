# PLAYBOOK_TESTS.md — testing and compliance evidence (Node backend)

How `<app>` proves behavior, what the shipped suite does not prove, and how
the gates map to audit evidence. The rule above all: **a green gate means the
things it runs passed, not that the system works.**

Premise for actors, roles and invariants: `MEMORY.md`.

---

## 1. What ships

```text
src/features/*/usecase.test.ts        unit — rules against fakes
src/platform/*/*.test.ts              unit — kernel/platform contracts
test/acceptance/app.ts                the real Fastify app, fake-wired
test/acceptance/fakes.ts              in-memory ports, test clock, real JWT pair
test/acceptance/steps-*.ts            cucumber step definitions (7 files)
specs/features/*.feature              60 executed scenarios (7 features; 57 declarations)
evals/contracts.test.mjs              14 kit-contract tests (node --test)
evals/run-live.sh                     operator smoke against a live stack
```

### 1.1 Naming

- Unit tests: `*.test.ts` beside the code; no `__tests__` directories.
- Acceptance: `specs/features/<slice>.feature` + a `steps-<slice>.ts` whose
  regexes match only its own vocabulary.
- Kit contracts: `evals/contracts.test.mjs`, fixtures under `evals/fixtures/`.
- A test named for the rule it protects (`refuses a wrong code after the
attempt cap`), never `test1`.

---

## 2. Test types and the pyramid

| Level            | Runner                                 | I/O                                           | Speed  | Catches                                                                            |
| ---------------- | -------------------------------------- | --------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| **Unit**         | Vitest (`make test`)                   | none — fakes                                  | ms     | domain rules, use-case ordering, envelope mapping, config validation, hub contract |
| **Acceptance**   | Cucumber (`make accept`)               | real Fastify on an ephemeral port; fake ports | fast   | route wiring, auth posture, status + envelope, cross-step flows                    |
| **Kit contract** | `node --test evals/contracts.test.mjs` | filesystem only                               | ms     | `.claude` schema compliance, placeholder leaks, slice-import guards                |
| **Live smoke**   | `evals/run-live.sh`                    | a running stack                               | manual | boot, migrations, real HTTP/WS                                                     |

There is **no integration tier**. Nothing in CI executes real SQL against
Postgres. That is the single biggest evidence gap (§7.1), and it is honest to
say so in an audit rather than to call the fake tier "integration".

---

## 3. Unit tier

### 3.1 Rules and use cases

- Fakes implement the slice's `ports.ts` as plain classes; no mocking library,
  no `vi.mock` of modules, no `toHaveBeenCalled` choreography.
- Inject the clock (`#shared/clockx`) so timeouts, expiry and rate windows
  are deterministic. Never `await sleep(…)` in a unit test.
- Table-driven when the behavior repeats (`it.each`); one case per boundary
  (empty, max, one-past-max, wrong type, wrong owner).
- Test the failure as carefully as the success: which `code`, which status,
  whether anything was written.

Shipped coverage: `adminusers` (4), `widgets` (4), `authx` (3),
`config` (6), `realtime/hub` (5) — 22 tests total.

### 3.2 What belongs at this level

- ownership/rank decisions (`canManageSelf`, `canManageSubordinate`);
- password hashing/verification with and without a pepper;
- JWT verify: algorithm pinning, issuer, expiry, malformed;
- config: placeholder rejection, offline-vs-SMTP conflict, duration parsing;
- hub: drop-on-full, cancel idempotency, room isolation;
- use-case ordering: enqueue only after the write, publish only on success.

### 3.3 What does not

- SQL correctness. A fake cannot prove a `WHERE` clause.
- Fastify route syntax, `preHandler` order, status codes. Those are
  acceptance's job.

---

## 4. Acceptance tier

`test/acceptance/app.ts` builds the **real** app — every slice's `register`
is called with the same shape the root uses — but with in-memory `Store`
implementations, a recorded mail queue, a test clock, and the real HS256
minter/verifier pair. It listens on an ephemeral port and cucumber drives it
over HTTP.

Why this is stronger than it looks: if a slice's `Deps` grew a dependency a
fake cannot supply, acceptance fails, and that is an architecture signal
(the boundary leaked infrastructure into the use case).

Rules:

- One feature file per slice or capability. Steps are HTTP-level, not
  implementation-level: "the response code is 403 AUTH_EMAIL_NOT_VERIFIED",
  never "the user row's verified_at is null".
- Every scenario asserts status **and** `code` on failures.
- A scenario may not depend on another scenario's state; each starts from a
  fresh app or explicitly reset fakes.
- Social flows run offline through fake Google/GitHub clients wired in
  `fakes.ts`; the suite never dials the network.
- A new route or a changed error code needs a scenario in the same PR.
- Acceptance is not E2E: no browser, no real Postgres, no SMTP, no multi-
  process. Do not describe it as E2E.

Shipped features (executed scenarios): `authsignup` (7), `authsession` (10),
`session-mgmt` (11), `authpassword` (9), `authoauth` (11), `adminusers` (8),
`widgets` (4) — 60 scenarios. `authsession` and `adminusers` each contain an
outline, so their executed counts exceed their declaration counts.

---

## 5. Kit contract tier

`node --test evals/contracts.test.mjs` is zero-dependency and offline. It
validates:

- agent/skill/command frontmatter against `.claude/schemas/*.md`;
- required body sections per document kind;
- CRLF rejection and literal-placeholder leaks;
- the slice-import lint fixtures (a pure-domain import passes; a cross-slice
  import is flagged).

Run it after any `.claude/**` or `scripts/lint-*.sh` change. It is not part of
`make ci`; add it to CI if the kit is load-bearing for your team.

`evals/run-live.sh` is an operator tool, not a gate: it assumes a reachable
stack and a valid `.env`.

---

## 6. Commands and CI

```sh
make ci                                   # fmt-check + tsc (both tsconfigs) + eslint + gen-check + vitest
make test                                 # unit only
make accept                               # cucumber over the fake-wired app
node --test evals/contracts.test.mjs      # kit contracts
make generate / make gen-check            # committed models are not stale
make vuln                                 # pnpm audit --audit-level=high
bash scripts/verify-template.sh           # staged renamed copy + factory smoke
```

`.github/workflows/ci.yml` runs `make ci`, then unit + acceptance,
`pnpm audit --audit-level=high`, and actionlint. It does not run the kit
contracts or any database-backed test.

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

`postgres.ts` adapters, the migration up/down pairs, `FOR UPDATE SKIP LOCKED`
claim semantics, and JSONB mapping are unproven. Add a real-Postgres lane
deliberately (see `docs/11-testing-and-ci.md` §3); do not call the fake tier
integration.

### 7.2 Auth slices lack unit tests

`authsignup`, `authsession`, `authpassword`, `authoauth` and `realtime` have
no `*.test.ts`. Their behavior is covered only by cucumber. The use cases are
where the ordering rules live; add unit tests before changing those slices.

### 7.3 No coverage or mutation gate

There is no `c8`/`vitest --coverage` threshold and no mutation run. Both are
worth adding once the suite is stable; neither substitutes for the missing
integration tier.

### 7.4 No secrets or dependency-provenance scan

CI runs `pnpm audit` only. No secret scanner, SBOM, license scan, or container
image scan. Add gitleaks/trivy/Syft as separate jobs; keep them advisory until
the baseline is clean.

### 7.5 No end-to-end tier

No browser, no compose-driven test, no SMTP sink. `evals/run-live.sh` is a
manual smoke, not a repeatable gate.

### 7.6 Realtime coverage is hub-only

`hub.test.ts` pins the drop/cancel contract. WS handshake, close `4001`, SSE
framing and reconnect behavior are covered only indirectly (if at all) by
acceptance. Add a WS scenario before relying on realtime in production.

---

## 8. Security tests (per boundary)

These belong at the tier named; the shipped suite has partial coverage.

| Property                                     | Tier            | Status                     |
| -------------------------------------------- | --------------- | -------------------------- |
| Unverified/inactive/blocked login gates      | acceptance      | shipped                    |
| Refresh rotation rejects the old token       | acceptance      | shipped                    |
| Code attempt cap and expiry                  | acceptance/unit | partial                    |
| Reset revokes sessions                       | acceptance      | shipped                    |
| Admin cannot manage a peer admin             | unit/acceptance | shipped (unit)             |
| Foreign row reads as 404, not 403            | acceptance      | shipped                    |
| JWT algorithm/issuer/expiry pinning          | unit            | shipped (`authx.test.ts`)  |
| Config rejects placeholders and offline+SMTP | unit            | shipped (`config.test.ts`) |
| Body size limit and malformed JSON           | acceptance      | add                        |
| CORS rejects an unlisted origin              | acceptance      | add                        |
| WS token absent/invalid → 4001               | acceptance      | add                        |
| Rate limit on reset codes                    | acceptance      | partial                    |

Rule: a security fix without a regression test is not fixed; it is deferred.

---

## 9. Non-functional tests

Not shipped. When they are added, keep them in a separate lane:

- **Load**: k6/autocannon against a compose stack; report p95 and error rate.
- **Soak**: 30+ minutes to catch socket/subscription leaks.
- **Migration timing**: `migrate-up` and `migrate-down` on a populated copy.
- **Outbox drain**: enqueue N, kill a worker mid-batch, assert at-least-once
  and no lost row.
- **Shutdown**: SIGTERM under load; assert in-flight requests finish or fail
  cleanly, and the pool ends.

---

## 10. Environments, data and pipeline

### 10.1 Environments

| Environment | Data                                             | Who can reach it               |
| ----------- | ------------------------------------------------ | ------------------------------ |
| Local       | disposable Postgres (compose), `OFFLINE=true`    | developer                      |
| CI          | no database                                      | automation                     |
| Staging     | anonymized or synthetic; never a production dump | team                           |
| Production  | real                                             | operators, break-glass audited |

### 10.2 Test data

- Never copy production PII into a lower environment. Synthesize users,
  emails and widgets.
- Fixtures are deterministic: fixed ids, fixed clock, fixed codes.
- Acceptance fakes are reset per scenario; a shared mutable fake is a
  flakiness source.

### 10.3 Pipeline stages

1. Format + typecheck + lint + gen-check + unit (`make ci`).
2. Acceptance.
3. Dependency audit.
4. (Add) kit contracts, secret scan, SBOM, image scan.
5. (Add) integration against a disposable Postgres.
6. Deploy with a rollback path.

A stage that can be skipped silently is not a gate.

---

## 11. Compliance evidence (SOC 2 and ISO/IEC 27001)

Map a control to a command and an artifact; an auditor accepts a repeatable
command, not a claim.

### 11.1 Control-to-test mapping (starting point)

| Control area             | Evidence in this repo                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Change management        | PR diff + `make ci` output + review approval                                |
| Segregation of duties    | `CODEOWNERS`/ownership table in `AGENTS.md`; human-only control plane       |
| Access control           | `authz` unit tests + admin acceptance scenarios + `PLAYBOOK_SECURITY.md` §5 |
| Cryptographic controls   | `authx.test.ts` (HS256 pinning, hashing) + config secret rules              |
| Boundary validation      | `httpx.decode` unit/acceptance + body-limit scenario (add)                  |
| Logging and monitoring   | request-id hook + Pino config + health/readiness probes                     |
| Vulnerability management | `pnpm audit` job + dependency review                                        |
| Backup and recovery      | Postgres backup config (outside this repo) + restore drill                  |
| Incident response        | `PLAYBOOK_SECURITY.md` §13 + runbook                                        |
| Supplier management      | lockfile/SBOM policy + `pnpm audit`                                         |

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

### 11.4 Release evidence bundle

```
release-<version>/
├── ci.log                 # make ci output
├── accept.log             # cucumber summary
├── audit.log              # pnpm audit
├── sbom.json              # when added
├── image-digest.txt       # when built
└── approvals.md           # reviewers + timestamp
```

### 11.5 Change-management gates

- No direct push to `main`; a PR with a green gate.
- Schema changes carry a migration and a rollback note.
- Security-sensitive changes (auth, crypto, config, CI) need a second
  reviewer.
- Emergency changes get a retrospective ticket within the next business day.

### 11.6 Exceptions and waivers

An exception is a record, not a comment: what control, why, compensating
measure, owner, expiry. Expired exceptions fail the next review by default.

---

## 12. Concrete example: updating a user as an admin

What a complete change looks like:

1. **Unit** (`adminusers/usecase.test.ts`): an ADMIN cannot update another
   ADMIN (`canManageSubordinate` strict `>`); a stale token whose rank is
   below the row's DB role is refused; an unknown id is `NOT_FOUND`.
2. **Acceptance** (`adminusers.feature`): `PUT /api/v1/admin/users/:id` by a
   USER → 403 `AUTH_FORBIDDEN`; by an ADMIN on a USER → 200 and the row
   changed; by an ADMIN on an ADMIN → 403; unknown id → 404.
3. **Contract**: the request/response shape stays in `specs/openapi.yaml`;
   `make generate` leaves `src/gen/api` consistent.
4. **Audit**: the handler writes `audit_logs` (actor, action, entity) in the
   same code path as the change; an acceptance step asserts the row exists.
5. **Gate**: `make ci` and `make accept` green; the PR includes both logs.

If any of those is missing, the change is not done — it is merely compiling.

---

## 13. Checklist

- [ ] New behavior has a unit test at the layer that owns the rule.
- [ ] New or changed route has an acceptance scenario, including a failure.
- [ ] Failure assertions check `code`, not only status.
- [ ] Time is injected; no sleeps, no wall-clock dependence.
- [ ] No real network, Postgres, SMTP or Redis in unit or acceptance.
- [ ] A bug fix ships with the test that fails without it.
- [ ] `make ci` and `make accept` green; output read, not assumed.
- [ ] Kit-contract evals green after `.claude/**` changes.
- [ ] New tests are deterministic in parallel and under shuffled order
      (`vitest --sequence.shuffle`).
- [ ] Evidence artifacts retained for release.
