# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a single-module Go
backend generated from the `backend-go` template: hexagonal architecture
(ports & adapters) plus vertical slicing, stdlib `net/http` + pgx + sqlc +
goose, full auth core, outbox email worker, WebSocket + SSE.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **Never put a comment on a `.gitignore` pattern line.** Git reads `#` only at
  the start of a line; put the comment on its own line above the pattern.
- **`make ci` green before you declare a change complete.** Plausibility is not
  correctness: read the output.
- **No `nolint` to silence an architecture rule.** Fix the boundary; the
  depguard rules in `.golangci.yml` are law.
- **Never fabricate** paths, API names, env vars, test results or commit hashes.
  Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.

## Layout

```
<app>/
├── cmd/
│   ├── api/main.go        # composition root — the only place that knows all slices
│   └── worker/main.go     # outbox worker (email_jobs) + janitor loops
├── internal/
│   ├── features/          # vertical slices; NEVER import each other
│   │   ├── authsignup/    # domain.go · ports.go · usecase.go · http.go · postgres.go
│   │   │                  # module.go · queries.sql (sqlc source)
│   │   ├── authsession/
│   │   ├── authpassword/
│   │   ├── authoauth/
│   │   ├── adminusers/
│   │   ├── widgets/       # example slice — replace with your domain
│   │   └── realtime/
│   ├── platform/          # transversal infra: authx, oauthx, email, realtime, config, httpx, pgx
│   ├── shared/            # kernel ONLY: apperr, rolex, authz, clockx, idgen
│   └── gen/               # committed generated code (db: sqlc, api: oapi models)
├── migrations/            # goose migrations (+goose Up / Down sections)
├── specs/                 # openapi.yaml + api.gen.yaml + features/*.feature
├── sqlc.yaml              # schema = migrations; queries live per-slice
├── test/acceptance/       # godog over the real mux wired with fakes
├── evals/                 # offline contract tests + live-tier smoke (run-live.sh)
├── docs/                  # 15 architecture docs + docs/plans/ from /plan
├── .claude/               # agents · skills · commands · schemas · settings · manifest
└── scripts/               # rename · verify-template · lint-* · leak-guard · install-runtime
                           # + hooks/guard-bash.sh
```

## Commands

```sh
make ci                  # gofmt check + go vet + golangci-lint + gen-check + go test
go test ./internal/features/<slice>/...    # single slice, fast loop
make test-int            # -tags=integration: testcontainers Postgres
make accept              # godog over the fake-wired mux (no DB, fast)
make generate            # sqlc + oapi-codegen; must leave internal/gen/** unchanged
make migrate-add NAME=…  # new goose migration
make run                 # API on :8080
```

`go mod tidy` must stay quiet (INV-T3 hygiene) — dependency changes are
reviewed, not incidental. `go.sum` is generated post-rename and committed.

## Slice playbook (add a feature)

1. Create `internal/features/<slug>/` with the standard six files:
   `domain.go` (pure types + errors, stdlib + shared only), `ports.go`
   (interfaces the consumer declares), `usecase.go`, `http.go` (ServeMux
   adapters + encoding), `postgres.go` (adapter over `gen/db`),
   `module.go` (`Register(mux *http.ServeMux, deps Deps)`).
2. Write sqlc queries in the slice's `queries.sql` (schema source is
   `migrations/`), add a migration (`make migrate-add NAME=...`), run
   `make generate`.
3. Wire in `cmd/api/main.go` only — construct the pgx adapters, inject them
   into the use case, call `Register`.
4. Tests: table-driven unit tests with in-package fakes against `ports.go`;
   godog steps if a behavioral contract exists in `specs/features/`.

The `PreToolUse` guard hook (`.claude/settings.json` +
`scripts/hooks/guard-bash.sh`) intercepts every `Bash` tool call and flags
high-risk shell patterns; it is not something you invoke.

## Workflow

Spec → plan → red → green → refactor:

1. `/spec` files a scoped, redaction-scanned issue. `/plan` writes
   `docs/plans/<slug>.md` (problem, constraints, design, files, risks, tests).
2. `/add-module` or `/add-handler` scaffolds the slice/route.
3. Write the failing test first (`/tdd-guide`, `test-author`), then the
   minimum implementation, then refactor.
4. `/review` for the pre-landing checklist; `make ci` before declaring done.

## Who edits what

| Area | Owner |
|---|---|
| `docs/plans/`, `specs/` | `planner`, `architect` |
| `*_test.go` | `test-author` |
| `internal/features/**`, `cmd/**` | implementers (`build-resolver`, `tdd-guide`) |
| `internal/shared/**`, `internal/platform/**`, `internal/gen/**`, `go.mod`, `specs/openapi.yaml`, `.github/**` | shared — integrator role |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**` | human only; agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (slice, spec); name sessions `<slice>:<spec-id>`.
- Do not edit a slice claimed by another session — message its owner instead.
- Changing a published contract or shared artifact? Message dependents and tell
  the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`backend`,
`security`, `testing`, `core`). `.claude/skills/` + `.claude/commands/` ship
reusable workflows; every command is a thin shim delegating 1:1 to the skill of
the same slug. New slugs are discovered by walking the tree — no registry edit.
`scripts/lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh` validate the
tree against `.claude/schemas/`.

Skills: **`/add-handler`**, **`/add-module`** (scaffolding), **`/ownership-author`**
(ports + adapter wiring), **`/depguard-guard`** (architecture law + INV-T7),
**`/review`**, **`/spec`**, **`/plan`**, **`/learn`**, **`/ship`** (release PR;
user-invoked only).

## Playbooks

Full contracts live next to this file; read the one you need, do not import
them wholesale:

| File | Covers |
|---|---|
| `PLAYBOOK_BACKEND.md` | Architecture, slices, dependency rules, persistence, worker, realtime |
| `PLAYBOOK_TESTS.md` | Test pyramid, tags, environments, gates, SOC 2 / ISO 27001 evidence |
| `PLAYBOOK_SECURITY.md` | Threat model, authn/authz, input, secrets, supply chain, incident |
| `PLAYBOOK_URLS.md` | Route naming, namespaces, error envelope, streaming |
| `PLAYBOOK_AUDIT.md` | Self-hosting an audit program (SOC 2 / ISO 27001 / 9001) |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code kit, SDD + TDD loop, multi-session governance |

## Learnings

`.claude/learnings.jsonl` is an append-only, project-local log of pitfalls,
patterns and preferences discovered while working on `<app>` (created on the
first `/learn`). Grep it by `kind` before repeating work that resembles a
logged pitfall.

## Conventions

- `context.Context` first parameter on every I/O boundary; use cases take and
  return it.
- Errors: wrap with `%w`, match with `errors.Is`/`errors.As`; stable codes live
  in `internal/shared/apperr`, mapped to `{detail, code}` only in `http.go` /
  `httpx`.
- Interfaces live in the consumer package, kept small (1–2 methods).
- No goroutine starts without an owner: contexts derive from the server's base
  context; workers shut down via `ctx.Done()`.
- This project was instantiated from a template: the three substitution tokens
  documented in `scripts/rename.sh` were replaced at instantiation time. If one
  survives verbatim outside test fixtures, it is a bug, not a template artifact.
