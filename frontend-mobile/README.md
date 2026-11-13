# `<app>` — React Native + Expo mobile skeleton

A mobile app template: Expo SDK 58 + React Native 0.88 + TypeScript 6 in a
pnpm workspace, the hexagonal layers of the mobile playbook (pure domain and
application, framework-free view models, React Native only at the views), a
shared route table, a device-tier quality governor and one example slice
(`notes`). Every `<app>`, `<APP>` and `<scope>` token is a **literal
placeholder** — the package name is invalid until you rename, so the project
cannot build (and no lockfile is committed). See [How to apply](#how-to-apply).

## Layout

```
<app>/
├── app/                    # the composition root: index.ts, App.tsx, app.json
│                           # and the metro.config.js of the workspace
├── packages/
│   ├── core/               # the shared kernel: Result, device tier, quality
│   │                       # governor, ports
│   ├── contracts/          # zod DTOs and the route table
│   └── feature-notes/      # the example slice: domain · application ·
│                           # adapters · viewmodels · views
├── tests/                  # notes/unit · core · contracts · _platform
├── evals/                  # native contract tests (node --test) + live smoke
├── .claude/                # agents · skills · commands · schemas · settings
├── scripts/                # rename · verify-template · lint-* · leak-guard ·
│                           # instruction-budget · install-claude-runtime
├── PLAYBOOK_*.md           # seven local contracts — read on demand
├── AGENTS.md               # agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md               # durable premise: slices, ports, routes, pins
└── Makefile                # ci · build · test · lint · format · verify
```

## The layer model

| Layer                | Imports                     | Never imports                 |
| -------------------- | --------------------------- | ----------------------------- |
| `packages/core`      | the stdlib                  | React, React Native, Expo     |
| `packages/contracts` | `zod`, the core             | React, React Native, Expo     |
| `domain`             | the core                    | React, React Native, Expo     |
| `application`        | domain, core, contracts     | React, React Native, adapters |
| `viewmodels`         | the application types       | React Native, Expo            |
| `views`              | the view-model state, React | adapters, repositories        |
| `app`                | everything                  | — it only wires the layers    |

The boundaries are enforced twice: ESLint fails the build on a forbidden
import, and `tests/_platform/architecture/import-boundaries.test.ts` reads
the source files and fails the build on the same violation.

## The device tier and the quality governor

The app classifies the phone as `low`, `mid` or `high` from the low-RAM
flag, the RAM size, the OS level and the measured p95 frame time. The
measurement can lower the hint, never raise it. `governQuality` turns the
tier plus the thermal state, the memory pressure and the battery saver into
a `QualityProfile` — render scale, frame budget, shadows, texture
resolution, particle budget. It is a pure function: the runtime loop, not
the governor, owns the hysteresis.

## The route model

One table (`packages/contracts/src/routes.ts`) and one grammar:
`/{userType}/{module}/{resource}[/{resourceId}]`, plus the session-less
routes `/onboarding`, `/login`, `/register`, `/forgot-password`,
`/reset-password`, `/verify-email`. Screens call the builders; the parser
is the only way back in. The `{userType}` segment is an experience
namespace, never an authorization.

## The gate

```sh
make ci                  # pnpm typecheck + lint + test + android export
pnpm run evals           # node --test evals/contracts.test.mjs (native, zero deps)
```

`pnpm install` needs a network and writes the lockfile the template does not
ship — run it on the renamed copy, never in the template root.

## Docs

The long-form documentation lives in [`docs/`](docs/README.md): the platform
model and the Expo runtime, the device tiers and the quality governor, the
hexagonal layers, the directory structure, the route grammar and deep links,
the offline outbox, the security model, push notifications, telemetry, the
testing tiers and the playbooks index.

## Working with agents

`AGENTS.md` (symlinked as `CLAUDE.md`) is the operating instruction file:
hard rules, the never-list, the layers and their rules, the gates, the spec →
test → code workflow, the playbook index. `MEMORY.md` holds the premise —
slices, the ports, the device tier and the governor, the route table, the
version pins that must not drift and the test tiers.

Seven local contracts ship next to this file and load **on demand** through
path-scoped rules, never as `@`-imports: `PLAYBOOK_FRONTEND_MOBILE.md`
(architecture, device classes, the quality governor, navigation, push),
`PLAYBOOK_FRONTEND.md`, `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_URLS.md` (the route grammar), `PLAYBOOK_AUDIT.md` (SOC 2 / ISO
27001 / ISO 9001 programs) and `PLAYBOOK_CLAUDE_CODE.md` (the kit, the SDD
loop, the governance).

The `.claude/` layer ships the agents, the skills with their 1:1 slash
commands, schema-driven linters in `scripts/`, a `PreToolUse` Bash guard and
a `manifest.json` target matrix for installing the kit into other agent
runtimes.

## How to apply

1. Copy this directory to your repo root.
2. `bash scripts/rename.sh <app> <scope>` — the script applies the
   `<scope> → <APP> → <app>` substitutions in safe order.
3. `pnpm install` — creates the lockfile; keep it out of the template.
4. `pnpm format` — one run: the substituted tokens re-align the Markdown
   tables.
5. `make ci` — green.
6. Replace the `notes` example slice with your domain.

### Installing the `.claude` runtime

The kit installs the same agents, skills and commands into other agent
runtimes (Cursor, Codex, Gemini, Zed, …) from the target matrix in
`.claude/manifest.json`:

```sh
bash scripts/install-claude-runtime.sh                        # claude + claude-project
bash scripts/install-claude-runtime.sh --targets=cursor,codex # opt-in runtimes
```

It is idempotent and never clobbers a file you already own — a pre-existing
destination is skipped with a warning, and every operation is recorded in
`.claude/install-state.json`. `bash scripts/verify-template.sh` stages a
throwaway copy under `/tmp`, renames it and runs every gate on the copy.
