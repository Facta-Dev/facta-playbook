# `<app>` — agent instructions

Operating instructions for any coding agent working in this repository
(Claude Code, Codex, Cursor, Gemini …). `<app>` is a cross-platform Qt 6
desktop client template: hexagonal architecture (ports & adapters), vertical
slices, MVVM on **Qt Widgets**, C++20, CMake presets, a SQLite local cache and
a generated C++ SDK for the backend.

`CLAUDE.md` is a symlink to this file. Keep one source of truth.

## Hard rules

- **English only** in code, comments, UI copy, docs, test names and tool
  descriptions.
- **`make ci` green before you declare a change complete.** Plausibility is not
  correctness: read the output.
- **Never fabricate** paths, CMake targets, symbols, test results or commit
  hashes. Read the file or run the command.
- **Never weaken a gate** to make a build pass — no deleted failing test, no
  `NOLINT` on a layering rule, no relaxed `target_link_libraries`.
- **Touch only what the request needs.** No drive-by refactors.
- This repository was instantiated from a template: the three substitution
  tokens documented in `scripts/rename.sh` are placeholders until that script
  runs. One surviving verbatim outside the frozen `scripts/` and `evals/`
  directories after instantiation is a bug, not a template artifact.

## Layer rules — enforced at link time

Each layer is its own CMake target. `target_link_libraries` encodes the
dependency direction, so a violation fails when the target is linked, not in a
review.

| Layer | Target | May depend on | Must not depend on |
|---|---|---|---|
| shared kernel | `desktop_shared_kernel` | QtCore value types | contracts, platform, infrastructure, features |
| contracts | `desktop_contracts` | shared kernel | platform, infrastructure, features |
| platform | `desktop_platform` | shared kernel, Qt Core/Gui/Network | contracts, domain, application, features, ui-kit |
| infrastructure | `desktop_infrastructure` | application, platform, contracts | views, view models, ui-kit |
| slice domain | `desktop_domain` | shared kernel, contracts | QtGui/Widgets/Quick/Network/Sql, adapters, platform, sdk |
| slice application | `desktop_application` | domain | widgets, network, sql, adapters, platform |
| slice adapters | `desktop_notes_adapters` | application, infrastructure, sdk | views, view models |
| slice presentation | `desktop_notes_presentation` | application, QtCore | infrastructure, sdk, ui-kit, widgets |
| ui-kit | `desktop_ui_kit` | QtWidgets | domain, application, features, infrastructure, sdk, contracts |
| app (composition root) | `<app>-desktop` | presentation, infrastructure, platform, slice adapters | — |

Rules that follow from the table:

- `#ifdef Q_OS_*` appears **only** under `src/platform/`.
- The domain and the application carry **no GUI types**: no `QWidget`,
  `QMainWindow`, `QLabel`, `QApplication`, no QtGui/Widgets/Quick/Network/Sql.
  The domain may use QtCore *value* types (`QString`, `QDateTime`, `QUuid`).
- Views talk to view models; view models talk to use cases. A widget never
  reaches the SDK, the storage or the network.
- Ports are consumer-defined (`src/features/<slice>/application/.../…_port.h`),
  adapters implement them, and a slice never imports another slice's internals.
- Errors are values, not exceptions: `kernel::Result<T>` inside a layer,
  `QFuture<Result<T>>` across an async port. Nothing throws across a slot
  boundary.
- `desktop_assert_no_forbidden_include()` (defined in
  `src/shared-kernel/CMakeLists.txt`) fails the **configure** step on a
  forbidden `#include`; `tests/_platform/architecture/tst_architecture.cpp`
  asserts the same rules against the source tree.

## Widgets-first

Qt **Widgets** is the default UI technology (playbook §2.3): it rasterizes on
the CPU, so it needs no GPU and behaves predictably on old drivers. Qt Quick is
allowed only for an isolated, motion-heavy surface, restricted to the
software-safe subset (no `ShaderEffect`, minimal `layer.enabled`, no blur or
shadow effects) and always shipped with the Compatibility Mode fallback. A
Quick surface needs an ADR before the first QML file exists.

## No-GPU fallback

- Compatibility Mode forces CPU rendering and disables non-essential animation
  and transparency.
- The renderer is chosen **before** the first window exists: `main.cpp` reads
  `--software-rendering` from `argv` and sets `QT_QUICK_BACKEND=software` and
  `QT_OPENGL=software` before `QApplication` is constructed.
