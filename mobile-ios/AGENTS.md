# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a native iOS app
skeleton: Swift 6 with strict concurrency, SwiftUI, iOS 18, an XcodeGen
`project.yml` that generates the project file, six local SwiftPM packages, a
UDF `notes` slice, and an explicitly wired composition root.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, resources, docs, test names
  and tool descriptions. No Spanish anywhere in the repo.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output. Off macOS the native tier prints a
  visible `SKIP` and exits 0 — report the SKIP, never fake a build.
- **Never fabricate** paths, target names, API names, versions, test results
  or commit hashes. Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **The template ships literal placeholders.** `<app>`, `<APP>` and `<scope>`
  are not valid Swift module or bundle-identifier segments, so the template
  root does not build. Run `bash scripts/rename.sh <app> <scope>` on a copy
  before the first build.
- **The project file is generated.** `.xcodeproj` is git-ignored; build
  settings live in `project.yml`. Never edit the generated project.

## Never

- **No SwiftUI or UIKit import in a pure package** (`Kernel`,
  `NotesContracts`, `NotesDomain`, `UICore`). The framework is not on the
  classpath, so it does not compile. `scripts/check-architecture.sh` names the
  rule before the compiler does.
- **No `NotesUI → NotesData` dependency.** The UI never picks an
  implementation; the app target does.
- **No string routes.** Routes are a typed enum in `NotesContracts`; a screen
  is entered by the enum case, and `navigationDestination` handles every case
  exhaustively.
- **No view, `UIViewController` or `UIScreen` inside a model or a domain
  type.** A model that holds a view leaks it and cannot be tested off the main
  actor.
- **No new capability without a review.** The template declares no
  entitlement and no `Info.plist` usage key; a capability needs a register
  entry, a justification, a test and a security review
  ([PLAYBOOK_SECURITY.md](PLAYBOOK_SECURITY.md)).
- **No secrets** in the bundle, `Info.plist`, user defaults, logs or
  telemetry. Signing material and API keys stay outside the tree.
- **No dependency-injection framework in the template.** `AppEnvironment`
  builds the object graph by hand ([ADR 0003](docs/adr/0003-explicit-app-environment.md)).
- **No business logic in a `View`.** It re-renders and it is not covered by a
  unit test.
- **No cross-slice imports.** A shape shared by two slices moves to `Kernel`
  (pure types) or the slice's `Contracts` package.

## Layout

```
<app>/
├── Apps/<APP>/Sources/
│   ├── <APP>App.swift            # the @main App — the only app entry point
│   ├── AppEnvironment.swift      # the composition root: builds the object graph
│   ├── Router.swift              # the typed navigation path
│   └── RootView.swift            # NavigationStack + navigationDestination
├── Packages/
│   ├── Core/
│   │   ├── Kernel/               # pure Swift: Outcome, AppError, ErrorCode, NoteID
│   │   └── UICore/               # SwiftUI-free: UiStatus, the closed status set
│   └── Feature/Notes/            # the example slice — replace with your domain
│       ├── Contracts/            # pure Swift: NotesRoute, the typed routes
│       ├── Domain/               # pure Swift: Note, NoteRepository, LoadNotes
│       ├── Data/                 # the in-memory adapter
│       └── UI/                   # SwiftUI screens + @Observable model
├── evals/                        # the .claude contract tests (node --test) + live tier
├── scripts/                      # rename · verify-template · check-architecture
│                                 # · native-gates · lint-* · leak-guard
│                                 # · instruction-budget · install-claude-runtime
│                                 # · hooks/guard-bash.sh
├── .claude/                      # agents · skills · commands · schemas · settings · manifest
├── docs/                         # 00–14 · adr/ · admin-guide · data-flows · dependency-register
├── PLAYBOOK_*.md                 # seven local contracts — read on demand
├── project.yml                   # XcodeGen: one app target, six local packages
├── Package.swift files           # one per package, all in Swift 6 language mode
├── AGENTS.md                     # this file; CLAUDE.md is a symlink to it
├── MEMORY.md                     # durable premise: packages, routes, UDF, stack
├── Makefile                      # ci · test · build · evals · lint · generate · verify
└── .gitignore · .env.example
```

## The packages and their rules

| Package | Kind | May depend on | Must not |
| --- | --- | --- | --- |
| `Kernel` | Pure Swift | the standard library | anything internal |
| `UICore` | Pure Swift (UI types) | `Kernel` | a feature package, SwiftUI, UIKit |
| `NotesContracts` | Pure Swift | the standard library | `Kernel` (keep it dependency-free), Data, UI |
| `NotesDomain` | Pure Swift | `Kernel` | SwiftUI, UIKit, adapters, UI |
| `NotesData` | Adapter | `NotesDomain`, `Kernel` | `NotesUI` |
| `NotesUI` | SwiftUI | `NotesDomain`, `NotesContracts`, `Kernel`, `UICore` | `NotesData` |
| `<APP>` app target | Application | every package | business rules |
| `evals` | Node contract tests | nothing app-side | the Swift packages |

