# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a single-package
Node 24 + TypeScript backend generated from the `backend-node` template:
hexagonal architecture (ports & adapters) plus vertical slicing, Fastify 5 +
pg + Kysely typed queries + node-pg-migrate, full auth core, outbox email
worker, WebSocket + SSE.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **Never put a comment on a `.gitignore` pattern line.** Git reads `#` only
  at the start of a line; put the comment on its own line above the pattern.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output.
- **Architecture law is the eslint `no-restricted-imports` groups.** Never
  weaken them and never silence them with `eslint-disable` or
  `@ts-expect-error`. If a rule fires, re-cut the layout.
- **No unexplained suppressions.** `@ts-ignore` and `@ts-nocheck` are
  forbidden; `@ts-expect-error` needs a written reason on the narrowest line.
- **Never fabricate** paths, exports, env vars, test results or commit hashes.
  Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **The template ships no `pnpm-lock.yaml` (INV-T3).** Do not stage one from
  the template context; after instantiation, decide deliberately whether to
  commit it.
- **`<APP>_OFFLINE=true` is a hard offline guard (INV-T7).** While set, the
  console mailer is selected, SMTP credentials are rejected at boot, and OAuth
  flows refuse instead of opening sockets.

## Layout

```
<app>/
├── src/
│   ├── bin/
│   │   ├── api.ts          # composition root — the only place that knows all slices
│   │   ├── middleware.ts   # request-id · security headers · CORS
│   │   └── worker.ts       # outbox worker (email_jobs) + janitor loop
│   ├── features/           # vertical slices; NEVER import each other
│   │   ├── authsignup/     # domain.ts · ports.ts · usecase.ts · http.ts
│   │   │                   # queries.ts · postgres.ts · module.ts
│   │   ├── authsession/
│   │   ├── authpassword/
│   │   ├── authoauth/
│   │   ├── adminusers/
│   │   ├── widgets/        # example slice — replace with your domain
│   │   └── realtime/
│   ├── platform/           # authx · oauthx · email · realtime · config · httpx · pg
│   ├── shared/             # kernel ONLY: apperr · rolex · authz · clockx · idgen
│   └── gen/                # committed generated code (openapi-typescript models)
├── migrations/             # node-pg-migrate files (each carries up + down)
├── specs/                  # openapi.yaml + features/*.feature
├── test/acceptance/        # cucumber over the fake-wired app
├── evals/                  # offline kit contract tests (node --test)
├── .claude/                # agents · skills · commands · schemas · settings · manifest
└── scripts/                # rename · verify-template · lint-* · leak-guard · install-runtime
                            # + hooks/guard-bash.sh
```

## Commands

```sh
make ci                  # prettier check + tsc --noEmit + eslint + gen-check + vitest
make vet                 # tsc --noEmit
make lint                # eslint .
make test                # vitest (unit)
make accept              # cucumber over the fake-wired app
make generate            # regenerate src/gen/api from specs/openapi.yaml
make gen-check           # fail if committed generated code is stale
make migrate-up          # apply migrations (needs <APP>_DATABASE_URL)
make migrate-add NAME=add_orders_status
make vuln                # pnpm audit --audit-level=high
docker compose up --build            # postgres + migrations + api + worker
bash scripts/verify-template.sh      # staged renamed copy + factory smoke
```

## Dependency rules (enforced by eslint)

| From                                            | May depend on                                   |
| ----------------------------------------------- | ----------------------------------------------- |
| `src/bin/*`                                     | anything (the roots know every slice)           |
| feature `http.ts` / `postgres.ts` / `module.ts` | own slice, `#platform/*`, `#shared/*`           |
| `usecase.ts`                                    | `ports.ts`, `domain.ts`                         |
| `ports.ts`                                      | `domain.ts`                                     |
| `domain.ts`                                     | Node stdlib, `#shared/*` — **zero I/O**         |
| `src/platform/*`                                | `#shared/*`, stdlib, `pg` — never `#features/*` |
| anything outside a feature                      | that feature's `module.ts` internals            |
| anything                                        | `src/bin/**` (nothing depends on the root)      |

Slices never import each other. Shared behavior moves down into `#shared` /
`src/platform`, or sideways through a port the consumer declares. Own-slice
files import relatively (`./x.js`); cross-layer imports use the package
`"imports"` map (`#shared/*`, `#platform/*`, `#features/*`, `#gen/*`).

Every port method returns a domain value or throws an `AppError`; adapters
map driver errors inward. Consumers hold the port interface, never the
`Postgres` adapter type. SQL lives only in a slice's `queries.ts`.

## Slice playbook (add a feature)

1. Create `src/features/<slug>/` with the standard seven files:
   `domain.ts` (pure types + errors), `ports.ts` (interfaces the use case
   needs), `usecase.ts`, `http.ts`, `queries.ts`, `postgres.ts`, `module.ts`.
2. Add a paired migration (`make migrate-add NAME=...`); fill both `up` and
   `down`. Audit the slice's SQL against the migration columns.
3. Write the use case against the ports; add `usecase.test.ts` with in-package
   fakes.
4. Wire in `src/bin/api.ts` only — construct the adapter, inject it, call the
   slice's `register`.
5. Add `specs/features/<slug>.feature` if a behavioral contract exists; run
   `make ci` and `make accept`.

The `PreToolUse` guard hook (`.claude/settings.json` +
`scripts/hooks/guard-bash.sh`) intercepts every `Bash` tool call and flags
high-risk shell patterns; it is not something you invoke.

