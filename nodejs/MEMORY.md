# MEMORY.md — product premise (Node.js backend)

Facts an agent needs before touching the code, and cannot cheaply derive from
a single file. Verify anything here against the migrations and the slice
`module.ts` files before acting on it; if they disagree, the code wins and
this file is wrong.

---

## 1. What `<app>` is

`<app>` is a **backend skeleton**, copied from the `backend-node` template
(`templates/package_builder/templates/backend-node/skeleton`). It is a single
Node 24 ESM package: TypeScript 5.9 strict, Fastify 5, `pg`, Kysely typed
queries, node-pg-migrate, zod at the edges, jose HS256, bcryptjs, an email
outbox, WebSocket + SSE, and a full identity core.

It is not a product and not multi-tenant. There is no tenant id, no
`{userType}` path segment, no frontend, no Redis/BullMQ broker, and no
multi-replica realtime fan-out. Those are deliberate omissions, not hidden
features.

After instantiation, every `<app>` / `<APP>` / `<scope>` token is replaced by
`bash scripts/rename.sh <app-slug> <scope>`. The template ships them literally
because the package name `<app>` is not installable.

The language rules come from the **beautifulnodejs** skill pack
(`Facta-Dev/beautifulnodejs`): strict TS, invalid states unrepresentable,
zero-I/O domain, one error envelope, durable side effects via an outbox,
Pino-structured logging, table-driven tests. Its published stack digest names
Drizzle + BullMQ + tsyringe; this runnable skeleton predates that digest and
uses Kysely + a DB outbox + constructor injection. The invariants are the
contract; the library names are the delta (§8).

---

## 2. Actors and roles

| Actor                   | Can do                                                                                                                 |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Unauthenticated visitor | signup, verify email, resend code, login, refresh, forgot/validate/reset password, Google One Tap, GitHub sign-in      |
| Authenticated user      | read/update own profile, list/revoke own sessions, CRUD own widgets, read the SSE stream                               |
| Admin                   | list/get/update users, read/revoke another user's sessions, read the multi-IP anomaly report                           |
| Bootstrap operator      | promote its own account via `POST /api/v1/auth/upgrade-to-admin`, only if the email is in `<APP>_ALLOWED_ADMIN_EMAILS` |

Roles are `ADMIN > MODERATOR > USER > GUEST` (`src/shared/rolex.ts`).
`requireRole(min)` uses `atLeast` (rank ≥ min). The admin user surface uses
`requireRole(Admin)` **and** the shared `authz` guard: `canManageSelf` requires
the token role to still rank at-or-above the row's DB role, and
`canManageSubordinate` requires a **strict** `>` — an admin cannot manage
another admin. Authorization reads the role, never `user_type`.

---

## 3. Slices and the tables they own

| Slice                       | Responsibility                                                        | Tables                                                                             |
| --------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `src/features/authsignup`   | signup, verify, resend                                                | `users`, `email_verifications`, `email_jobs` (outbox)                              |
| `src/features/authsession`  | login, refresh, logout, me, own + admin sessions, anomalies           | `users`, `sessions`                                                                |
| `src/features/authpassword` | forgot, validate, reset, change                                       | `users`, `password_resets`, `sessions`, `email_jobs`                               |
| `src/features/authoauth`    | Google One Tap, GitHub auth-code                                      | `users`, `sessions`                                                                |
| `src/features/adminusers`   | admin roster + audit                                                  | `users`, `audit_logs`                                                              |
| `src/features/widgets`      | example CRUD + SSE lifecycle events                                   | `widgets`                                                                          |
| `src/features/realtime`     | WebSocket upgrade + per-connection pump                               | none                                                                               |
| `src/bin/api.ts`            | composition root                                                      | none directly                                                                      |
| `src/bin/worker.ts`         | outbox + janitor loops                                                | `email_jobs`; deletes expired `sessions`, `email_verifications`, `password_resets` |
| `src/platform/*`            | transversal infra (authx, oauthx, email, realtime, config, httpx, pg) | none                                                                               |
| `src/shared/*`              | kernel (apperr, rolex, authz, clockx, idgen)                          | none                                                                               |

