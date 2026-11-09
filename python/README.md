# `<app>` — Python backend skeleton

A hexagonal, vertically-sliced Python 3.12 backend template: FastAPI,
pydantic v2, SQLAlchemy 2.0 async + asyncpg, Alembic, PyJWT HS256, argon2id,
full auth core, DB outbox email worker, WebSocket + SSE, pytest/pytest-bdd
acceptance over a fake-wired app, and import-linter contracts as the
dependency law. Every `<app>`, `<APP>` and `<scope>` token is a **literal
placeholder** — the config prefix and the package metadata are invalid until
you rename, so the project cannot boot as shipped. The template also ships no
`uv.lock` (INV-T3). See [How to apply](#how-to-apply).

This directory is both the runnable skeleton and the documentation set: the
six `PLAYBOOK_*.md` contracts, the numbered `docs/`, and the `.claude/` kit
govern the code in `src/app/`.

## Layout

```
<app>/
├── src/app/
│   ├── bin/
│   │   ├── api.py           composition root: env → engine → adapters → register → serve
│   │   ├── middleware.py    request-id · security headers · CORS
│   │   └── worker.py        outbox (email_jobs) + janitor loops — same image, different entry
│   ├── features/            seven vertical slices (never import each other)
│   │   ├── authsignup/      signup · verify · resend (email_verifications)
│   │   ├── authsession/     login · refresh · logout · me · sessions + anomalies
│   │   ├── authpassword/    forgot · validate · reset · change (password_resets)
│   │   ├── authoauth/       Google ID token + GitHub auth-code social sign-in
│   │   ├── adminusers/      admin user surface + audit_logs (ADMIN only)
│   │   ├── widgets/         example slice: CRUD + SSE lifecycle events
│   │   └── realtime/        WebSocket /api/v1/ws?token= (module only; hub lives in platform)
│   ├── platform/            transversal infra: authx · config · db · envelope · logging
│   │                        oauthx · email/ (outbox + sender) · realtime/ (hub, sse, ws)
│   └── shared/              kernel only: apperr · rolex · authz · clockx · idgen
├── migrations/              Alembic revisions, each with upgrade + downgrade
├── specs/
│   ├── openapi.yaml         API contract, exported from the app
│   └── features/*.feature   pytest-bdd acceptance specs (7 features, 59 executed cases)
├── tests/unit/              in-process service tests with fake stores
├── tests/acceptance/        real uvicorn on an ephemeral port, in-memory fakes
├── docs/                    design docs 00–14 + architecture/*.json diagrams
├── evals/                   offline kit contracts (pytest) + manual live tiers
├── .claude/                 agents · skills · commands · schemas · settings · manifest
├── scripts/                 rename · verify-template · lint-* · leak-guard · export-openapi
│                            + hooks/guard-bash.sh
├── infra/nginx/             edge proxy config + Dockerfile
├── .github/workflows/       ci.yml · deploy.yml · docker-publish.yml · rollback.yml
├── AGENTS.md                root agent instructions (CLAUDE.md is a symlink)
├── MEMORY.md                premise · roles · slice→table map · invariants · gates
├── PLAYBOOK_*.md            six deep contracts
├── Makefile                 ci · test · accept · generate · migrate{,-up,-down,-add}
├── pyproject.toml           Python >=3.12, uv; distribution `app-template` until renamed
├── .env.example             placeholder values only — never real secrets
├── Dockerfile + docker-compose.yml (+ docker-compose.prod.yml)
└── README.md                this file
```

## Prerequisites

- Python **3.12** and **uv** (`uv --version`). `pyproject.toml` pins
  `requires-python`; the Dockerfile base image is `python:3.12-slim`.
- Postgres 17 for local dev (`docker compose up -d` manages it, or use any
  reachable instance). The shipped test suites are fake-wired and need no
  database.
- Docker for `docker compose` and for `scripts/verify-template.sh`.

## The gate

```sh
make ci          # ruff format --check + mypy + ruff/import-linter + gen-check + unit + acceptance + evals
make accept      # pytest-bdd + HTTP contracts over the fake-wired app (no DB)
make vuln        # pip-audit
```

CI (`.github/workflows/ci.yml`) runs `make ci`, then `make vuln`, plus
actionlint. There is no integration lane: nothing in CI executes real SQL.
`verify-template.sh` runs the migration up/down/up smoke in a throwaway
Postgres 17 container, and `evals/run-live.sh` is a manual smoke against a
running stack.

## Auth ships here

Unlike some siblings in this template batch, this template **does** ship a full
authentication/authorization core — it is not a "wire your own auth" skeleton:

- Signup (idempotent) → email verification codes (24h, 5 attempts) → login
  with inactive/blocked/unverified gates → JWT access (15m) + rotating opaque
  refresh (720h) → logout/me (read + partial profile update) →
  forgot/validate/reset/change password (codes 30m, rate-limited 3 per
  window). `forgot` always answers 202 silently, even for an unknown email or
  an over-limit caller; reset does not auto-verify, and reset/change revoke
  every session.
- Roles `guest < user < moderator < admin` (`src/app/shared/rolex.py`, a
  `StrEnum`), `require_auth` / `require_role` dependencies, an admin surface
  writing audit logs. Admin writes pass the shared `authz` guard:
  self-or-strictly-subordinate — an admin cannot touch another admin, and a
  stale token role never outranks the row's DB role.
- Session management: own list (active by default), current, revoke-current,
  revoke-all, admin-side variants (`/api/v1/admin/auth-session/...`) and the
  multi-IP anomalies report (threshold: more than one distinct address).
- Social sign-in: Google ID tokens verified against Google's JWKS + GitHub
  auth-code flow — link by provider id, else email, else provision an
  auto-verified user. Both refuse under `<APP>_OFFLINE=true`.
- A worker janitor purges expired sessions and stale verification/reset codes
  every `<APP>_JANITOR_POLL_SECONDS` (default 3600) — run
  `python -m app.bin.worker` alongside the API.
- Email goes through the outbox (`email_jobs`) with a polling worker, retries
  with backoff (no jitter) governed by the row's `max_retries`, and
  dead-lettering. The enqueue is **atomic** with the business write: the
  adapter stages the job on the caller's session and commits both rows
  together.

Replace nothing to keep auth; to re-key a TTL or gate, edit `.env` values
documented in `.env.example`. To drop a surface, remove the slice's
registration call from `src/app/bin/api.py` — nothing else references it.

## How to apply

1. Copy this directory to your repo root (the skeleton is the repo).
2. `bash scripts/rename.sh <app-slug> <org-scope>` — applies the
   `<scope> → <APP> → <app>` substitutions in safe order, rewrites the
   distribution name `app-template`, and deletes any stale `uv.lock`.
3. `uv sync` — resolves dependencies into `.venv` and writes a local
   `uv.lock`. The template ships none (INV-T3); after rename, decide
   deliberately whether to commit it. Committing it gives reproducible CI and
   a meaningful `pip-audit` baseline; leaving it out keeps the template
   property, and the Docker build resolves at build time.
4. `uv run ruff format .` — run once. Substituted token lengths re-align
   markdown tables, so the first `fmt-check` would otherwise fail on docs.
5. Commit the renamed `pyproject.toml` (and the lockfile if you chose to).
6. `make ci` — green. Then `make accept`.
7. Replace the `widgets` example slice with your domain.

`bash scripts/verify-template.sh` performs the same rename in a temp copy,
then installs, builds the image, runs every gate, migrates a throwaway
Postgres up/down/up, checks the OpenAPI shape, runs the kit evals, and
verifies `run-live.sh` no-ops without `EVALS=1`. The working tree is never
mutated.

## Working with agents

`AGENTS.md` is the root instruction file; `CLAUDE.md` is a symlink to it, so
Claude Code, Codex, Cursor, Gemini and the rest read the same rules.
`MEMORY.md` carries the product premise, actors, slice→table map, stack,
invariants and gates.

The deep contracts are next to this README; read the one you need instead of
importing all of them:

| File                      | Covers                                                                              |
| ------------------------- | ----------------------------------------------------------------------------------- |
| `PLAYBOOK_BACKEND.md`     | slices, dependency edges, ports/adapters, SQL in `queries.py`, outbox, realtime     |
| `PLAYBOOK_URLS.md`        | route naming, namespaces, auth postures, error envelope, streaming                  |
| `PLAYBOOK_TESTS.md`       | test pyramid, pytest/pytest-bdd, contract evals, gates, audit evidence              |
| `PLAYBOOK_SECURITY.md`    | threat model, authn/authz, input, secrets, supply chain, incident                   |
| `PLAYBOOK_AUDIT.md`       | self-hosting a SOC 2 / ISO 27001 audit program                                      |
| `PLAYBOOK_CLAUDE_CODE.md` | the `.claude` kit, SDD + TDD loop, multi-session governance                         |

The kit ships 11 agents (`backend`, `core`, `security`, `testing`), 9 skills
and 9 matching `/commands` (`add-handler`, `add-module`, `ownership-author`,
`depguard-guard`, `review`, `spec`, `plan`, `learn`, `ship`), a `PreToolUse`
Bash guard, and 14 offline contract evals.

```sh
bash scripts/lint-agents.sh                     # frontmatter + body sections
bash scripts/lint-skills.sh                     # strict post-rename
bash scripts/lint-commands.sh                   # slug must match a skill
uv run pytest evals                             # 14 contract tests, offline
bash scripts/verify-template.sh                 # staged renamed copy + full gate
```

Before instantiation, `lint-skills.sh` and `lint-commands.sh` reject the
intentional `<app>` placeholders; set `ALLOW_PLACEHOLDERS=1` to lint the
template itself.

## Container stack

```sh
docker compose up --build      # postgres + migrations + api + worker
docker compose down            # stop (volume persists)
docker compose down -v         # stop + drop the postgres volume
```

The Dockerfile is a single-stage `python:3.12-slim` build: it installs the
project with pip, copies `alembic.ini` + `migrations/`, and runs as the
unprivileged `app` user (uid 10001) on `:8000`, probing its own `/healthz` with
a stdlib `urllib` healthcheck. The same image runs the worker with
`entrypoint: ["python", "-m", "app.bin.worker"]`. A one-shot `migrations`
compose service applies the schema before api/worker boot; the API itself
never migrates at startup (replicas would race). The prod release step is
`docker compose -f docker-compose.prod.yml --profile tools run --rm migrations`.

## Supply chain

- `make vuln` / CI run `pip-audit`.
- The template ships no lockfile (INV-T3). Until one is committed, installs
  resolve latest-for-range and the audit result is a snapshot, not a pin.
- No secret scanner, SBOM, license scan or image scan ships. Add
  gitleaks/trivy/Syft as separate jobs and keep them advisory until the
  baseline is clean (`PLAYBOOK_TESTS.md` §7.4).

## Offline guard (INV-T7)

`<APP>_OFFLINE=true` selects the console mailer unconditionally and disables
outbound sockets: `src/app/platform/email` never reads `<APP>_SMTP_*` (boot
rejects SMTP credentials while offline), the OAuth flows (Google JWKS fetch,
GitHub token/user exchange) refuse with a config-style error at the request
boundary instead of dialing out, and the realtime hub stays in-process. A
fresh `.env` copy with dummy values plus `OFFLINE=true` boots a fully working
stack. To exercise the SMTP path locally, point `<APP>_SMTP_HOST`/`_PORT` at a
local sink with `<APP>_OFFLINE=false`.

## Stack delta from beautifulpython

The published beautifulpython digest (FastAPI, TaskIQ + Dragonfly, SQLAlchemy
2.0 async) is the language guidance; this runnable skeleton makes four
substitution decisions:

| Concern              | beautifulpython digest        | This skeleton                                                     | Why                                                        |
| -------------------- | ----------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------- |
| Query layer          | SQLAlchemy ORM entities       | SQLAlchemy Core `text()` in each `queries.py`                     | SQL stays in the slice's query file, no entity graph       |
| Durable side effects | TaskIQ + Dragonfly (Redis)    | Postgres outbox (`email_jobs`)                                    | one datastore, no Redis dependency for the shipped stack   |
| Dependency injection | FastAPI `Depends` graph       | constructor injection in the composition root (`Depends` only for auth guards) | explicit wiring, testable without the framework |
| Architecture gate    | ruff TID banned imports + import-linter | import-linter contracts only (layers, forbidden, independence) | one authoritative law; `include_external_packages` covers stdlib/third-party |

The dependency law, zero-I/O domain, error envelope and outbox contract are
the same. Swap an adapter, never the contract. Redis pub/sub is the documented
evolution for multi-replica realtime (`PLAYBOOK_BACKEND.md` §8).

## ponytail notes

Deliberate simplifications are marked `# ponytail:` with the ceiling and
upgrade path (e.g. `src/app/platform/realtime/hub.py` drops the oldest event
for a stalled subscriber instead of backpressuring the publisher). Migrations
deliberately do **not** run at server startup — apply them with
`make migrate-up` (Alembic) in your deploy step. Per-slice `queries.py` files
hold the SQL as SQLAlchemy `text()` statements; `postgres.py` maps rows into
domain values and raises `AppError`s.
