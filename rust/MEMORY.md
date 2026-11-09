# MEMORY.md — product premise (Rust backend)

Facts an agent needs before touching the code, and cannot cheaply derive from
a single file. Verify anything here against the migrations and `router.rs`
before acting on it; if they disagree, the code wins and this file is wrong.

---

## 1. What `<app>` is

`<app>` is a **backend skeleton**, generated from the `backend-rust` template:
Axum 0.8 + sqlx 0.8 + Postgres, a crate-per-concern workspace with hexagonal
ports and vertical slices. It ships an example `widgets` slice plus a working
identity stack (signup, email verification, login, sessions, password reset,
OAuth) and an admin roster surface.

It is not a product and not multi-tenant. There is no tenant id, no
`{userType}` path segment, no frontend, no message broker, and no
multi-replica realtime fan-out. Those are deliberate omissions, not hidden
features.

After instantiation, every `<app>` and `<scope>` token is replaced by
`scripts/rename.sh <app-slug> <scope>`; the template ships them literally
because a literal `<app>` is not a valid crate name.

---

## 2. Actors and roles

| Actor | Can do |
|---|---|
| Unauthenticated visitor | signup, verify email, login, refresh, forgot/reset password, Google/GitHub sign-in |
| Authenticated user (Viewer/Editor) | read and mutate its own widgets, read its own identity, manage its sessions |
| Admin (Admin/Owner rank) | the admin roster: list users, patch a lower-ranked user, revoke a lower-ranked user's sessions, read anomaly counters |
| Bootstrap operator | promote its own account via `/api/v1/auth/upgrade-to-admin`, only if its email is in `<APP>_ALLOWED_ADMIN_EMAILS` |

Roles are `Viewer < Editor < Admin < Owner` in `crates/<app>-core/src/rbac.rs`
(`can()`); `Owner` is the unrestricted role. The database role strings are
`ADMIN/MODERATOR/USER/GUEST`: `USER` and `GUEST` → `Viewer`, `MODERATOR` →
`Editor`, `ADMIN` → `Admin`; `Owner` has no DB string and is stored as
`ADMIN`. The admin roster gate is rank-based (`<app>-adminusers`): it maps
`Admin` and `Owner` to the same rank 3 and requires a strict `>`, so an admin
cannot manage another admin, while an `Editor` or `Viewer` cannot reach the
roster at all. Authorization reads the role, never `user_type`.

---

## 3. Crates and the tables they own

| Crate | Owns | Tables |
|---|---|---|
| `<app>-core` | domain types, `Error`, RBAC | none |
| `<app>-ports` | repository traits, Pg adapters, mocks — **the only SQL home** | all of the below, via adapters |
| `<app>-widgets` | example slice | `widgets` |
| `<app>-auth` | signup, verification, sessions, password, OAuth | `users`, `companies`, `users_profile`, `sessions`, `email_verifications`, `password_resets`, `email_jobs` |
| `<app>-adminusers` | admin roster, rank gate, audit write | `users`, `audit_logs` |
| `<app>-warden` | HS256 JWT, argon2, cookies, OAuth clients, auth middleware | none |
| `<app>-mailer` | `Mailer` implementations | none |
| `<app>-loom` | realtime hub, outbox drain, janitor, worker binary | `email_jobs`; janitor deletes expired `sessions`, `email_verifications`, `password_resets` |
| `<app>-api` | composition root: config, `AppState`, router, routes, error mapping | none directly |
| `<app>-sdk`, `<app>-cli` | typed client and CLI | none |

`settings` exists in the schema (`0002_identity_core.up.sql`) but no shipped
slice writes to it. `audit_logs` is written by the admin slice; treat it as
append-only.

Dependency direction: `api → any`; `loom → slices, ports, core, mailer,
warden`; a slice → `ports`, `core` (and `warden` for auth); `ports → core`;
`mailer`/`sdk`/`cli → core` (`cli → sdk`); `core → nothing`. Consumers hold
`Arc<dyn Trait>`; a concrete `Pg*` type appears only in the composition root
and the adapter module.

---

## 4. Stack