`settings` exists in `0001_init` but no shipped slice reads or writes it.
`audit_logs` is written by the admin slice; treat it as append-only.

Dependency direction: `src/bin/* → any`; a slice's `http.ts`/`postgres.ts`/
`module.ts` → own slice + `#platform/*` + `#shared/*`; `usecase.ts` →
`ports.ts` + `domain.ts`; `ports.ts` → `domain.ts`; `domain.ts` → Node
stdlib + `#shared/*` only; `src/platform/*` → `#shared/*` + stdlib + `pg`,
never `#features/*`; nothing imports `src/bin/*`.

---

## 4. Stack

| Concern         | Choice                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Language        | TypeScript 5.9, `strict`, `noUncheckedIndexedAccess`, ESM, Node ≥ 24                                                                   |
| HTTP            | Fastify 5 (`@fastify/cookie`), hooks for request-id / security headers / CORS                                                          |
| Validation      | zod 4 at the edges (HTTP bodies, config) — never in the domain                                                                         |
| Database        | Postgres 17, `pg` Pool; one pool per process, built at the root                                                                        |
| Typed SQL       | Kysely 0.29 — per-slice `queries.ts` over a slice-local `Database` map                                                                 |
| Migrations      | node-pg-migrate 9, numbered `.mjs` files exporting `up` + `down`; never at startup                                                     |
| API models      | `specs/openapi.yaml` → openapi-typescript → committed `src/gen/api/schema.d.ts`                                                        |
| Auth            | jose HS256 access JWT (15m) + opaque rotating refresh (720h); bcryptjs cost 10 over HMAC-SHA256 pre-hash + pepper; 6-digit email codes |
| Email           | nodemailer SMTP sender + console sender; durable outbox rows in `email_jobs`; polling worker with jittered backoff + dead-letter       |
| Realtime        | in-process hub (`EventBus`), `ws` upgrade at `/api/v1/ws`, SSE at `/api/v1/widgets/events`                                             |
| Logging         | Fastify's Pino logger (JSON); request id echoed as `x-request-id`                                                                      |
| Tests           | Vitest (unit), `@cucumber/cucumber` over a fake-wired real app (acceptance), `node --test` (kit contract evals)                        |
| Task runner     | `make` (`make ci` is the gate)                                                                                                         |
| Package manager | pnpm via corepack; **no `pnpm-lock.yaml` in the template** (INV-T3)                                                                    |

---

## 5. Load-bearing invariants

1. **Dependency direction** — the table in §3. ESM tolerates cycles, so
   eslint's `no-restricted-imports` groups (`domain-pure`, `slice-isolation`)
   are the enforcement, not the module system.
2. **The domain is zero-I/O** — `domain.ts` imports Node builtins and
   `#shared/*` only. No fastify, pg, kysely, zod, platform, gen, or other
   slice.
3. **Ports are consumer-defined** — the use case declares the interface it
   needs in its `ports.ts` and stores it as a field type, never an adapter
   type. Adapters implement structurally.
4. **SQL lives only in a slice's `queries.ts`** and is consumed only by that
   slice's `postgres.ts`. No raw SQL in `usecase.ts` or `http.ts`.
5. **Paired migrations** — every migration file exports both `up` and `down`;
   an empty `down` is a review block. Migrations never run at server startup.
6. **One error envelope** — every non-2xx body is `{"detail","code"}` written
   by the root's `setErrorHandler` + `httpx`; stable codes live in
   `src/shared/apperr.ts`. Adapters map driver errors inward once.
7. **Durable side effects ride the outbox** — a slice's use case inserts a
   rendered-intent (`templateKey` + vars) row into `email_jobs` through an
   `EmailEnqueuer` port; the worker relays it. Never send mail inline from a
   request path. **Caveat (shipped state):** the insert uses its own pool
   connection, so it is _not_ atomic with the state change — a crash between
   the two loses the mail, and the reverse can mail a rolled-back state. The
   upgrade is a transaction-capable port or an adapter that takes the caller's
   tx; do not claim atomicity until the code has it.
