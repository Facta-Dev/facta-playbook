# MEMORY — `<app>` desktop template

Durable facts about this repository. Facts that survive a session; not a
scratchpad, not a changelog. Update it when the reality changes, in the same
commit as the change that causes it.

## Product

A cross-platform **desktop client** for a SaaS backend: Qt 6 Widgets, offline
first, one process. The backend is reached through a generated C++ SDK; the
client keeps a local SQLite cache so the app stays usable when the network is
not. No user data leaves the machine except through the API.

## Layer map

One direction only, enforced by CMake target links (link time) plus
`desktop_assert_no_forbidden_include()` (configure time).

| Layer | Path | Owns |
|---|---|---|
| shared kernel | `src/shared-kernel/` | `Result<T>`, ids, time, logging primitives — QtCore value types only |
| contracts | `src/contracts/` | wire types shared with the SDK and the server |
| platform | `src/platform/` | the **only** OS-specific code: `#ifdef Q_OS_*`, keystore, app paths, single instance |
| infrastructure | `src/infrastructure/` | SQLite store, HTTP client, filesystem — adapters |
| features | `src/features/<slice>/` | one vertical slice per feature |
| ui-kit | `src/ui-kit/` | reusable widgets, theme, QSS, design tokens |
| app | `src/app/` | composition root: wiring only, no business logic |
| sdk | `packages/sdk-cpp/` | generated client, regenerated, never hand-edited |

## Slice model

A slice is vertical and self-contained:

```
src/features/<slice>/
  domain/         entities, value objects, domain rules — no GUI, no IO
  application/    use cases, ports (consumer-defined interfaces)
  adapters/       port implementations (storage, SDK, filesystem)
  presentation/   view model (QObject) + item model (QAbstractItemModel)
```

Rules of thumb that never change: the domain knows nothing about Qt beyond
value types; the application knows nothing about Widgets; the presentation layer
knows nothing about the SDK or the storage. Views talk to view models, view
models talk to use cases. Slices never import each other's internals.

## Stack

- **Qt 6.10** (Widgets, Network, Sql, Test) — no WebEngine, no QML by default.
- **C++20**, CMake ≥ 3.21 with **presets** (`dev` = Debug + tests, `release`),
  Ninja, `clang-format` + `clang-tidy`.
- SQLite via `QtSql` with **WAL** and `synchronous=NORMAL`.
- Version in `cmake/versioning.cmake` as `DESKTOP_VERSION` (MAJOR.MINOR.PATCH).

## Headless test policy

Every GUI test runs with `QT_QPA_PLATFORM=offscreen`. `desktop_add_test()`
(`cmake/qt.cmake`) sets the environment for every registered suite, so a suite
cannot accidentally need a display. Tests are hermetic: no network, no live
services, no real tenant data; ports are exercised against the fakes in
`tests/_shared/fakes/`. Waits are condition-based, never a fixed sleep.

## Storage schema version

**1.** The local schema version is `PRAGMA user_version`, and the migration
list in `src/infrastructure/src/storage/note_store.cpp` currently holds a
single entry (`{1, CREATE TABLE IF NOT EXISTS notes …}`). Migrations are
forward-only and append-only: never edit a shipped migration, add the next
number.

## Packaging targets

| OS | Targets |
|---|---|
| Windows | MSI (WiX), EXE installer, ZIP, MSIX |
| macOS | universal2 `.app` bundle, `.dmg`, `.pkg` — signed and notarized |
| Linux | AppImage, `.deb`, `.rpm`, Flatpak, tarball |

Nothing is published from an agent session: packaging jobs run in CI on a
signed tag.

## Gates

`make ci` = format check + build + tests. The full set: `cmake --preset dev`,
`cmake --build build`, `QT_QPA_PLATFORM=offscreen ctest --test-dir build`,
`make evals` (`-DEVALS=ON`, validates the `.claude` layer), `make verify`
(stages a renamed copy and re-runs every gate).

## Where the contracts live

- Layer and link rules: `CMakeLists.txt` (root) + `AGENTS.md`.
- Architecture decisions: `PLAYBOOK_DESKTOP_QT.md` (D1…D12).
- Test policy and gates: `PLAYBOOK_TESTS.md`.
- Secrets and threat model: `PLAYBOOK_SECURITY.md`.
- Agent kit governance: `PLAYBOOK_CLAUDE_CODE.md` and `.claude/schemas/**`.
