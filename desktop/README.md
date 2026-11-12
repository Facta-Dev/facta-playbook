# `<app>` — Qt 6 desktop client template

A hexagonal, vertically-sliced **Qt 6 / C++20** desktop template: Qt Widgets
(never QML by default), MVVM with `QObject` view models, a SQLite local cache
in WAL mode, a generated C++ SDK and a CMake link-time dependency law. Every
`<app>`, `<APP>` and `<scope>` token is a **literal placeholder** until you
rename; the project and org names are placeholders in `src/app/main.cpp`. See
[How to apply](#how-to-apply).

## Layout

```
<app>/
├── src/
│   ├── shared-kernel/       Result<T> · ids · time · logging primitives (QtCore value types only)
│   ├── contracts/           wire types shared with the SDK and the server
│   ├── platform/            the only OS-specific code: #ifdef Q_OS_* · keystore · app paths
│   ├── infrastructure/      adapters: SQLite store · HTTP client · telemetry · filesystem
│   ├── features/            vertical slices, one per feature (never import each other)
│   │   └── notes/           example slice: domain · application · adapters · presentation · tests
│   ├── ui-kit/              reusable widgets, theme, QSS, design tokens
│   └── app/                 composition root: main.cpp wires store → use cases → view model → window
├── packages/sdk-cpp/        generated client — regenerated, never hand-edited
├── tests/                   shared suites (app · contracts · shared-kernel · features) + _shared/fakes + _platform/architecture
├── evals/                   offline contract tests for the .claude layer + live tier (run-live.sh)
├── cmake/                   qt.cmake · hardening.cmake · versioning.cmake
├── .claude/                 agents · skills · commands · schemas · settings · manifest
├── scripts/                 rename.sh · verify-template.sh · lint-*.sh · leak-guard.sh · instruction-budget.sh · install-claude-runtime.sh
├── PLAYBOOK_*.md            seven contracts (see Working with agents)
├── AGENTS.md                agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md                premise, layer map, slices, stack, invariants
├── CMakeLists.txt           layer targets + the link-time dependency law
├── CMakePresets.json        dev (Debug, tests on) · release
├── Makefile                 ci · configure · build · test · lint · tidy · evals · verify
├── .env.example             placeholder values only — never real secrets
└── README.md                this file, post-rename
```

## Layer rules

One direction only, enforced by `target_link_libraries` (link time) and `desktop_assert_no_forbidden_include()` (configure time).

| Layer | Target | May depend on | Forbidden |
|---|---|---|---|
| shared kernel | `desktop_shared_kernel` | QtCore value types | contracts, platform, features, ui-kit |
| contracts | `desktop_contracts` | shared kernel | platform, infrastructure, features |
| platform | `desktop_platform` | shared kernel, Qt Core/Gui/Network | contracts, domain, application, features |
| infrastructure | `desktop_infrastructure` | application, platform, contracts | views, view models, ui-kit |
| slice domain | `desktop_domain` | shared kernel, contracts | QtGui/Widgets/Quick/Network/Sql, adapters |
| slice application | `desktop_application` | domain | widgets, network, sql, adapters, platform |
| slice adapters | `desktop_notes_adapters` | application, infrastructure, sdk | views, view models |
| slice presentation | `desktop_notes_presentation` | application, QtCore | infrastructure, sdk, ui-kit, widgets |
| ui-kit | `desktop_ui_kit` | QtWidgets | domain, application, features, infrastructure |
| app | `<app>-desktop` | presentation, infrastructure, platform, adapters | — |

## Prerequisites

- Qt **6.10** — `Core`, `Gui`, `Widgets`, `Network`, `Sql`, `Test`; no WebEngine.
- CMake ≥ 3.21 with **presets** and **Ninja**, a C++20 compiler (GCC 12+,
  Clang 16+, MSVC 2022), plus `clang-format` and `clang-tidy` for `make lint`.
- No display needed: every test runs headless (`QT_QPA_PLATFORM=offscreen`).

## The gate

```sh
make ci                  # clang-format check + configure + build + tests
make test                # QT_QPA_PLATFORM=offscreen ctest
make evals               # -DEVALS=ON: validates the .claude layer and the rename state
make verify              # stages a renamed copy and re-runs every gate on it
```

The architecture law is executable: a forbidden `#include` fails **configure**,
a forbidden dependency fails **link**, and `tests/_platform/architecture/` fails **test**.

## Working with agents

`AGENTS.md` (symlinked as `CLAUDE.md`) is the operating instruction file: hard
rules, layer table, gates, the TDD loop, who edits what, and the playbook
index. `MEMORY.md` holds the premise — layers, slices, stack, invariants — so
a fresh session does not have to rediscover it.

Seven contracts ship next to this file:

| File | Covers |
|---|---|
| `PLAYBOOK_DESKTOP_QT.md` | Platform model, layer architecture, MVVM, storage, packaging, budgets |
| `PLAYBOOK_TESTS.md` | Test pyramid, environments, gates, SOC 2 / ISO 27001 evidence |
| `PLAYBOOK_SECURITY.md` | Threat model, secrets, keystore, TLS, supply chain |
| `PLAYBOOK_URLS.md` | Route naming, namespaces, error envelope |
| `PLAYBOOK_AUDIT.md` | Running an audit program (SOC 2 / ISO 27001 / ISO 9001) |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code kit, TDD loop, multi-session governance |
| `PLAYBOOK_FRONTEND.md` | UI copy, accessibility, shared design tokens |

The `.claude/` layer ships **11 agents** across four divisions (`core`,
`client`, `security`, `testing`), **9 skills** with 1:1 slash commands
(`/add-slice`, `/add-view`, `/add-linux-packaging`, `/platform-port`, `/plan`,
`/spec`, `/review`, `/ship`, `/learn`), schema-driven linters in `scripts/`, a
`PreToolUse` Bash guard and a `manifest.json` target matrix for other runtimes.

```sh
bash scripts/lint-agents.sh
ALLOW_PLACEHOLDERS=1 bash scripts/lint-skills.sh     # pre-rename template
ALLOW_PLACEHOLDERS=1 bash scripts/lint-commands.sh
bash scripts/instruction-budget.sh
```

`ALLOW_PLACEHOLDERS=1` is the escape hatch for the **pre-rename** template, where the placeholder tokens are intentional; after the rename the linters run strict.

## How to apply

1. Copy this template to your repo root.
2. `bash scripts/rename.sh <app> <scope>` — the `<scope> → <APP> → <app>`
   substitutions, in safe order, skipping `scripts/` and `evals/fixtures/`.
3. `cmake --preset dev` — the development preset (Debug, tests on, `build/`).
4. `make ci` — green.
5. Replace the `notes` example slice with your domain.

## Installing the agent runtime

`scripts/install-claude-runtime.sh` copies `.claude/{agents,skills,commands}`
into the targets of `.claude/manifest.json` (Claude Code always-on; codex,
opencode and zed adapters) and writes `.claude/install-state.json`.

## ponytail notes

Deliberate simplifications are marked with a `ponytail:` comment naming the
ceiling and the upgrade path. The `notes` slice is a **worked example**, not
a feature to keep: delete it once your own slice exists.