- Automatic safe mode: a crash or hang during startup is detected on the next
  launch and the app starts in Compatibility Mode.
- Widgets-only windows need no switch — which is exactly why Widgets is the
  default.
- CI runs the UI suites headless with `QT_QPA_PLATFORM=offscreen`.

## Playbooks — restrictions, loaded on demand

Seven local playbooks ship next to this file:

| File | Covers |
|---|---|
| `PLAYBOOK_DESKTOP_QT.md` | Platform model, layer architecture, MVVM, storage, packaging, performance budgets |
| `PLAYBOOK_TESTS.md` | Test pyramid, environments, gates, audit evidence |
| `PLAYBOOK_SECURITY.md` | Threat model, secrets, keystore, TLS, supply chain |
| `PLAYBOOK_URLS.md` | Route naming, error envelope, realtime topics |
| `PLAYBOOK_AUDIT.md` | Self-hosting an audit program (SOC 2 / ISO 27001 / 9001) |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code kit, TDD loop, multi-session governance |
| `PLAYBOOK_FRONTEND.md` | UI copy, accessibility, shared design tokens |

They are **restrictions, not suggestions**: a change that contradicts one is
wrong until the playbook changes in the same pull request. They load **on
demand only**: adopt one by adding a rule under `.claude/rules/` whose
`paths:` frontmatter names the files it guards and whose body names the
playbook — then read that playbook before editing those files. **Never
`@`-import them** — the always-on set (this file plus the path-less rules)
must stay inside the harness budget, and `scripts/instruction-budget.sh`
enforces it: 40,000 chars per file, 120,000 chars for the whole always-on set.

## Gates

```sh
cmake --preset dev                                  # configure (Debug, tests on)
cmake --build build -j "$(nproc)"                   # compile every target
QT_QPA_PLATFORM=offscreen ctest --test-dir build --output-on-failure
make evals                                          # -DEVALS=ON: desktop_evals_contracts
make ci                                             # format check + build + tests
bash scripts/verify-template.sh                     # template factory: stages a
                                                     # renamed copy, runs every gate
```

`make evals` and `bash scripts/verify-template.sh` are the template-factory
gates: they validate the `.claude` layer and the rename state of the tree.

## TDD loop

Red → green → refactor, one behaviour at a time:

1. Write the failing test first, in the layer that owns the behaviour.
   Suites are Qt Test binaries registered with `desktop_add_test()`; ports are
   exercised against the fakes in `tests/_shared/fakes/`, never against a real
   adapter.
2. Watch it fail for the right reason (missing implementation, not a typo).
3. Implement the minimum that makes it pass.
4. Refactor with the whole suite green — `ctest` over the full build tree, not
   only the new test.

Test rules: headless (`QT_QPA_PLATFORM=offscreen`), hermetic (no network, no
live services, no real tenant data), waits on observable conditions instead of
fixed sleeps.

## Who edits what

| Area | Owner |
|---|---|
| `docs/plans/`, specs | `planner`, `architect` |
| `src/features/<slice>/**` | implementers (`build-resolver`, `tdd-guide`) |
| `tests/**`, `src/features/*/tests/**` | `test-author` |
| `src/ui-kit/**`, `src/app/**` | `client` division (`widgets-ui-reviewer`) |
| `src/platform/**` | `platform-boundary-reviewer` (security) |
| `packaging/**`, installers, desktop files | `packaging-reviewer` |
| `cmake/**`, `CMakePresets.json`, `packages/sdk-cpp/**` | shared — integrator role |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**` | human only; an agent writes a proposal under `.claude/proposals/` |

Multi-session rules: one worktree per (slice, spec); never edit a slice
another session owns — message its owner instead; message dependents before
changing a published contract; never force-push a shared branch.

## Never

- **No QML** (or any new Qt Quick surface) without an ADR.
- **No OS headers** (`<windows.h>`, Cocoa, X11/D-Bus) outside `src/platform/`.
- **No GUI types** in the domain or the application layer — no widget, no
  `QApplication`, no network or SQL headers.
- **No secrets** in the repository, the logs, `QSettings` or a crash dump:
  tokens live in the OS keystore, `.env` is gitignored, `.env.example` holds
  placeholders only.
- No third-party dependency without an ADR, a license check and an SBOM entry.
- No blocking call on the GUI thread (`waitForReadyRead`, `QThread::sleep`,
  `QFuture::result()`); the GUI thread runs views, view models and trivial
  orchestration only.
- No `@`-import of a playbook.
