# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Aider, Gemini …). `<app>` is a native Android
app skeleton: Kotlin 2.4 + Jetpack Compose, Gradle 9.6 with AGP 9.5, one
Gradle module per architectural layer, Navigation 3 typed `NavKey` routes, a
UDF `notes` slice, and manual dependency injection in `:app`.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, resources, docs, test names
  and tool descriptions. No Spanish anywhere in the repo.
- **`make ci` green before you declare a change complete.** Plausibility is
  not correctness: read the output.
- **Never fabricate** paths, task names, API names, versions, test results or
  commit hashes. Read the file or run the command.
- **Touch only what the request needs.** No drive-by refactors.
- **The template ships literal placeholders.** `<app>`, `<APP>` and `<scope>`
  are not valid Kotlin package segments, so the template root does not build.
  Run `bash scripts/rename.sh <app> <scope>` on a copy before the first build.

## Never

- **No `android.*` import in a pure JVM module** (`:core:kernel`,
  `:feature:notes:contracts`, `:feature:notes:domain`, `:evals`). The SDK is
  not on the classpath, so it does not compile. `androidx.*` is allowed only
  where the artifact publishes a JVM variant — `:feature:notes:contracts` uses
  the multiplatform Navigation3 runtime, and it stays a pure JVM module.
  `scripts/check-architecture.sh` names the rule before the compiler does.
- **No `:feature:notes:ui → :feature:notes:data` dependency.** The UI never
  picks an implementation; `:app` does.
- **No string routes.** Routes are `@Serializable` `NavKey` objects in
  `:feature:notes:contracts`; a screen is entered by the typed key.
- **No `Context`, Activity, `View` or composable inside a ViewModel or a
  domain class.** A ViewModel that holds a UI reference leaks it.
- **No new Android permission without a review.** The manifest declares none;
  a permission needs a register entry, a justification, a test and a security
  review ([PLAYBOOK_SECURITY.md](PLAYBOOK_SECURITY.md)).
- **No secrets** in `BuildConfig`, `strings.xml`, resources, logs or
  telemetry. The release keystore and its passwords stay outside the tree.
- **No DI framework in the template.** Manual DI in `:app` is the contract
  ([ADR 0003](docs/adr/0003-manual-di-in-the-app-module.md)).
- **No business logic in a composable.** It recomposes and it is not covered
  by a unit test.
- **No cross-slice imports.** A shape shared by two slices moves to
  `:core:kernel` (pure types) or the slice's `contracts` module.

## Layout

```
<app>/
├── app/                          # :app — the composition root and the only Application
│   └── src/main/
│       ├── kotlin/dev/<scope>/<app>/MainActivity.kt
│       ├── AndroidManifest.xml
│       └── res/                  # strings.xml · themes.xml
├── core/
│   ├── kernel/                   # :core:kernel — pure JVM: Outcome, AppError, NoteId
│   └── ui-core/                  # :core:ui-core — Android library: UiStatus
├── feature/
│   └── notes/                    # the example slice — replace with your domain
│       ├── contracts/            # :feature:notes:contracts — pure JVM: @Serializable NavKey routes
│       ├── domain/               # :feature:notes:domain — pure JVM: entity, port, use case
│       ├── data/                 # :feature:notes:data — the in-memory adapter
│       └── ui/                   # :feature:notes:ui — Compose screens + ViewModel
│           └── res/values/strings.xml   # the slice's own user-visible strings
├── evals/                        # :evals — pure JVM: the .claude contract tests + live tier (run-live.sh)
├── gradle/
│   ├── libs.versions.toml        # the version catalog — the single source of versions
│   └── wrapper/                  # the pinned Gradle wrapper
├── gradlew · gradlew.bat
├── scripts/                      # rename · verify-template · check-architecture
│                                # · lint-* · leak-guard · instruction-budget · jdk
│                                # · install-claude-runtime · hooks/guard-bash.sh
├── .claude/                      # agents · skills · commands · schemas · settings · manifest
├── docs/                         # 00–14 · adr/ · admin-guide · data-flows · dependency-register
├── PLAYBOOK_*.md                # seven local contracts — read on demand
├── AGENTS.md                    # this file; CLAUDE.md is a symlink to it
├── MEMORY.md                    # durable premise: modules, routes, UDF, stack
├── Makefile                     # ci · test · build · evals · lint · verify
└── settings.gradle.kts · build.gradle.kts · gradle.properties
```

## The layers and their rules

| Module | Kind | May depend on | Must not |
| --- | --- | --- | --- |
| `:core:kernel` | Pure JVM | the Kotlin stdlib | anything internal |
| `:core:ui-core` | Android library | `:core:kernel` | a feature module |
| `:feature:notes:contracts` | Pure JVM | Navigation3 runtime, serialization | `android.*`, a data or UI module |
| `:feature:notes:domain` | Pure JVM | `:core:kernel` | `android.*`, adapters, UI |
| `:feature:notes:data` | Android library | `:feature:notes:domain` | `:feature:notes:ui` |
| `:feature:notes:ui` | Android library | domain, contracts, `:core:ui-core`, Compose | `:feature:notes:data` |
| `:app` | Android application | every feature module | business rules |
| `:evals` | Pure JVM | nothing app-side (test deps only) | the app modules |

Three invariants make the architecture hold, and each one has a machine check:

1. **The module graph is the architecture.** The dependency edge is declared
   in the build file; a forbidden edge is a build or gate failure, not a code
   review preference. `scripts/check-architecture.sh` checks the edges the
   compiler cannot.
2. **State flows down, events flow up, effects leave once.**
   `NotesState` is an immutable value, the screen sends `NotesEvent`, and a
   one-shot `NotesEffect` leaves through a `Channel(BUFFERED)` with a single
   consumer in `:app`.
3. **Routes are typed keys, not strings.** `NotesRoute` is a
   `@Serializable sealed interface : NavKey` in a pure JVM module; `:app`
   registers the polymorphic serializers and maps each key to a screen.

## Gates

```sh
make ci                          # check-architecture + test + testDebugUnitTest + lintDebug + assembleDebug
./gradlew test                   # pure JVM tests: kernel, contracts, domain, evals
./gradlew testDebugUnitTest      # Android unit tests: ui, data, ui-core
./gradlew :app:lintDebug         # Android Lint on the debug variant
./gradlew :app:assembleDebug     # the compile proof of the whole tree
./gradlew :evals:test            # the .claude contract tests, named explicitly
bash scripts/check-architecture.sh
bash scripts/verify-template.sh  # stages a renamed copy and runs every gate on it
```

The first build needs a network (Gradle resolves the wrapper and the
dependencies) and an Android SDK with platform 37 (`ANDROID_HOME`). A full JDK
17+ is required — a JRE alone cannot run the compiler, and `scripts/jdk.sh`
pins a JDK that has both `java` and `javac`.

## The TDD loop

Spec → plan → red → green → refactor:

1. `/spec` writes `docs/specs/<slug>.md` (behaviour, acceptance criteria,
   non-goals, route and permission surface, open questions); `/plan` writes
   `docs/plans/<slug>.md` (problem, constraints, design, files, risks, tests).
2. `/add-slice` or `/add-module` scaffolds the shape; `/add-route` adds a
   typed `NavKey` and its screen entry; `/add-port` adds a domain port and its
   adapter in `:app`.
3. Write the failing test first (`/tdd-guide`, `test-author`): a use-case test
   against a fake port, or a ViewModel test with a `TestDispatcher`. Watch it
   fail for the right reason — a missing symbol, not a typo.
4. Implement the minimum that makes it pass, then refactor with the suite as
   the safety net.
5. `/review` for the pre-landing checklist; `make ci` before declaring done.

Every new screen ships with a ViewModel test covering `Ready`, `Empty` and
one failure status. Every new route ships with a serializer test: the key
round-trips through the polymorphic module. Every new port ships with a fake
and a use-case test.

## Who edits what

| Area | Owner |
| --- | --- |
| `docs/specs/`, `docs/plans/`, `docs/adr/` | `planner`, `architect` |
| test sources in every module | `test-author` |
| `feature/**`, `core/**`, `app/src/main/**` | implementers (`build-resolver`, `tdd-guide`) |
| `settings.gradle.kts`, the catalog, the root build files, `scripts/**` | shared — integrator role |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**` | human only; an agent writes a proposal under `.claude/proposals/` |

## Multi-session rules

- One worktree per (module, spec); name sessions `<module>:<spec-id>`.
- Do not edit a module claimed by another session — message its owner instead.
- Changing a published contract (a route key, a port, the `UiStatus` set)? 
  Message the dependents and tell the integrator.
- Never force-push a shared branch.

## Native `.claude` layer

`.claude/agents/` ships the roster across four divisions (`core`, `client`,
`security`, `testing`). `.claude/skills/` + `.claude/commands/` ship reusable
workflows; every command is a thin shim delegating 1:1 to the skill of the
same slug. New slugs are discovered by walking the tree — no registry edit.
`scripts/lint-agents.sh`, `lint-skills.sh` and `lint-commands.sh` validate the
tree against `.claude/schemas/`; `:evals:test` asserts the same contract from
the JVM side.

Skills: **`/add-slice`** (scaffold a slice across four modules),
**`/add-module`** (a new Gradle module with its layer rules),
**`/add-route`** (typed `NavKey` + serializer registration + screen entry),
**`/add-port`** (domain port + adapter + fake + test), **`/review`**,
**`/spec`**, **`/plan`**, **`/learn`**, **`/ship`** (release PR; user-invoked
only).

## Playbooks

Seven local contracts ship next to this file:
`PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` (architecture, Gradle graph, UDF,
Navigation 3, platform), `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`,
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

- Errors are values (`Outcome<T>`), never exceptions for an expected failure;
  the codes live in `:core:kernel`.
- `CancellationException` is always rethrown; swallowing it breaks structured
  concurrency.
- The dispatcher is injected into the ViewModel; a test never waits on a real
  thread.
- The state is immutable and `@Immutable`; the note id is a `NoteId` value
  class, not a `String` in the domain.
- UI strings live in `strings.xml` — the module's own
  `src/main/res/values/strings.xml`, read with `stringResource(R.string…)`.
  No user-visible literal in Kotlin. A feature module owns its strings: it
  cannot read the `:app` resources.
- Every version comes from `gradle/libs.versions.toml`; an inline version is
  a review finding.
- This project was instantiated from a template: the three substitution
  tokens documented in `scripts/rename.sh` were replaced at instantiation
  time. If one survives verbatim outside the frozen fixtures, it is a bug,
  not a template artifact.
