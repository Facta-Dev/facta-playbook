# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a React Native +
Expo mobile skeleton: TypeScript, a plain `App.tsx` entry (no expo-router),
a pnpm workspace with the shared kernel, the contracts and one example
slice, and a device-tier quality governor. The layers are hexagonal: the
domain and the application are pure TypeScript, the view models hold state
and commands without the native framework, and React Native only enters at
the views.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions. No Spanish anywhere in the repo.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output.
- **Never fabricate** paths, API names, status names, env vars, test results
  or commit hashes. Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **Never commit a lockfile.** The template ships without one; the first
  `pnpm install` happens after `scripts/rename.sh`, on the renamed copy.

## Never

- **No React Native or Expo below the views.** The core, the contracts, the
  domain and the application layer are pure TypeScript: `react`,
  `react-native` and `expo` are banned there. The view models may use
  `react` (hooks), but never `react-native` or `expo`. A capability that
  needs a native module is a **port** (`packages/core/src/ports.ts`) plus an
  adapter, never an import. ESLint (`no-restricted-imports`) and the
  architecture suite (`tests/_platform/architecture/`) fail the build when
  they are.
- **No `expo-router`.** The entry is a plain `App.tsx` registered by
  `app/index.ts`. A route is a **string built from the route table**
  (`packages/contracts/src/routes.ts`), never a hand-written path in a
  screen.
- **No secrets in the bundle.** The app ships to every user's phone: only
  `EXPO_PUBLIC_`-prefixed variables are inlined, and they are public.
  `app/app.json` declares no key and no secret; the eval suite fails the
  build when it does.
- **No version drift.** `react` is pinned to **19.3.0** exactly, `expo` to
  the **SDK 58** range and `react-native` to **0.88.1**. A newer patch of
  `react` breaks `expo-doctor`; do not "fix" the pin.
- **No hand-rolled networking in a view.** HTTP goes through the SDK, behind
  a port.
- **No `Date.now()` for identity.** Identifiers come from the injected
  `newId`, which uses `crypto.randomUUID()` when the runtime has it.

## Layout

```
<app>/
├── app/                          # THIN: the composition root
│   ├── index.ts                  #   registerRootComponent(App)
│   ├── App.tsx                   #   builds the adapter, wires the view model
│   ├── app.json                  #   Expo config: slug, scheme, bundle ids
│   └── metro.config.js           #   the workspace watch folders (monorepo)
├── packages/
│   ├── core/                     # the shared kernel — pure TypeScript
│   │   └── src/
│   │       ├── result.ts         #   Result<T, E>: errors are values
│   │       ├── device-tier.ts    #   low | mid | high, from signals
│   │       ├── quality-governor.ts # tier + thermal + memory -> profile
│   │       └── ports.ts          #   PushPort, SecureStorePort, …
│   ├── contracts/                # zod DTOs + the route table — pure TypeScript
│   │   └── src/{notes.ts,routes.ts}
│   └── feature-notes/            # the example slice — replace with your domain
│       └── src/
│           ├── domain/           #   entities and their invariants
│           ├── application/      #   use cases and the ports of the slice
│           ├── adapters/out/     #   the outbound adapters (in-memory repo)
│           ├── viewmodels/       #   state and commands, no native framework
│           └── views/            #   React Native components + the React hook
├── tests/
│   ├── notes/unit/{domain,application,viewmodels}/
│   ├── core/                     # the kernel (quality governor)
│   ├── contracts/                # the route table
│   └── _platform/architecture/   # the layer boundaries, read from the source
├── evals/                        # native contract tests (node --test) + live smoke
├── .claude/                      # agents · skills · commands · schemas · settings
├── scripts/                      # rename · verify-template · lint-* · leak-guard
│                                # · instruction-budget · install-claude-runtime
│                                # · hooks/guard-bash.sh
├── PLAYBOOK_*.md                # seven local contracts — read on demand
├── AGENTS.md                    # agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md                    # durable premise: slices, ports, routes, pins
├── Makefile                     # ci · build · test · lint · format · verify
└── package.json · pnpm-workspace.yaml · vitest.config.mts · tsconfig*.json
```

## The layers and their rules

| Layer                                  | May import                         | Must not import                     |
| -------------------------------------- | ---------------------------------- | ----------------------------------- |
| `packages/core/src/`                   | the stdlib                         | React, React Native, Expo           |
| `packages/contracts/src/`              | `zod`, `@<scope>/core`             | React, React Native, Expo           |
| `packages/feature-*/src/domain/`       | `@<scope>/core`                    | React, React Native, Expo, storage  |
| `packages/feature-*/src/application/`  | domain, `@<scope>/core`, contracts | React, React Native, Expo, adapters |
| `packages/feature-*/src/adapters/out/` | application, contracts, core       | another slice's internals, views    |
| `packages/feature-*/src/viewmodels/`   | application ports and types        | React Native, Expo, the router      |
| `packages/feature-*/src/views/`        | view-model state, the React hook   | adapters, repositories, the SDK     |
| `app/` (the composition root)          | everything                         | — it only wires the layers          |

