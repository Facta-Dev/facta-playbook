# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a Chrome extension
(Manifest V3) skeleton: TypeScript + React 19 side panel, a classic service
worker, zod contracts shared by both ends of the message protocol, a typed
storage layer with schema versions, and zero host permissions.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output.
- **Never fabricate** paths, API names, message types, env vars, test results
  or commit hashes. Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **Never commit a lockfile.** The template ships without one; the first
  `pnpm install` happens after `scripts/rename.sh`, on the renamed copy.

## Never

- **No blanket host permissions.** `host_permissions` stays `[]`; a host is
  granted per use case, through `optional_host_permissions` or `activeTab`,
  and every grant needs a `docs/permissions-register.md` entry.
- **No `debugger` permission, ever.** It is not in the manifest, not in
  `optional_permissions`, and not proposed in a plan.
- **No `chrome.*` outside `src/platform/**`.** Feature code, contracts, UI
  and ViewModels go through the platform adapters; ESLint
  (`no-restricted-globals`) and dependency-cruiser
  (`no-chrome-outside-platform`) fail the build when they do not.
- **No secrets.** Nothing sensitive in the bundle, the manifest, the logs or
  the telemetry. The extension is a public client: PKCE, no client secret, no
  token in `storage.local`.
- **No remote code.** No `eval`, no `new Function`, no script fetched from a
  URL, no `unsafe-eval` in the CSP.

## Layout

```
<app>/
├── entrypoints/                  # THIN: composition roots only
│   ├── background.ts             # service worker: builds the container, registers the RPC server
│   ├── content/example.content.ts
│   ├── options/{index.html,main.ts}
│   └── sidepanel/{index.html,main.tsx}
├── src/
│   ├── shared-kernel/            # ids · result · errors — pure, imported by everyone
│   ├── contracts/                # zod: protocol · notes · storage · policy · errors
│   ├── platform/                 # THE ONLY place that touches chrome.*
│   │   ├── runtime.ts            #   the message transport
│   │   ├── storage.ts            #   local / session / managed areas
│   │   └── sidepanel.ts          #   the panel behaviour
│   ├── messaging/                # typed RPC client + server, sender validation
│   ├── storage/                  # typed stores, schema versions, migrations
│   ├── policy/                   # managed policy reader + resolver
│   ├── telemetry/                # logger + redaction
│   ├── security/                 # sender allowlist, URL matchers
│   ├── features/notes/           # the example slice — replace with your domain
│   └── manifest.ts               # MV3 manifest, written to dist/manifest.json by the build
├── packages/sdk/                 # typed API client (no React, no chrome)
├── public/_locales/en/messages.json
├── tests/
│   ├── notes/frontend/{unit,contract}/
│   ├── _platform/{architecture,security}/
│   └── _shared/fakes/
├── evals/                        # native contract tests (node --test) + live smoke
├── docs/                         # plans/ · adr/ · permissions-register.md (created on demand)
├── managed-schema.json           # the storage.managed schema (admin policy)
├── .claude/                      # agents · skills · commands · schemas · settings · manifest
├── scripts/                      # rename · verify-template · lint-* · leak-guard
│                                # · instruction-budget · install-claude-runtime
│                                # · hooks/guard-bash.sh
├── PLAYBOOK_*.md                # seven local contracts — read on demand
├── AGENTS.md                    # agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md                    # durable premise: slices, permissions, protocol, storage
├── Makefile                     # ci · build · test · lint · format · verify
└── package.json · vite.config.ts · vitest.config.ts · tsconfig*.json
```

## The layers and their rules

| Layer                                     | May import                           | Must not import                |
| ----------------------------------------- | ------------------------------------ | ------------------------------ |
| `src/shared-kernel/`                      | nothing but the stdlib               | everything internal            |
| `src/contracts/`                          | `zod`, `shared-kernel`               | everything else                |
| `src/features/<slice>/domain/`            | `shared-kernel`                      | `chrome`, React, DOM, adapters |
| `src/features/<slice>/application/`       | domain, `shared-kernel`, `contracts` | adapters, `chrome`, DOM        |
| `src/features/<slice>/adapters/{in,out}/` | application, `contracts`, `platform` | another slice's internals      |
| `src/platform/`                           | `chrome`                             | application, domain            |
| `src/features/<slice>/ui/`                | ViewModel hooks, React               | `chrome`, storage, SDK         |
| `src/features/<slice>/viewmodels/`        | port types                           | `react-dom`, `chrome`          |

Three invariants make the hexagon hold, and each one has a machine check:

1. **`src/platform/**` owns `chrome.*`.** ESLint forbids the `chrome` and
   `browser` globals outside that folder; dependency-cruiser fails on a
   `chrome` module import from anywhere else.
2. **Features never import each other.** A slice is sealed; the composition
   root in `entrypoints/` wires the slices together.
3. **Both ends of a message use the same zod schemas** in
   `src/contracts/protocol.ts`. A feature never calls `sendMessage` directly.

## Gates