| Concern | Choice |
|---|---|
| Language | Rust, edition 2024, MSRV 1.88, toolchain pinned to 1.93.1 |
| HTTP | axum 0.8, tower-http 0.6 |
| Database | Postgres 17, sqlx 0.8 with **runtime** queries (no `.sqlx/`, no `query_as!`) |
| Migrations | paired `NNNN_*.up.sql` / `.down.sql`, embedded via `sqlx::migrate!`, applied by the `migrate` subcommand |
| Async | tokio |
| Errors | `thiserror` in libraries, `anyhow` at the binary edge |
| API docs | utoipa 5 + Swagger UI; `openapi.json` generated from code, guarded by `just gen-check` |
| Auth | argon2id (`Params::DEFAULT`) over `base64(HMAC-SHA256(password, pepper))`; HS256 JWT with pinned `kid` and audience; opaque refresh tokens |
| Observability | `tracing`, request id `req_<uuid>`, optional Prometheus metrics |
| Tests | nextest; mocks behind `feature = "mock"`; Postgres service in CI |
| Task runner | `just` |

---

## 5. Load-bearing invariants

1. **Dependency direction** — the table above; a cycle does not compile, but a
   slice→slice import is a review defect even when it does.
2. **SQL lives only in `crates/<app>-ports/src/adapters/`**, and every value
   is bound with `$n`, never interpolated. There is no compile-time schema
   check — adapter tests are the check.
3. **Paired migrations**, monotonic numbering, expand/contract for anything
   destructive.
4. **`unsafe_code = "forbid"`** workspace-wide. A suppression uses
   `#[expect(lint, reason = "…")]`, never a bare `#[allow]`.
5. **Offline guard (INV-T7)** — `<APP>_OFFLINE=true` selects `NoopMailer`
   unconditionally and opens no outbound socket. SMTP credentials are
   rejected in offline mode.
6. **Error envelope** — `{"error": {code, title, detail, request_id, extra}}`;
   a 5xx body is redacted to `"An internal error occurred."`.
7. **Auth** — access JWT ≤ 900s, `exp >= iat`, `jti` required, `kid` required
   and known; the refresh token is rotated with a CAS update; the WS route
   authenticates after upgrade and never logs `?token=`.
8. **Two auth middlewares** — `require_auth` (bearer only) and
   `require_auth_cookie` (bearer, then the cookie). A browser-facing route
   uses the latter.
9. **Control plane is human-only** — `.claude/settings.json`, the manifest,
   schemas and `scripts/**` are not agent-editable.
10. **`just ci` green before "done"** — plausibility is not correctness.

---

## 6. Gates

```sh
just ci          # fmt-check + clippy -D warnings + check + nextest + contract evals
just gen-check   # openapi.json is not stale
just audit       # cargo-audit + cargo-deny + cargo-machete
just migrate     # apply the embedded migrations (needs <APP>_DATABASE_URL)
```

CI: `fmt`, `clippy`, `test` (Postgres + migrate + nextest), `audit`, `eval`.

---

## 7. Known gaps (do not mistake for features)

- Widget CRUD is mounted at `/widgets`, not `/api/v1/widgets` (deviation D1).
- `GET /me` duplicates `/api/v1/auth/me` (D2).
- Widget routes are bearer-only, so a cookie-mode SPA gets 401 (D3).
- `GET /widgets` is unpaginated (D4).
- `/metrics` and `/docs` are unguarded (D5).
- `docs/02`, `docs/08`, `docs/09` describe a different stack (compile-time
  `query_as!`, `.sqlx/`, testcontainers) — the playbooks are authoritative (D6).
- Login returns early on an unknown email, which is a timing oracle.
- Revocation is in-memory and single-process; the outbox/realtime hub is
  single-process.
- There is no coverage tool, no mutation testing, no secret scanner, no
  container/SBOM scan.

---

## 8. Contract pointers

| Question | Read |
|---|---|
| Crate roles, ports, SQL, workers, realtime | `PLAYBOOK_BACKEND.md` |
| Routes, namespaces, error envelope, streaming | `PLAYBOOK_URLS.md` |
| Test tiers, gates, audit evidence | `PLAYBOOK_TESTS.md` |
| Threat model, authn/authz, secrets, supply chain | `PLAYBOOK_SECURITY.md` |
| Self-hosting an audit program | `PLAYBOOK_AUDIT.md` |
| The Claude Code kit and its governance | `PLAYBOOK_CLAUDE_CODE.md` |
| How to work in the repo | `AGENTS.md` |