Three invariants make the hexagon hold, and each one has a machine check:

1. **React Native and Expo enter only at the views.** ESLint forbids
   `react`, `react-native` and `expo` in the core, the contracts, the domain
   and the application layer, and forbids `react-native` and `expo` in the
   view models — where `react` (hooks) is allowed. The architecture suite
   reads the source files and fails the build when it finds one.
2. **A view never reaches for an adapter.** The view renders the view-model
   state and calls the view-model commands. The composition root
   (`app/App.tsx`) is the only place that builds an adapter.
3. **One route table.** Paths are built from
   `packages/contracts/src/routes.ts` and parsed back by the same table.

## Gates

```sh
make ci                  # pnpm typecheck + lint + test + export (android)
pnpm run typecheck       # tsc over the app program and the test program, no emit
pnpm run lint            # eslint: type-aware rules + architecture guards
pnpm run test            # vitest run (unit + architecture)
pnpm run export:android  # expo export --platform android
pnpm run evals           # node --test evals/contracts.test.mjs (native, zero deps)
pnpm run format          # prettier --write .   (pnpm run format:check to verify)
bash scripts/verify-template.sh   # stages a renamed copy and runs every gate on it
```

`pnpm install` needs a network and a lockfile the template does not ship;
run it on the renamed copy, never in the template root.

## The TDD loop

Spec → plan → red → green → refactor:

1. `/spec` writes `docs/specs/<slug>.md` (behaviour, acceptance criteria,
   non-goals, the permission surface, the device-tier behaviour, the open
   questions); `/plan` writes `docs/plans/<slug>.md` (problem, constraints,
   design, files, risks, tests).
2. Scaffold the shape: the slice folders, the domain entity, the port, the
   use case, the view model and the view.
3. Write the failing test first (`/tdd-guide`, `test-author`): a use-case
   test against a fake port, or a view-model test against a fake
   repository. Watch it fail for the right reason — a missing symbol, not a
   typo.
4. Implement the minimum that makes it pass, then refactor with the suite
   as the safety net.
5. `/review` for the pre-landing checklist; `make ci` before declaring done.

Every new status, port or route ships with a test: the view-model statuses
against the eight of the contract, the route builders against the parser,
and the architecture suite against the layer that changed.

## Who edits what

| Area                                                                                               | Owner                                                             |
| -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `docs/plans/`, `docs/specs/`                                                                       | `planner`, `architect`                                            |
| `tests/**`                                                                                         | `test-author`                                                     |
| `packages/feature-*/src/**`, `app/**`                                                              | implementers (`build-resolver`, `tdd-guide`)                      |
| `packages/core/**`, `packages/contracts/**`, `package.json`, `pnpm-workspace.yaml`, `app/app.json` | shared — integrator role                                          |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**`               | human only; an agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (slice, spec); name sessions `<slice>:<spec-id>`.
- Do not edit a slice claimed by another session — message its owner instead.
- Changing a published contract (a zod schema, a route row, a port)? Message
  the dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`core`, `client`,
`security`, `testing`). `.claude/skills/` + `.claude/commands/` ship
reusable workflows; every command is a thin shim delegating 1:1 to the skill
of the same slug. New slugs are discovered by walking the tree — no registry
edit. `scripts/lint-agents.sh`, `lint-skills.sh` and `lint-commands.sh`
validate the tree against `.claude/schemas/`.

Skills: **`/add-slice`** (scaffold a slice), **`/add-port`** (declare a port
and its adapter), **`/add-route`** (a row of the route table plus its test),
**`/review`**, **`/spec`**, **`/plan`**, **`/learn`**, **`/ship`** (release;
user-invoked only).

## Playbooks

Seven local contracts ship next to this file: `PLAYBOOK_FRONTEND_MOBILE.md`
(architecture, device classes, the quality governor, navigation, push),
`PLAYBOOK_FRONTEND.md`, `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_URLS.md` (the route grammar), `PLAYBOOK_AUDIT.md` and
`PLAYBOOK_CLAUDE_CODE.md`.

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

- Errors are values (`Result<T, E>`), never exceptions thrown across a
  layer; the codes live with the use case that produces them.
- The device tier is a hint the measurement may lower, never raise; the
  quality governor only steps down from the tier baseline, and stepping up
  again needs a sustained healthy period.
- The status of a view model is one of the eight of the contract:
  `loading | ready | empty | forbidden | not-found | offline | error |
policy-blocked`.
- A path is built from the route table; the `{userType}` segment is an
  experience namespace and never an authorization.
- Identifiers come from the injected `newId`, timestamps from the injected
  `now`: the domain never reads the clock.
- This project was instantiated from a template: the three substitution
  tokens documented in `scripts/rename.sh` were replaced at instantiation
  time. If one survives verbatim outside the frozen fixtures, it is a bug,
  not a template artifact.
