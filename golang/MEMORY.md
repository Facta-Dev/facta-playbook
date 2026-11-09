# MEMORY.md — `<app>` premise

Durable, versioned premise for every session. Keep it under ~100 lines: facts
an agent cannot derive quickly from the code, not a copy of the code.

## Product

`<app>` is a self-hosted Go backend: one HTTP API, one worker process, one
Postgres database, one `docker compose` stack. It ships a full auth core so a
new product can start on a working identity layer. There is no multi-tenancy
and no organization model: a row belongs to one user.

## Actors

| Actor | Role | Surface |
|---|---|---|
| Anonymous visitor | no session | `/api/v1/auth/*` (signup, login, password reset, OAuth) |
| User | `USER` (or `GUEST` before verification) | `/api/v1/*` behind `authx.RequireAuth` |
| Moderator | `MODERATOR` | user surface + moderation routes when a slice adds them |
| Admin | `ADMIN` | `/api/v1/admin/*` behind `authx.RequireRole(rolex.Admin)` |
| Worker | no HTTP | `cmd/worker`: outbox drain + janitor |

Role hierarchy: `ADMIN ⊃ MODERATOR ⊃ USER ⊃ GUEST`
(`internal/shared/rolex`). `users.user_type` is a signup/profile
classification, **not** an authorization input.

## Slices

| Slice | Owns | Notes |
|---|---|---|
| `authsignup` | `users`, `companies`, `users_profile`, `email_verifications` | idempotent signup, verify, resend |
| `authsession` | `sessions` | login, refresh rotation, logout, me, session lists, anomalies |
| `authpassword` | `password_resets` | forgot / validate / reset / change |
| `authoauth` | no table of its own — writes `users.google_id` / `users.github_id` | Google One Tap (JWKS), GitHub auth-code |
| `adminusers` | `audit_logs` | admin user CRUD, self-or-subordinate guard |
| `widgets` | `widgets` | **example slice** — replace with the real domain |
| `realtime` | — | in-process hub, `/api/v1/ws?token=` |

Cross-slice reads go through the owner's port or an event, never an import.
Only `users(id)` is referenced across slices.

## Stack

Go 1.27 · stdlib `net/http` ServeMux (method-first patterns) · `pgx/v5` +
`pgxpool` · `sqlc` (queries per slice, schema from `migrations/`) · `goose`
· `oapi-codegen` (models only) · `godog` · `testcontainers-go` ·
`golangci-lint` v2 with `depguard`.

## Invariants worth remembering

- **INV-T3**: no `go.sum` in the template; `go mod tidy` post-rename.
- **INV-T7**: `<APP>_OFFLINE=true` forces the console mailer and forbids
  outbound sockets (OAuth, SMTP). `config.Load` rejects OFFLINE + SMTP
  credentials.
- Passwords: bcrypt cost 10 over HMAC-SHA256(password, pepper).
- Tokens: HS256 access 15m; rotating opaque refresh 720h; verification 24h;
  reset 30m.
- Migrations: expand/contract, applied by the deploy step, never at API
  startup.
- Errors: stable codes in `internal/shared/apperr` → `{detail, code}` at the
  HTTP edge only.

## Gates

`make ci` (fmt, vet, lint, gen-check, unit) · `make test-int` (testcontainers)
· `make accept` (godog, fake-wired mux) · `scripts/verify-template.sh`
(template factory only).

## Where the contracts live

`PLAYBOOK_BACKEND.md` · `PLAYBOOK_TESTS.md` · `PLAYBOOK_SECURITY.md` ·
`PLAYBOOK_URLS.md` · `PLAYBOOK_AUDIT.md` · `PLAYBOOK_CLAUDE_CODE.md` ·
`docs/01..14-*.md` (architecture) · `AGENTS.md` (agent workflow).
