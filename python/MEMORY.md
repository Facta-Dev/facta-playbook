# MEMORY.md — product premise (Python backend)

Facts an agent needs before touching the code, and cannot cheaply derive from
a single file. Verify anything here against the migrations and the slice
`module.py` files before acting on it; if they disagree, the code wins and
this file is wrong.

---

## 1. What `<app>` is

`<app>` is a **backend skeleton**, copied from the `backend-python` template.
It is a single uv project: Python 3.12, FastAPI, pydantic v2 / pydantic
settings, SQLAlchemy 2.0 async + asyncpg, Alembic, PyJWT HS256, argon2id,
httpx, an email outbox, WebSocket + SSE, and a full identity core. The import
package is `app`; the distribution is `app-template` until `scripts/rename.sh`
rewrites it.

It is not a product and not multi-tenant. There is no tenant id, no
`{userType}` path segment, no frontend, no Redis/TaskIQ broker, and no
multi-replica realtime fan-out. Those are deliberate omissions, not hidden
features.

After instantiation, every `<app>` / `<APP>` / `<scope>` token is replaced by
`bash scripts/rename.sh <app-slug> <scope>`. The template ships them literally
because the env prefix and the distribution name are not usable as-is.

The language rules come from the **beautifulpython** skill pack
(`Facta-Dev/beautifulpython`): mypy strict, invalid states unrepresentable,
zero-I/O domain, one error envelope, durable side effects via an outbox,
structured logging, faked-port tests. Its published stack digest names
TaskIQ + Dragonfly; this runnable skeleton uses a DB outbox and no Redis. The
invariants are the contract; the library names are the delta (§8).

---

## 2. Actors and roles

| Actor                   | Can do                                                                                                                 |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Unauthenticated visitor | signup, verify email, resend code, login, refresh, forgot/validate/reset password, Google sign-in, GitHub sign-in      |
| Authenticated user      | read/update own profile, list/revoke own sessions, CRUD own widgets, read the SSE stream                               |
| Admin                   | list/get/update users, read/revoke another user's sessions, read the multi-IP anomaly report                           |
| Bootstrap operator      | promote its own account via `POST /api/v1/auth/upgrade-to-admin`, only if the email is in `<APP>_ALLOWED_ADMIN_EMAILS` |

Roles are `guest < user < moderator < admin` (`src/app/shared/rolex.py`, a
`StrEnum`). `require_role(min)` uses `at_least` (rank ≥ min). The admin user
surface uses `require_role(Role.ADMIN)` **and** the shared `authz` guard:
`can_manage_self` requires the token role to still rank at-or-above the row's
DB role, and `can_manage_subordinate` requires a **strict** `>` — an admin
cannot manage another admin. Authorization reads the role, never a
`user_type`.

---

## 3. Slices and the tables they own

| Slice                              | Responsibility                                                        | Tables                                                                             |
| ---------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `src/app/features/authsignup`      | signup, verify, resend                                                | `users`, `email_verifications`, `email_jobs` (outbox)                              |
| `src/app/features/authsession`     | login, refresh, logout, me, own + admin sessions, anomalies           | `users`, `sessions`                                                                |
| `src/app/features/authpassword`    | forgot, validate, reset, change                                       | `users`, `password_resets`, `sessions`, `email_jobs`                               |
| `src/app/features/authoauth`       | Google ID token, GitHub auth-code                                     | `users`, `sessions`                                                                |
| `src/app/features/adminusers`      | admin roster + audit                                                  | `users`, `audit_logs`                                                              |
| `src/app/features/widgets`         | example CRUD + SSE lifecycle events                                   | `widgets`                                                                          |
| `src/app/features/realtime`        | WebSocket upgrade + per-connection pump                               | none                                                                               |
| `src/app/bin/api.py`               | composition root                                                      | none directly                                                                      |
| `src/app/bin/worker.py`            | outbox + janitor loops                                                | `email_jobs`; deletes expired `sessions`, `email_verifications`, `password_resets` |
| `src/app/platform/*`               | transversal infra (authx, oauthx, email, realtime, config, db, envelope, logging) | none                                                                    |
| `src/app/shared/*`                 | kernel (apperr, rolex, authz, clockx, idgen)                          | none                                                                               |

`audit_logs` is written by the admin slice; treat it as append-only. The
second migration (`0002_profiles`) adds `users.profile_picture` and the
`sessions(expires_at)` / `email_jobs(status)` indexes.

Dependency direction: `app.bin.* → any`; a slice's `http.py`/`postgres.py`/
`module.py` → own slice + `app.platform.*` + `app.shared.*`; `usecase.py` →
`ports.py` + `domain.py` + platform + shared; `ports.py` → `domain.py`;
`domain.py` → Python stdlib + `app.shared.*` only; `app.platform.*` →
`app.shared.*` + stdlib + SQLAlchemy, never `app.features.*`; nothing imports
`app.bin.*`.

---

## 4. Stack