## Language rules (beautifulnodejs)

The deep contract is `PLAYBOOK_BACKEND.md`; these are the non-negotiables:

- TypeScript strict is the floor. No `any` in shipped types; use `unknown`
  and narrow once at the boundary. Explicit return types on every export.
- Make invalid states unrepresentable: discriminated unions over boolean
  flags, branded ids, `readonly` for data a callee must not mutate.
- No boolean parameters. Use a two-variant union, a named options object, or
  two functions.
- The event loop is not blocked: no sync fs/crypto/zlib on a request path; no
  unbounded `Promise.all` over user-sized input; every external call gets a
  timeout or `AbortSignal`.
- No floating promises. Await, `void` with a reason, or `.catch`.
- Errors are typed values with stable codes. Match on `code`, never on
  message text. Never swallow a `catch`.
- Structured logging only through the Fastify/Pino logger on request paths;
  `console.*` is allowed only for the worker entrypoint's own loops.
- Tests are table-driven where behavior repeats, fake the ports, inject the
  clock, and never touch real Postgres/Redis/network in the unit tier.

## Workflow

Spec → plan → red → green → refactor:

1. `/spec` files a scoped, redaction-scanned issue. `/plan` writes
   `docs/plans/<slug>.md` (problem, constraints, design, files, risks, tests).
2. `/add-module` or `/add-handler` scaffolds the slice/route.
3. Write the failing test first (`/tdd-guide`, `test-author`), then the
   minimum implementation, then refactor.
4. `/review` for the pre-landing checklist; `make ci` before declaring done.

## Who edits what

| Area                                                                                  | Owner                                                             |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `docs/plans/`, `specs/openapi.yaml`                                                   | `planner`, `architect`, `api-designer`                            |
| `*.test.ts`, `test/acceptance/**`, `specs/features/**`                                | `test-author`                                                     |
| `src/features/<slice>/**`                                                             | the slice's implementer                                           |
| `src/shared/**`, `src/platform/**`, `eslint.config.mjs`, `package.json`, `.github/**` | shared — integrator role                                          |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**`  | human only; an agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (slice, spec); name sessions `<slice>:<spec-id>`.
- Do not edit a slice claimed by another session — message its owner instead.
- Changing a published contract (route, error code, port shape, migration) or
  a shared module? Message dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`backend`,
`security`, `testing`, `core`). `.claude/skills/` + `.claude/commands/` ship
reusable workflows; every command is a thin shim delegating 1:1 to the skill
of the same slug. New slugs are discovered by walking the tree — no registry
edit. `scripts/lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh` validate
the tree against `.claude/schemas/`; `evals/contracts.test.mjs` is the
zero-dependency contract suite.

Skills: **`/add-handler`**, **`/add-module`** (scaffolding), **`/ownership-author`**
(ports, adapters, async interleaving), **`/depguard-guard`** (import law and
INV-T7), **`/review`**, **`/spec`**, **`/plan`**, **`/learn`**, **`/ship`**
(release PR; user-invoked only).

## Playbooks

Full contracts live next to this file; read the one you need, do not import
them wholesale:

| File                      | Covers                                                               |
| ------------------------- | -------------------------------------------------------------------- |
| `PLAYBOOK_BACKEND.md`     | Slices, dependency edges, ports/adapters, SQL, outbox, realtime      |
| `PLAYBOOK_TESTS.md`       | Test pyramid, vitest/cucumber, contract evals, gates, audit evidence |
| `PLAYBOOK_SECURITY.md`    | Threat model, authn/authz, input, secrets, supply chain, incident    |
| `PLAYBOOK_URLS.md`        | Route naming, auth postures, error envelope, streaming               |
| `PLAYBOOK_AUDIT.md`       | Self-hosting an audit program (SOC 2 / ISO 27001 / ISO 9001)         |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code kit, SDD + TDD loop, multi-session governance        |

## Learnings

`.claude/learnings.jsonl` is an append-only, project-local log of pitfalls,
patterns and preferences discovered while working on `<app>` (created on the
first `/learn`). Grep it by `kind` before repeating work that resembles a
logged pitfall.

## Conventions

- `AbortSignal` threads the request/deploy signal through every I/O boundary;
  use cases take and honor it (`signal.throwIfAborted()` at loop yields).
- Errors: wrap with the message + `cause`, branch on `code`; stable codes live
  in `src/shared/apperr`, rendered as `{detail, code}` only in `http.ts` /
  `httpx`.
- Interfaces live in the consumer slice's `ports.ts`, kept small (1–3
  methods).
- No `Promise` is started without an owner: workers shut down via the process
  `AbortSignal`; `Promise.all` only for genuinely independent work.
- Migrations never run at startup; `make migrate-up`, or the compose one-shot
  `migrations` service, is the deploy/boot step you control.
- This project was instantiated from a template: the substitution tokens
  documented in `scripts/rename.sh` were replaced at instantiation time. If
  one survives verbatim outside `evals/fixtures/` or the linter/leak-guard
  scripts, it is a bug, not a template artifact.
- Stack delta from beautifulnodejs's published digest: this skeleton uses
  Kysely (not Drizzle), a DB outbox (not BullMQ/Redis), constructor injection
  (not tsyringe), and Fastify's Pino logger. The dependency law, zero-I/O
  domain, error envelope and outbox contract are the same. Swap an adapter,
  never the contract.
