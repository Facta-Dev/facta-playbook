# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a single-package
Python 3.12 backend generated from the `backend-python` template: hexagonal
architecture (ports & adapters) plus vertical slicing, FastAPI + pydantic v2,
SQLAlchemy 2 async + asyncpg, Alembic, argon2id, full auth core, DB outbox
email worker, WebSocket + SSE. Dependencies and the venv are managed by `uv`.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **Never put a comment on a `.gitignore` pattern line.** Git reads `#` only
  at the start of a line; put the comment on its own line above the pattern.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output.
- **Architecture law is the import-linter contracts in `pyproject.toml`.**
  Never weaken them and never silence them. If a contract fires, re-cut the
  layout.
- **No unexplained suppressions.** `# type: ignore` needs a written reason on
  the narrowest line; `# noqa` needs the rule code and a reason, and is only
  acceptable at a tested boundary.
- **Never fabricate** paths, symbols, env vars, test results or commit hashes.
  Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **The template ships no `uv.lock` (INV-T3 analog).** Do not stage one from
  the template context; after instantiation, decide deliberately whether to
  commit it.
- **`<APP>_OFFLINE=true` is a hard offline guard (INV-T7).** While set, the
  console mailer is selected, SMTP credentials are rejected at boot, and OAuth
  flows refuse instead of opening sockets.

## Layout

```
<app>/
├── src/app/
│   ├── bin/
│   │   ├── api.py           # composition root — the only place that wires all slices
│   │   ├── middleware.py    # request-id · security headers · CORS
│   │   └── worker.py        # outbox worker (email_jobs) + janitor loop
│   ├── features/            # vertical slices; NEVER import each other
│   │   ├── authsignup/      # domain.py · ports.py · usecase.py · queries.py
│   │   │                    # postgres.py · http.py · module.py
│   │   ├── authsession/
│   │   ├── authpassword/
│   │   ├── authoauth/
│   │   ├── adminusers/
│   │   ├── widgets/         # example slice — replace with your domain
│   │   └── realtime/        # module.py only; the hub lives in platform/
│   ├── platform/            # authx · oauthx · email · realtime · config · db · envelope · logging
│   └── shared/              # kernel ONLY: apperr · rolex · authz · clockx · idgen
├── migrations/              # Alembic versions (each carries upgrade + downgrade)
├── specs/                   # openapi.yaml (exported) + features/*.feature
├── tests/unit/              # in-process service tests with fake stores
├── tests/acceptance/        # real uvicorn on an ephemeral port, fake adapters
├── evals/                   # offline kit contract suite (pytest)
├── .claude/                 # agents · skills · commands · schemas · settings · manifest
└── scripts/                 # rename · verify-template · lint-* · leak-guard · export_openapi
                             # + hooks/guard-bash.sh
```

## Commands

```sh
make ci                  # fmt-check + mypy + ruff/import-linter + gen-check + unit + acceptance + evals
make vet                 # mypy src (strict)
make lint                # ruff check . && lint-imports
make test                # pytest tests/unit
make accept              # pytest tests/acceptance (real server, fake adapters)
make generate            # regenerate specs/openapi.yaml from the app
make gen-check           # fail if the committed OpenAPI contract is stale
make migrate-up          # apply migrations (needs <APP>_DATABASE_URL)
make migrate-add NAME=add_widgets_status
make vuln                # pip-audit
docker compose up --build            # postgres + migrations + api + worker
bash scripts/verify-template.sh      # staged renamed copy + factory smoke
```

## Dependency rules (enforced by import-linter)

| From                                            | May depend on                                   |
| ----------------------------------------------- | ----------------------------------------------- |
| `src/app/bin/*`                                 | anything (the roots know every slice)           |
| feature `http.py` / `postgres.py` / `module.py` | own slice, `app.platform.*`, `app.shared.*`     |
| `usecase.py`                                    | `ports.py`, `domain.py`, platform, shared       |
| `ports.py`                                      | `domain.py`                                     |
| `domain.py`                                     | stdlib, `app.shared.*` — **zero I/O**           |
| `src/app/platform/*`                            | `app.shared.*`, stdlib, SQLAlchemy — never `app.features.*` |
| anything outside a feature                      | that feature's `module.py` internals            |
| anything                                        | `src/app/bin/**` (nothing depends on the root)  |

Slices never import each other. Shared behavior moves down into `app.shared` /
`app.platform`, or sideways through a port the consumer declares. Imports are
absolute everywhere (`from app.features.widgets.domain import …`), including
within a slice — there are no relative imports in `src/app/`.

