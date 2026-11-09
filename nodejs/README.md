# `<app>` — Node/TypeScript backend skeleton

A hexagonal, vertically-sliced Node 24 + TypeScript 5.9 backend template:
Fastify 5, `pg` + Kysely typed queries, node-pg-migrate, zod 4, full auth
core, outbox email worker, WebSocket + SSE, Cucumber acceptance over a
fake-wired app, and an eslint `no-restricted-imports` dependency law. Every
`<app>`, `<APP>` and `<scope>` token is a **literal placeholder** — the
imports are invalid until you rename, so the project cannot typecheck as
shipped. The template also ships no `pnpm-lock.yaml` (INV-T3). See
[How to apply](#how-to-apply).

This directory is both the runnable skeleton and the documentation set: the
six `PLAYBOOK_*.md` contracts, the numbered `docs/`, and the `.claude/` kit
govern the code in `src/`.

## Layout

```
<app>/
├── src/
│   ├── bin/
│   │   ├── api.ts           composition root: env → pool → adapters → register → listen
│   │   ├── middleware.ts    request-id · security headers · CORS
│   │   └── worker.ts        outbox (email_jobs) + janitor loops — same image, different entry
│   ├── features/            seven vertical slices (never import each other)
│   │   ├── authsignup/      signup · verify · resend (email_verifications)
│   │   ├── authsession/     login · refresh · logout · me · sessions + anomalies
│   │   ├── authpassword/    forgot · validate · reset · change (password_resets)
│   │   ├── authoauth/       Google One Tap + GitHub auth-code social sign-in
│   │   ├── adminusers/      admin user CRUD + audit_logs (ADMIN only)
│   │   ├── widgets/         example slice: CRUD + SSE lifecycle events
│   │   └── realtime/        WebSocket /api/v1/ws?token= (hub from platform)
│   ├── platform/            transversal infra: authx · config · httpx · oauthx · pg
│   │                        email/ (outbox + Sender) · realtime/ (hub)
│   ├── shared/              kernel only: apperr · rolex · authz · clockx · idgen
│   └── gen/                 committed generated code (openapi-typescript models)
├── migrations/              node-pg-migrate files, each with up + down
├── specs/
│   ├── openapi.yaml         API contract → src/gen/api/schema.d.ts
│   └── features/*.feature   Cucumber acceptance specs (7 features, 60 executed scenarios)
├── test/acceptance/         real Fastify app wired with in-memory fakes
├── docs/                    design docs 00–14 + architecture/*.json diagrams
├── evals/                   offline kit contracts (node --test) + manual live tiers
├── .claude/                 agents · skills · commands · schemas · settings · manifest
├── scripts/                 rename · verify-template · lint-* · leak-guard · install-runtime
│                            + hooks/guard-bash.sh
├── infra/nginx/             edge proxy config + Dockerfile
├── .github/workflows/       ci.yml · deploy.yml · docker-publish.yml · rollback.yml
├── AGENTS.md                root agent instructions (CLAUDE.md is a symlink)
├── MEMORY.md                premise · roles · slice→table map · invariants · gates
├── PLAYBOOK_*.md            six deep contracts
├── Makefile                 ci · test · accept · generate · migrate{,-up,-down,-add}
├── package.json             Node >=24, pnpm 10 pinned via corepack
├── .env.example             placeholder values only — never real secrets
├── Dockerfile + docker-compose.yml (+ docker-compose.prod.yml)
└── README.md                this file
```

## Prerequisites

- Node **24** (`node -v`). Raise the floor by updating `engines`, the CI
  matrix and the Dockerfile base image together.
- **pnpm** via corepack (`corepack enable`) — `packageManager` pins the exact
  version (`pnpm@10.34.7`); no global install needed.
- Postgres 17 for local dev (`docker compose up -d` manages it, or use any
  reachable instance). The shipped test suites are fake-wired and need no
  database.
- Docker for `docker compose` and for `scripts/verify-template.sh`.

## The gate

```sh
make ci          # prettier check + tsc (app + test configs) + eslint + gen-check + vitest
make accept      # Cucumber over the fake-wired app (no DB, fast)
make vuln        # pnpm audit --audit-level=high
```

CI (`.github/workflows/ci.yml`) runs `make ci`, then `make test` +
`make accept`, then `pnpm audit --audit-level=high`, plus actionlint. There is
no integration lane: nothing in CI executes real SQL. `verify-template.sh`
runs the migration up/down/up smoke in a throwaway Postgres 17 container, and
`evals/run-live.sh` is a manual smoke against a running stack.

## Auth ships here

Unlike some siblings in this template batch, this template **does** ship a full
authentication/authorization core — it is not a "wire your own auth" skeleton:

- Signup (idempotent) → email verification codes → login with inactive/blocked/
  unverified gates → JWT access (15m) + rotating opaque refresh (720h) →
  logout/me (read + partial profile update) → forgot/validate/reset/change
  password (codes 30m, rate-limited 3/5min, reset auto-verifies and revokes
  sessions).
- Roles `ADMIN > MODERATOR > USER > GUEST` (`src/shared/rolex.ts`),
  `requireAuth` / `requireRole` preHandlers, admin surface writing audit logs.
  Admin writes pass the shared `authz` guard: self-or-strictly-subordinate —
  an ADMIN cannot touch another ADMIN, and a stale token role never outranks
  the row's DB role.
- Session management: own list (active by default), current, revoke-current,
  revoke-all, admin-side variants (`/api/v1/admin/auth-session/...`) and the
  multi-IP anomalies report.
- Social sign-in: Google One Tap (RS256 ID token verified against Google's
  JWKS) + GitHub auth-code flow — link by provider id, else email, else
  provision an auto-verified user. Both refuse under `<APP>_OFFLINE=true`.
- A worker janitor purges expired sessions and stale verification/reset codes
  every `<APP>_JANITOR_POLL` (default 1h) — run `node dist/bin/worker.js`
  alongside the API.
- Email goes through the outbox (`email_jobs`) with a polling worker, retries
  with jittered backoff, and dead-lettering. The enqueue is **not atomic**
  with the state change (own pool connection); see `MEMORY.md` invariant 7.

Replace nothing to keep auth; to re-key a TTL or gate, edit `.env` values
documented in `.env.example`. To drop a surface, remove the slice's
registration call from `src/bin/api.ts` — nothing else references it.

## How to apply

1. Copy this directory to your repo root (the skeleton is the repo).
2. `bash scripts/rename.sh <app-slug> <org-scope>` — applies the
   `<scope> → <APP> → <app>` substitutions in safe order.
3. `pnpm install` — resolves dependencies. A local `pnpm-lock.yaml` is
   generated. The template ships none (INV-T3); after rename, decide
   deliberately whether to commit it. Committing it gives reproducible CI and
   a meaningful `pnpm audit` baseline; leaving it out keeps the template
   property, and the Docker build self-heals with a plain `pnpm install`.
4. `pnpm format` — run once. Substituted token lengths re-align markdown
   tables, so the first `fmt-check` would otherwise fail on docs.
5. Commit the renamed `package.json` (and the lockfile if you chose to).
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

| File                      | Covers                                                                          |
| ------------------------- | ------------------------------------------------------------------------------- |
| `PLAYBOOK_BACKEND.md`     | slices, dependency edges, ports/adapters, Kysely SQL, outbox, realtime, workers |
| `PLAYBOOK_URLS.md`        | route naming, namespaces, auth postures, error envelope, streaming              |
| `PLAYBOOK_TESTS.md`       | test pyramid, vitest/Cucumber, contract evals, gates, audit evidence            |
| `PLAYBOOK_SECURITY.md`    | threat model, authn/authz, input, secrets, supply chain, incident               |
| `PLAYBOOK_AUDIT.md`       | self-hosting a SOC 2 / ISO 27001 audit program                                  |
| `PLAYBOOK_CLAUDE_CODE.md` | the `.claude` kit, SDD + TDD loop, multi-session governance                     |

The kit ships 11 agents (`backend`, `core`, `security`, `testing`), 9 skills
and 9 matching `/commands` (`add-handler`, `add-module`, `ownership-author`,
`depguard-guard`, `review`, `spec`, `plan`, `learn`, `ship`), a `PreToolUse`
Bash guard, and 14 offline contract evals.

```sh
bash scripts/lint-agents.sh                     # frontmatter + body sections
bash scripts/lint-skills.sh                     # strict post-rename
bash scripts/lint-commands.sh                   # slug must match a skill
node --test evals/contracts.test.mjs            # 14 contract tests, offline
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

The Dockerfile is a two-stage `node:24-alpine` build. The runtime image ships
`dist/` and `node_modules/`, re-points the `package.json` `"imports"` map at
`dist/`, runs as the unprivileged `node` user on `:8080`, and probes its own
`/healthz` on a container healthcheck. The same image runs the worker with
`entrypoint: ["node", "dist/bin/worker.js"]`. A one-shot `migrations` compose
service applies the schema before api/worker boot; the API itself never
migrates at startup (replicas would race). The prod release step is
`docker compose -f docker-compose.prod.yml --profile tools run --rm migrations`.

## Supply chain

- `make vuln` / CI run `pnpm audit --audit-level=high`.
- The template ships no lockfile (INV-T3). Until one is committed, installs
  resolve latest-for-range and the audit result is a snapshot, not a pin.
- No secret scanner, SBOM, license scan or image scan ships. Add
  gitleaks/trivy/Syft as separate jobs and keep them advisory until the
  baseline is clean (`PLAYBOOK_TESTS.md` §7.4).

## Offline guard (INV-T7)

`<APP>_OFFLINE=true` selects the console mailer unconditionally and disables
outbound sockets: `src/platform/email` never reads `<APP>_SMTP_*`, the OAuth
flows (Google JWKS fetch, GitHub token/user exchange) refuse with a
config-style error at the request boundary instead of dialing out, and the
realtime hub stays in-process. A fresh `.env` copy with dummy values plus
`OFFLINE=true` boots a fully working stack. To exercise the SMTP path locally,
point `<APP>_SMTP_HOST`/`_PORT` at a local sink with `<APP>_OFFLINE=false`.

## Stack delta from beautifulnodejs

The published beautifulnodejs digest (Drizzle, BullMQ/Redis, tsyringe) is the
language guidance; this runnable skeleton makes three substitution decisions:

| Concern              | beautifulnodejs digest | This skeleton                                 | Why                                                           |
| -------------------- | ---------------------- | --------------------------------------------- | ------------------------------------------------------------- |
| Query layer          | Drizzle ORM            | Kysely 0.29                                   | typed query builder, no schema DSL, SQL stays in `queries.ts` |
| Durable side effects | BullMQ + Redis         | Postgres outbox (`email_jobs`)                | one datastore, no Redis dependency for the shipped stack      |
| Dependency injection | tsyringe               | constructor injection in the composition root | no `reflect-metadata`, no decorators, explicit wiring         |

The dependency law, zero-I/O domain, error envelope and outbox contract are
the same. Swap an adapter, never the contract. Redis pub/sub is the documented
evolution for multi-replica realtime (`PLAYBOOK_BACKEND.md` §8).

## ponytail notes

Deliberate stubs are marked `// ponytail:` with the ceiling and upgrade path
(e.g. `src/platform/oauthx.ts` leaves the JWKS cache/cooldown state machine to
`jose`; `src/features/authsignup/usecase.ts` names the non-transactional
outbox write; `test/acceptance/fakes.ts` bounds its offset window to the
scenario size). Migrations deliberately do **not** run at server startup —
apply them with `make migrate-up` (node-pg-migrate) in your deploy step.
Per-slice `queries.ts` files type the schema and export named queries as
type-checked calls through Kysely; `postgres.ts` binds the pg Pool into the
slice's Kysely instance.