| Concern         | Choice                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Language        | Python 3.12, `from __future__ import annotations`, mypy `strict`                                                                       |
| HTTP            | FastAPI 0.115 (`uvicorn[standard]`), middleware for request-id / security headers / CORS                                               |
| Validation      | pydantic v2 / pydantic-settings at the edges (HTTP bodies, config) — never in the domain                                               |
| Database        | Postgres 17, SQLAlchemy 2.0 async engine + asyncpg; one engine per process, built at the root                                          |
| Typed SQL       | SQLAlchemy Core `text()` per slice `queries.py`; row mapping in `postgres.py`                                                          |
| Migrations      | Alembic 1.14, `migrations/versions/` files exporting `upgrade` + `downgrade`; never at startup                                         |
| API models      | `specs/openapi.yaml`, **exported from the app** by `scripts/export_openapi.py` (`make generate`) and committed                         |
| Auth            | PyJWT HS256 access (15m, claims sub/email/role/iss/iat/exp, no `jti`) + opaque rotating refresh (720h); argon2id over `plain + pepper`; 6-digit email codes |
| Email           | SMTP sender + console sender; durable outbox rows in `email_jobs`; polling worker with backoff + dead-letter                            |
| Realtime        | in-process hub, WS upgrade at `/api/v1/ws`, SSE at `/api/v1/widgets/events`                                                            |
| Logging         | stdlib `logging` with a JSON formatter; `request_id` bound through a `ContextVar`, echoed as `x-request-id`                            |
| Tests           | pytest + pytest-asyncio (unit), pytest-bdd over a fake-wired real app (acceptance), pytest (kit contract evals)                        |
| Task runner     | `make` (`make ci` is the gate)                                                                                                         |
| Tooling         | uv; **no `uv.lock` in the template** (INV-T3); ruff (format + lint), mypy, import-linter, pip-audit                                     |

---

## 5. Load-bearing invariants

1. **Dependency direction** — the table in §3. Python tolerates import cycles,
   so the import-linter contracts in `pyproject.toml` (`layers`,
   `forbidden`, `independence`) are the enforcement, not the module system.
2. **The domain is zero-I/O** — `domain.py` imports Python stdlib and
   `app.shared.*` only. No FastAPI, SQLAlchemy, asyncpg, Alembic, httpx,
   uvicorn, platform, or other slice.
3. **Ports are consumer-defined** — the use case declares the interface it
   needs in its `ports.py` (a `typing.Protocol`) and stores it as a field
   type, never an adapter type.
4. **SQL lives only in a slice's `queries.py`** and is consumed only by that
   slice's `postgres.py`. No raw SQL in `usecase.py` or `http.py`. This one is
   review-enforced; import-linter enforces the layer edges.
5. **Paired migrations** — every revision implements both `upgrade` and
   `downgrade`; an empty `downgrade` is a review block. Migrations never run
   at server startup.
6. **One error envelope** — every non-2xx body is `{"detail","code"}` written
   by `install_error_handlers` (AppError, validation, HTTP, and a catch-all
   `Exception` → 500 `INTERNAL`). Stable codes live in
   `src/app/shared/apperr.py`. Adapters map driver errors inward once.
7. **Durable side effects ride the outbox** — the slice's adapter stages a
   rendered-intent row (`template_key` + `payload`) in `email_jobs` by calling
   `enqueue_email(session, …)` on the caller's `AsyncSession`; the worker
   relays it. Never send mail inline from a request path. **The enqueue is
   atomic**: the adapter commits the business write and the job in one
   transaction.
8. **Offline guard (INV-T7)** — `<APP>_OFFLINE=true` selects the console
   sender and forbids SMTP credentials at boot; OAuth socket paths refuse with
   `AUTH_OFFLINE` (503) at the request boundary instead of dialing out.
9. **Auth** — access JWT ≤ configured TTL, issuer checked, HS256 only; the
   refresh token rotates with a compare-and-swap on the old token; bearer only
   (no cookie mode).
10. **Control plane is human-only** — `.claude/settings.json`,
    `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**` and CI files
    are not agent-editable without an explicit request.
11. **`make ci` green before "done"** — plausibility is not correctness.
12. **English only** everywhere; never put a comment on a `.gitignore`
    pattern line (git reads `#` only at line start).

---

## 6. Gates

```sh
make ci          # ruff format --check + mypy + ruff/import-linter + gen-check + unit + acceptance + evals
make accept      # pytest-bdd over the fake-wired app (no DB)
make generate    # regenerate specs/openapi.yaml from the app
make gen-check   # fail if the committed OpenAPI contract is stale
make migrate-up  # apply migrations locally (needs <APP>_DATABASE_URL)
make vuln        # pip-audit
bash scripts/verify-template.sh   # staged renamed copy + full factory smoke
```

CI (`.github/workflows/ci.yml`) runs `make ci`, then `make vuln`, and
actionlint.

---

## 7. Known gaps (do not mistake for features)

- No integration lane ships: `make test` is unit-only with fakes and
  `make accept` uses the fake-wired app, so **no gate executes real SQL**.
  `verify-template.sh` is the only migration smoke (throwaway Postgres).
- `authoauth` and `realtime` have no unit test; their behavior is covered by
  the acceptance scenarios (`authoauth.feature`, the widgets SSE scenario, and
  the WS close test in `test_http.py`).
- No coverage threshold, no mutation testing, no secret scanner, no SBOM or
  container vulnerability scan in CI.
- Realtime is single-process. WS and SSE only see events published in the
  same process; the documented evolution is a Redis pub/sub adapter behind the
  hub.
- The template ships no lockfile (INV-T3). After instantiation, decide whether
  to commit `uv.lock` (reproducible builds) and remove the ignore entry if you
  do.
- `POST /api/v1/auth/upgrade-to-admin` is unauthenticated by design; it is
  safe only while `ALLOWED_ADMIN_EMAILS` is set/empty as intended and should
  be removed after bootstrap.
- The worker entrypoint owns its own loop output; the request path must log
  through `app/platform/logging.py`.
- The stack differs from beautifulpython's published digest: SQLAlchemy Core
  `text()` instead of ORM entities, a DB outbox instead of TaskIQ/Dragonfly.
  Swapping either is an adapter change; do not change the contract.

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
