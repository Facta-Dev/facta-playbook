# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a crate-per-concern
Rust workspace generated from the `backend-rust` template: hexagonal
architecture (ports & adapters) plus vertical slicing, axum 0.8 + sqlx 0.8 +
Postgres.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **Never put a comment on a `.gitignore` pattern line.** Git reads `#` only at
  the start of a line; put the comment on its own line above the pattern.
- **`just ci` green before you declare a change complete.** Plausibility is not
  correctness: read the output.
- **`unsafe` is forbidden** (`unsafe_code = "forbid"`). Do not relax it; a real
  FFI need is a design discussion, not a lint exemption.
- **No unexplained suppressions.** Prefer `#[expect(lint, reason = "…")]`; a new
  bare `#[allow]` is a defect. Never suppress a clippy lint just to make the
  gate pass.
- **Never fabricate** paths, crate names, env vars, test results or commit
  hashes. Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.

## Layout

```
<app>/
├── crates/
│   ├── <app>-core/        # pure domain (zero I/O) — identity types included
│   ├── <app>-ports/       # traits + Pg adapters + mock/ (the only SQL home)
│   ├── <app>-widgets/     # example vertical slice
│   ├── <app>-auth/        # identity services (signup/sessions/password/oauth)
│   ├── <app>-adminusers/  # admin roster service
│   ├── <app>-warden/      # auth infra (JWT codec, password, OAuth verifiers)
│   ├── <app>-mailer/      # email infra adapter (SMTP/console/offline)
│   ├── <app>-loom/        # realtime hub + outbox/janitor workers + worker bin
│   ├── <app>-api/         # composition root (AppState + router + main; subcommands)
│   ├── <app>-sdk/         # typed client
│   └── <app>-cli/         # CLI
├── migrations/            # paired .up.sql + .down.sql, embedded and applied by `migrate`
├── docs/                  # architecture docs 00–14 + docs/architecture/*.json diagrams
├── evals/                 # offline contract tests for this kit + live-tier smoke
├── .claude/               # agents · skills · commands · schemas · settings · manifest
└── scripts/               # rename · verify-template · lint-* · leak-guard · install-runtime
                           # + hooks/guard-bash.sh
```

## Commands

```sh
just ci                  # fmt-check + clippy -D warnings + check + nextest + contract evals
cargo nextest run -p <app>-<slice>     # single crate, fast loop
cargo clippy --workspace --all-targets --all-features -- -D warnings
just migrate             # one-shot schema apply (needs <APP>_DATABASE_URL)
just migrate-add create_widgets        # paired up/down migration
just gen-openapi         # emit openapi.json from the code
just gen-check           # fail if the committed openapi.json is stale
just audit               # cargo-audit + cargo-deny + cargo-machete
docker compose up --build              # postgres + migrations + api + worker
```

`Cargo.lock` is committed for this binary workspace; dependency changes are
reviewed, not incidental. `cargo machete` flags unused deps in `just audit`.

## Dependency rules (enforced by the manifest graph)

| From | May depend on |
|---|---|
| `<app>-api` | any crate (root knows everything) |
| `<app>-loom` | slices, ports, core (orchestrator exception) |
| `<app>-<slice>` | ports, core, infra adapters — **never** a sibling slice |
| `<app>-ports` | core |
| `<app>-mailer`, `<app>-sdk`, `<app>-cli` | core (+ sdk) |
| `<app>-core` | nothing in the workspace |

Every port method returns `core::Result<T>`; adapters map foreign errors
inward. Consumers hold `Arc<dyn Trait>`, never a `Pg*` adapter type. SQL lives
only in `crates/<app>-ports/src/adapters/`.

## Slice playbook (add a feature)

1. `crates/<app>-<slug>/` with `[lints] workspace = true`; add it to
   `[workspace] members` and `[workspace.dependencies]`.
2. If it needs new storage: a trait in `ports/domain/`, a `Pg*` adapter in
   `ports/adapters/`, a mock in `ports/mock/`, re-exported from `ports/lib.rs`.
3. Write `<Scope>Service` holding `Arc<dyn …Repository>`; unit-test with the
   mock from `ports`.
4. Write a paired migration (`just migrate-add create_widgets`); refresh the
   committed `openapi.json` if the HTTP surface changed.
5. Wire in `AppState::from_env` and mount routes in `router::build` — the only
   place that knows every slice.
6. Run `just ci`.

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
4. `/review` for the pre-landing checklist; `just ci` before declaring done.

## Who edits what

| Area | Owner |
|---|---|
| `docs/plans/`, `openapi.json` | `planner`, `architect` |
| `#[cfg(test)]` modules and `tests/` | `test-author` |
| `crates/<app>-<slice>/**` | the slice's implementer |
| `crates/<app>-core/**`, `crates/<app>-ports/**`, `Cargo.toml` (workspace), `openapi.json`, `.github/**` | shared — integrator role |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**` | human only; agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (slice, spec); name sessions `<slice>:<spec-id>`.
- Do not edit a slice claimed by another session — message its owner instead.
- Changing a published contract, a port trait, or a shared crate? Message
  dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`backend`,
`security`, `testing`, `core`). `.claude/skills/` + `.claude/commands/` ship
reusable workflows; every command is a thin shim delegating 1:1 to the skill of
the same slug. New slugs are discovered by walking the tree — no registry edit.
`scripts/lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh` validate the
tree against `.claude/schemas/`.

Skills: **`/add-handler`**, **`/add-module`** (scaffolding), **`/ownership-author`**
(borrow/lifetime + `Arc<dyn Trait>` wiring), **`/unsafe-guard`** (soundness and
the no-unsafe invariant), **`/review`**, **`/spec`**, **`/plan`**, **`/learn`**,
**`/ship`** (release PR; user-invoked only).

## Playbooks

Full contracts live next to this file; read the one you need, do not import
them wholesale:

| File | Covers |
|---|---|
| `PLAYBOOK_BACKEND.md` | Crates, dependency rules, ports/adapters, SQL, workers, realtime |
| `PLAYBOOK_TESTS.md` | Test pyramid, nextest, contract evals, gates, SOC 2 / ISO 27001 evidence |
| `PLAYBOOK_SECURITY.md` | Threat model, authn/authz, input, secrets, supply chain, incident |
| `PLAYBOOK_URLS.md` | Route naming, namespaces, error envelope, streaming |
| `PLAYBOOK_AUDIT.md` | Self-hosting an audit program (SOC 2 / ISO 27001 / ISO 9001) |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code kit, SDD + TDD loop, multi-session governance |

## Learnings

`.claude/learnings.jsonl` is an append-only, project-local log of pitfalls,
patterns and preferences discovered while working on `<app>` (created on the
first `/learn`). Grep it by `kind` before repeating work that resembles a
logged pitfall.

## Conventions

- Immutable data patterns: return new values, never mutate in place; clone at
  the seam, not in the loop.
- Small, cohesive files over large ones; extract a module before it grows
  unwieldy.
- Errors: `thiserror` in libraries, `anyhow` at the binary edge; never leak
  `sqlx::Error` past an adapter.
- No glob imports (`wildcard_imports`); `super::*` is allowed in tests.
- Libraries log with `tracing`, never `println!`.
- This project was instantiated from a template: the angle-bracket tokens
  documented in `scripts/rename.sh` were replaced at instantiation time. If one
  survives verbatim outside `evals/` fixtures or the linter/leak-guard scripts,
  it is a bug, not a template artifact.