Three invariants make the architecture hold, and each one has a machine check:

1. **The package graph is the architecture.** The dependency edge is declared
   in `Package.swift`; a forbidden edge is a build or gate failure, not a code
   review preference. `scripts/check-architecture.sh` checks the edges the
   compiler cannot.
2. **State flows down, events flow up, effects leave once.**
   `NotesViewState` is an immutable value, the view sends `NotesEvent`, and a
   one-shot `NotesEffect` leaves through an `AsyncStream` with a single
   consumer in the app target.
3. **Routes are typed cases, not strings.** `NotesRoute` is a `Hashable`
   enum in a pure package; the app's `navigationDestination` maps each case to
   a view, exhaustively.

## Gates

```sh
make ci                          # lint + contract evals + native gates
make lint                        # check-architecture; after rename, also the .claude linters + instruction budget
make evals                       # node --test evals/contracts.test.mjs
bash scripts/check-architecture.sh
bash scripts/native-gates.sh     # swift test per package + xcodegen + xcodebuild (macOS)
bash scripts/native-gates.sh test
bash scripts/native-gates.sh build
bash scripts/verify-template.sh  # stages a renamed copy and runs every gate on it
```

The portable tier runs anywhere with `node` and `bash`. The native tier needs
macOS with Xcode and XcodeGen; on any other host it prints
`SKIP native-gates (macOS + Xcode required)` and exits 0. The first native
build needs a network for SwiftPM resolution and an installed iOS 18
simulator runtime.

## The TDD loop

Spec → plan → red → green → refactor:

1. `/spec` writes `docs/specs/<slug>.md` (behaviour, acceptance criteria,
   non-goals, route and capability surface, open questions); `/plan` writes
   `docs/plans/<slug>.md` (problem, constraints, design, files, risks, tests).
2. `/add-slice` or `/add-package` scaffolds the shape; `/add-route` adds a
   typed case and its destination; `/add-port` adds a domain port and its
   adapter.
3. Write the failing test first (`/tdd-guide`, `test-author`): a use-case test
   against a fake port, or a model test on the main actor. Watch it fail for
   the right reason — a missing symbol, not a typo.
4. Implement the minimum that makes it pass, then refactor with the suite as
   the safety net.
5. `/review` for the pre-landing checklist; `make ci` before declaring done.

Every new screen ships with a model test covering `ready`, `empty` and one
failure status. Every new route ships with a round-trip test and an exhaustive
destination. Every new port ships with a fake and a use-case test.

## Who edits what

| Area | Owner |
| --- | --- |
| `docs/specs/`, `docs/plans/`, `docs/adr/` | `planner`, `architect` |
| test sources in every package | `test-author` |
| `Packages/**`, `Apps/**` | implementers (`build-resolver`, `tdd-guide`) |
| `project.yml`, `scripts/**`, the root files | shared — integrator role |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**` | human only; an agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (package, spec); name sessions `<package>:<spec-id>`.
- Do not edit a package claimed by another session — message its owner
  instead.
- Changing a published contract (a route case, a port, the `UiStatus` set)?
  Message the dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`core`, `client`,
`security`, `testing`). `.claude/skills/` + `.claude/commands/` ship reusable
workflows; every command is a thin shim delegating 1:1 to the skill of the
same slug. New slugs are discovered by walking the tree — no registry edit.
`scripts/lint-agents.sh`, `lint-skills.sh` and `lint-commands.sh` validate the
tree against `.claude/schemas/`; `node --test evals/contracts.test.mjs`
asserts the same contract.

Skills: **`/add-slice`** (scaffold a slice across four packages),
**`/add-package`** (a new Swift package with its layer rules),
**`/add-route`** (typed case + destination + deep link),
**`/add-port`** (domain port + adapter + stub + test), **`/review`**,
**`/spec`**, **`/plan`**, **`/learn`**, **`/ship`** (release; user-invoked
only).

## Playbooks

Seven local contracts ship next to this file:
`PLAYBOOK_MOBILE_IOS_SWIFT.md` (architecture, package graph, UDF, navigation,
platform), `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_FRONTEND.md`, `PLAYBOOK_URLS.md`, `PLAYBOOK_AUDIT.md` and
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

- Errors are values (`Outcome<T>`), never thrown for an expected failure; the
  codes live in `Kernel`.
- The domain converts a thrown `AppError` into `Outcome.failure`; an unknown
  error maps to `.unknown`. Cancellation is rethrown, never swallowed.
- Concurrency is Swift 6 mode with complete strict concurrency: models are
  `@MainActor`, ports and value types are `Sendable`.
- The state is immutable; the note id is a `NoteID` value type, not a `String`
  in the domain.
- UI strings are Swift string literals in the view (the template ships no
  localization catalog); a user-visible literal in a model or domain type is a
  review finding.
- This project was instantiated from a template: the three substitution
  tokens documented in `scripts/rename.sh` were replaced at instantiation
  time. If one survives verbatim outside the frozen fixtures, it is a bug,
  not a template artifact.