```sh
make ci                  # pnpm typecheck + lint (eslint + dependency-cruiser) + test + build
pnpm run typecheck       # tsc -p tsconfig.json + tsconfig.test.json, no emit
pnpm run lint            # eslint . && depcruise --config dependency-cruiser.config.cjs .
pnpm run test            # vitest run (unit + contract + architecture + security)
pnpm run build           # vite build → dist/
pnpm run evals           # node --test evals/contracts.test.mjs (native, zero deps)
pnpm run format          # prettier --write .   (pnpm run format:check to verify)
bash scripts/verify-template.sh   # stages a renamed copy and runs every gate on it
```

`pnpm install` needs a network and a lockfile the template does not ship;
run it on the renamed copy, never in the template root.

## The TDD loop

Spec → plan → red → green → refactor:

1. `/spec` writes `docs/specs/<slug>.md` (behaviour, acceptance criteria,
   non-goals, permission surface, open questions); `/plan` writes
   `docs/plans/<slug>.md` (problem, constraints, design, files, risks, tests).
2. `/add-slice` or `/add-message` scaffolds the shape (slice folders, zod
   schemas, the protocol entry, the sender-class entry).
3. Write the failing test first (`/tdd-guide`, `test-author`): a use-case
   test against a fake port, or a contract test against the shared schemas.
   Watch it fail for the right reason — a missing symbol, not a typo.
4. Implement the minimum that makes it pass, then refactor with the suite as
   the safety net.
5. `/review` for the pre-landing checklist; `make ci` before declaring done.

Every new message type ships with two tests: a contract test (both ends
parse the same envelope) and a sender-class test (each type × each allowed
sender class). Every storage change ships with a migration and a fixture of
the previous version.

## Who edits what

| Area                                                                                                                                      | Owner                                                             |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `docs/plans/`, `docs/specs/`                                                                                                              | `planner`, `architect`                                            |
| `tests/**`                                                                                                                                | `test-author`                                                     |
| `src/features/**`, `entrypoints/**`                                                                                                       | implementers (`build-resolver`, `tdd-guide`)                      |
| `src/platform/**`, `src/contracts/**`, `src/shared-kernel/**`, `src/manifest.ts`, `package.json`, `managed-schema.json`, `pnpm-lock.yaml` | shared — integrator role                                          |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**`                                                      | human only; an agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (slice, spec); name sessions `<slice>:<spec-id>`.
- Do not edit a slice claimed by another session — message its owner instead.
- Changing a published contract (a zod schema, a message type, a storage
  version)? Message the dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`core`, `client`,
`security`, `testing`). `.claude/skills/` + `.claude/commands/` ship
reusable workflows; every command is a thin shim delegating 1:1 to the skill
of the same slug. New slugs are discovered by walking the tree — no registry
edit. `scripts/lint-agents.sh`, `lint-skills.sh` and `lint-commands.sh`
validate the tree against `.claude/schemas/`.

Skills: **`/add-slice`** (scaffold a slice), **`/add-message`** (schema +
sender class + handler + contract test), **`/permission-change`** (register
entry + justification + test), **`/storage-migration`** (version bump +
migration + fixture), **`/review`**, **`/spec`**, **`/plan`**, **`/learn`**,
**`/ship`** (release PR; user-invoked only).

## Playbooks

Seven local contracts ship next to this file: `PLAYBOOK_CHROME_EXTENSION.md`
(architecture, manifest, messaging, storage, security),
`PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`, `PLAYBOOK_FRONTEND.md`,
`PLAYBOOK_URLS.md`, `PLAYBOOK_AUDIT.md` and `PLAYBOOK_CLAUDE_CODE.md`.

They are **restrictions, not suggestions**: a change that contradicts one is
wrong until the playbook changes in the same pull request. They load **on
demand only**: adopt one by adding a rule under `.claude/rules/` whose
`paths:` frontmatter names the files it guards and whose body names the
playbook — then read that playbook before editing those files. Never
`@`-import a playbook: the always-on set (this file plus the path-less rules)
must stay inside the harness budget, and `scripts/instruction-budget.sh`
fails the build when it does not.

## Learnings

`.claude/learnings.jsonl` is an append-only, project-local log of pitfalls,
patterns and preferences discovered while working on `<app>` (created on the
first `/learn`). Grep it by `kind` before repeating work that resembles a
logged pitfall.

## Conventions

- Errors are values (`Result<T, RpcError>`), never exceptions thrown across
  an execution context; the codes live in `src/shared-kernel/errors.ts`.
- The sender is validated **before** anything is parsed: id, origin, frame,
  then the per-type sender allowlist in the protocol table.
- `crypto.randomUUID()` and `crypto.getRandomValues`, never `Math.random`.
- Storage is single-writer: the service worker mutates durable state, under
  a serialized read-modify-write queue.
- This project was instantiated from a template: the three substitution
  tokens documented in `scripts/rename.sh` were replaced at instantiation
  time. If one survives verbatim outside the frozen fixtures, it is a bug,
  not a template artifact.