Every port method returns a domain value or raises an `AppError`; adapters map
driver errors inward. Consumers hold the port protocol, never the `Pg…`
adapter class. SQL lives only in a slice's `queries.py` (review-enforced; the
import-linter contracts enforce the layer edges).

## Slice playbook (add a feature)

1. Create `src/app/features/<slug>/` with the standard seven files:
   `domain.py` (pure types + errors), `ports.py` (interfaces the use case
   needs), `usecase.py`, `http.py`, `queries.py`, `postgres.py`, `module.py`.
2. Add a paired Alembic migration (`make migrate-add NAME=...`); fill both
   `upgrade` and `downgrade`. Audit the slice's SQL against the migration
   columns.
3. Write the use case against the ports; add unit tests with in-package fakes.
4. Wire in `src/app/bin/api.py` only — construct the adapter, inject it, call
   the slice's `register`.
5. Add `specs/features/<slug>.feature` if a behavioral contract exists; run
   `make ci` and `make accept`.

The `PreToolUse` guard hook (`.claude/settings.json` +
`scripts/hooks/guard-bash.sh`) intercepts every `Bash` tool call and flags
high-risk shell patterns; it is not something you invoke.

## Language rules (beautifulpython)

The deep contract is `PLAYBOOK_BACKEND.md`; these are the non-negotiables:

- `mypy --strict` is the floor. No `Any` in shipped signatures; use `object`
  and narrow once at the boundary. Explicit return annotations on every
  public function.
- Make invalid states unrepresentable: `StrEnum` over bare strings,
  `@dataclass(frozen=True)` for data a callee must not mutate, no
  `dict[str, Any]` where a model fits.
- No boolean parameters. Use a two-variant enum, a named options object, or
  two functions.
- The event loop is not blocked: no sync file/HTTP/DB calls on a request path;
  no unbounded `asyncio.gather` over user-sized input; every external call
  gets a timeout.
- No floating tasks. Await, keep a reference and cancel it on shutdown, or
  `asyncio.TaskGroup`.
- Errors are typed values with stable codes (`app/shared/apperr.py`). Match on
  `Code`, never on message text. Never `except Exception: pass`.
- Structured logging only through `logging` (`app/platform/logging.py`); no
  `print` outside a script's own CLI output.
- Tests fake the ports, inject the clock (`FixedClock`), and never touch real
  Postgres/Redis/network in the unit tier.

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
| `docs/plans/`, `specs/openapi.yaml`, `specs/features/**`                              | `planner`, `architect`, `api-designer`                            |
| `tests/**`, `specs/features/**`                                                       | `test-author`                                                     |
| `src/app/features/<slice>/**`                                                         | the slice's implementer                                           |
| `src/app/shared/**`, `src/app/platform/**`, `pyproject.toml`, `.github/**`             | shared — integrator role                                          |
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
the tree against `.claude/schemas/`; `evals/contracts_test.py` is the offline
contract suite.

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
| `PLAYBOOK_TESTS.md`       | Test pyramid, pytest/pytest-bdd, contract evals, gates, audit evidence |
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

- `asyncio` task ownership threads through every I/O boundary; use cases
  accept the signal they need to cancel (`asyncio.Event` / `CancelledError`)
  and honor it at loop yields.
- Errors: raise `AppError` subclasses (or `E(Code, detail, status)`); stable
  codes live in `src/app/shared/apperr.py`, rendered as `{detail, code}` only
  in `http.py` / `platform/envelope.py`.
- Interfaces live in the consumer slice's `ports.py`, kept small (1–3
  methods), typed with `typing.Protocol`.
- No task is started without an owner: workers shut down on the process
  signal; `asyncio.gather` only for genuinely independent work.
- Migrations never run at startup; `make migrate-up`, or the compose one-shot
  `migrations` service, is the deploy/boot step you control.
- `specs/openapi.yaml` is generated from the app (`make generate`) and
  committed; never hand-edit it. `make gen-check` fails when it is stale.
- This project was instantiated from a template: the substitution tokens
  documented in `scripts/rename.sh` were replaced at instantiation time. If
  one survives verbatim outside `evals/fixtures/` or the linter/leak-guard
  scripts, it is a bug, not a template artifact.
- Stack delta from beautifulpython's published digest: this skeleton uses
  SQLAlchemy Core `text()` in `queries.py` (no ORM entities), a DB outbox
  (not Celery/Redis), constructor injection, stdlib `logging`, and
  argon2id. The dependency law, zero-I/O domain, error envelope and outbox
  contract are the same. Swap an adapter, never the contract.
