# `<app>` — native Android skeleton

> Companion repo:
> [beautifulandroid](https://github.com/Facta-Dev/beautifulandroid) — the Agent
> Skills and linter config for this stack.

A native Android template: Kotlin 2.4 + Jetpack Compose, Gradle 9.6 with AGP
9.5, **one Gradle module per architectural layer**, Navigation 3 typed
`NavKey` routes, a unidirectional-data-flow `notes` slice, and manual
dependency injection in `:app`. Every `<app>`, `<APP>` and `<scope>` token is
a **literal placeholder** — the package path is invalid until you rename, so
the template root does not build. See [How to apply](#how-to-apply).

## Layout

```
<app>/
├── app/                  # :app — the composition root, the manifest, NavDisplay
├── core/
│   ├── kernel/           # :core:kernel — pure JVM: Outcome, AppError, NoteId
│   └── ui-core/          # :core:ui-core — UiStatus, the closed status set
├── feature/
│   └── notes/            # the example slice — replace with your domain
│       ├── contracts/    #   pure JVM: @Serializable NavKey routes
│       ├── domain/       #   pure JVM: entity, port, use case
│       ├── data/         #   the in-memory adapter
│       └── ui/           #   Compose screens + ViewModel
├── evals/                # :evals — the .claude contract tests (pure JVM) + live tier (run-live.sh)
├── gradle/               # libs.versions.toml · wrapper/
├── scripts/              # rename · verify-template · check-architecture · linters · jdk
├── .claude/              # 11 agents · 9 skills/commands · schemas · hooks
├── docs/                 # 00–14 · adr/ · admin-guide · data-flows · dependency-register
├── PLAYBOOK_*.md        # seven local contracts — read on demand
├── AGENTS.md            # agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md            # durable premise: modules, routes, UDF, stack
└── Makefile             # ci · test · build · evals · lint · verify
```

## The module model

The dependency edge is declared in the build file, which makes the
architecture a build fact. `ui` never depends on `data`; `:app` chooses the
adapter. `kernel`, `contracts`, `domain` and `evals` are pure JVM modules —
an `android.*` import there does not compile. `scripts/check-architecture.sh`
gates the edges the compiler cannot catch.

`NotesRoute` is a `@Serializable sealed interface : NavKey`; `:app` registers
the polymorphic serializers and maps each key to a screen through
`NavDisplay`. `NotesState` flows down, `NotesEvent` flows up, and
`NotesEffect` leaves once through a `Channel(BUFFERED)`.

## The gate

```sh
make ci                          # check-architecture + test + testDebugUnitTest + lintDebug + assembleDebug
./gradlew test testDebugUnitTest
./gradlew :app:lintDebug
./gradlew :app:assembleDebug
./gradlew :evals:test
bash scripts/check-architecture.sh
```

Requirements: a full JDK 17+ (`javac`, not a JRE alone) and an Android SDK
with platform 37, resolved from `ANDROID_HOME`. The first build needs a
network. `scripts/jdk.sh` pins a JDK that has both `java` and `javac`.

## Working with agents

`AGENTS.md` (symlinked as `CLAUDE.md`) is the operating instruction file:
hard rules, the never-list, the module table, the gates, the spec → test →
code workflow and the playbook index. `MEMORY.md` holds the premise — the
module graph, the routes, the UDF contract, the status set, the stack.

Seven local contracts ship next to this file and load **on demand** through
path-scoped rules, never as `@`-imports:
`PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` (architecture, Gradle, UDF, Navigation 3,
platform features), `PLAYBOOK_TESTS.md` (tiers, fixtures, gates, audit
evidence), `PLAYBOOK_SECURITY.md` (threat model, permissions, keystore,
secrets), `PLAYBOOK_FRONTEND.md` (the cross-client UI contract),
`PLAYBOOK_URLS.md` (route and API naming, the error envelope),
`PLAYBOOK_AUDIT.md` (SOC 2 / ISO 27001 / ISO 9001 programs) and
`PLAYBOOK_CLAUDE_CODE.md` (the kit, SDD + TDD loop, governance).

The `.claude/` layer ships 11 agents across four divisions (`core`, `client`,
`security`, `testing`), 9 skills with 1:1 slash commands (`/add-slice`,
`/add-module`, `/add-route`, `/add-port`, `/review`, `/spec`, `/plan`,
`/learn`, `/ship`), schema-driven linters in `scripts/`, a `PreToolUse` Bash
guard, and a `manifest.json` target matrix for installing the kit into other
agent runtimes.

## How to apply

1. Copy this directory to your repo root.
2. `bash scripts/rename.sh <app> <scope>` — substitutes `<scope> → <APP> →
   <app>` in text files and renames the `dev/<scope>/<app>` package
   directories deepest-first.
3. `./gradlew :app:assembleDebug` — the first run resolves the wrapper and
   the dependencies; keep build output out of the template.
4. `make ci` — green.
5. Replace the `notes` example slice with your domain.

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