8. **Offline guard (INV-T7)** — `<APP>_OFFLINE=true` selects the console
   sender and forbids SMTP credentials; OAuth flows refuse at the request
   boundary instead of dialing out.
9. **Auth** — access JWT ≤ configured TTL, issuer checked, HS256 only; the
   refresh token rotates with a compare-and-swap on the old token; cookie
   mode is HttpOnly + SameSite=strict + Secure outside `dev`.
10. **Control plane is human-only** — `.claude/settings.json`,
    `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**` and CI files
    are not agent-editable without an explicit request.
11. **`make ci` green before "done"** — plausibility is not correctness.
12. **English only** everywhere; never put a comment on a `.gitignore`
    pattern line (git reads `#` only at line start).

---

## 6. Gates

```sh
make ci          # prettier check + tsc --noEmit + eslint + gen-check + vitest
make accept      # cucumber over the fake-wired app (no DB)
make generate    # regenerate src/gen/api from specs/openapi.yaml
make gen-check   # fail if the committed generated models are stale
make migrate-up  # apply migrations locally (needs <APP>_DATABASE_URL)
make vuln        # pnpm audit --audit-level=high
bash scripts/verify-template.sh   # staged renamed copy + full factory smoke
```

CI (`.github/workflows/ci.yml`) runs `make ci`, then unit + acceptance,
`pnpm audit --audit-level=high`, and actionlint.

---

## 7. Known gaps (do not mistake for features)

- `docs/11-testing-and-ci.md` and `docs/12-playbooks.md` mention
  `make test-int` and a testcontainers integration lane; the shipped Makefile
  has **no `test-int` target** and no integration suite. The playbooks are
  authoritative: unit fakes + acceptance today; add an integration lane
  deliberately.
- `src/features/authsignup`, `authsession`, `authpassword`, `authoauth` and
  `realtime` have no `*.test.ts`; their behavior is covered only by cucumber
  acceptance scenarios. `adminusers` and `widgets` have unit tests.
- No coverage threshold, no mutation testing, no secret scanner, no SBOM or
  container vulnerability scan in CI.
- Realtime is single-process. WS and SSE only see events published in the
  same process; the documented evolution is a Redis pub/sub adapter behind
  `EventBus`.
- The template ships no lockfile (INV-T3). After instantiation, decide whether
  to commit `pnpm-lock.yaml` (reproducible builds) and remove the ignore
  entry if you do.
- `POST /api/v1/auth/upgrade-to-admin` is unauthenticated by design; it is
  safe only while `ALLOWED_ADMIN_EMAILS` is set/empty as intended and should
  be removed after bootstrap.
- The worker entrypoint uses `console.*` for its own loops (allowed: bounded
  entrypoint output); the request path must use the Fastify/Pino logger.
- The stack differs from beautifulnodejs's published digest: Kysely instead
  of Drizzle, a DB outbox instead of BullMQ/Redis, plain constructor
  injection instead of tsyringe. Swapping any of them is an adapter change;
  do not change the contract.

---

## 8. Contract pointers

| Question                                         | Read                      |
| ------------------------------------------------ | ------------------------- |
| Slices, edges, ports, SQL, outbox, realtime      | `PLAYBOOK_BACKEND.md`     |
| Routes, postures, envelopes, streaming           | `PLAYBOOK_URLS.md`        |
| Test tiers, gates, audit evidence                | `PLAYBOOK_TESTS.md`       |
| Threat model, authn/authz, secrets, supply chain | `PLAYBOOK_SECURITY.md`    |
| Self-hosting an audit program                    | `PLAYBOOK_AUDIT.md`       |
| The Claude Code kit and its governance           | `PLAYBOOK_CLAUDE_CODE.md` |
| How to work in the repo                          | `AGENTS.md`               |
