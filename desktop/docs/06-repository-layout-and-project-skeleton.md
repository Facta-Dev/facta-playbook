# 06 — Repository Layout & Project Skeleton

## The tree

```
desktop/
├── CMakeLists.txt              ← the project, the link rules, the layer targets
├── CMakePresets.json           ← dev (Debug, tests on) and release (Release, tests off)
├── Makefile                    ← the commands: build, test, lint, tidy, evals, ci
├── cmake/                      ← qt.cmake, hardening.cmake, versioning.cmake
├── src/
│   ├── app/                    ← the composition root and the windows
│   ├── shared-kernel/          ← Result, ErrorCode, StrongId, Timestamp
│   ├── contracts/              ← DTOs, the error envelope, the policy schema
│   ├── platform/               ← the only OS-aware code
│   ├── infrastructure/         ← storage, HTTP transport, telemetry
│   ├── ui-kit/                 ← the shared widgets and the theme
│   └── features/<name>/        ← the slices: domain, application, adapters, presentation
├── packages/sdk-cpp/           ← the HTTP SDK
├── tests/                      ← the cross-layer suites
├── evals/                      ← the .claude contract harness
├── scripts/                    ← rename, verify-template, leak-guard, lint-*
├── docs/                       ← this set
└── .claude/                    ← the agent kit (manifest, schemas, agents, skills, commands)
```

The domain and the application layers live **inside the slice**, not above
it: `src/features/notes/domain/` and `src/features/notes/application/` build
the `desktop_domain` and `desktop_application` targets. There are no global
`src/domain/` or `src/application/` layers; what every slice shares lives in
`src/shared-kernel/` and `src/contracts/`
([05 — Vertical Slicing](05-vertical-slicing.md)).

## The targets

| Target | Kind | Depends on |
| --- | --- | --- |
| `desktop_shared_kernel` | static | Qt Core |
| `desktop_contracts` | static | kernel |
| `desktop_domain` | static | kernel, contracts |
| `desktop_application` | static | domain |
| `desktop_platform` | static | kernel |
| `desktop_infrastructure` | static | application, platform, contracts |
| `desktop_ui_kit` | static | — |
| `desktop_presentation` | static | application, slice view models, ui_kit |
| `desktop_sdk` | static | contracts, Qt Core, Qt Network |
| `desktop_notes_adapters` | static | application, infrastructure, sdk |
| `desktop_notes_presentation` | static | application |
| `<app>-desktop` | executable | presentation, infrastructure, platform, slice adapters |

The rules are in the root [`CMakeLists.txt`](../CMakeLists.txt) and in
[ADR 0002](adr/0002-one-static-library-per-layer.md); every layer also calls
`desktop_assert_no_forbidden_include(...)`.

## The commands

Every command below is a real `Makefile` target.

| Command | What it does |
| --- | --- |
| `make help` | Lists the targets |
| `make configure` | Configures the tree: `cmake --preset dev` the first time, plain `cmake -S . -B build` afterwards |
| `make build` | `configure` + `cmake --build build -j $(nproc)` |
| `make test` | Builds with `ENABLE_TESTS=ON` and runs `ctest --test-dir build --output-on-failure` |
| `make lint` | `clang-format --dry-run -Werror` over `src packages tests evals`; **skipped with a notice** when the tool is absent |
| `make tidy` | `clang-tidy -p build` over `src` and `packages`; **skipped with a notice** when the tool or `compile_commands.json` is absent |
| `make evals` | Configures `build-evals` with `-DEVALS=ON -DENABLE_TESTS=ON`, builds it and runs the harness |
| `make verify` | Alias of `make test` |
| `make ci` | `lint` + `verify`: the gate |

Variables: `PRESET`, `BUILD_DIR`, `CONFIG`, `JOBS`, `ENABLE_TESTS`.

```sh
make ci                      # the gate: format, build, test
make build PRESET=release    # the release tree
make evals                   # the .claude harness
```

## The build options

| Option | Default | Effect |
| --- | --- | --- |
| `ENABLE_TESTS` | `ON` in the `dev` preset, `OFF` in `release` | Builds the test targets and finds `Qt6::Test` |
| `EVALS` | `OFF` | Builds the `evals` harness |
| `DESKTOP_VERSION` | unset | The version injected as the `DESKTOP_VERSION` compile definition; must be `MAJOR.MINOR.PATCH` |

`cmake/versioning.cmake` validates the version string and refuses anything
that is not three numbers. `cmake/hardening.cmake` adds the warning flags
(`-Wall -Wextra -Wpedantic`, `/W4 /sdl /GS /guard:cf /permissive-` on
MSVC), position independent code, stack protection and, on Linux,
`-Wl,-z,relro,-z,now`. There is deliberately **no `-Werror`**: a new
compiler's warning must not break the build of a release.

## Starting a project from the template

```sh
cp -a <path-to>/desktop my-app
cd my-app
bash scripts/rename.sh my-app myorg
make ci
```

`scripts/rename.sh` substitutes the three placeholder tokens — the lowercase
app name, its uppercase form and the organisation scope — across the tree,
and rewrites the CMake project name, the organisation name and the target
name. **Never run it in the template root**: the template keeps the literal
tokens so that the kit stays a template.

`bash scripts/verify-template.sh` checks the result: it renames a
throwaway copy, builds it, runs the tests and the evals, and greps for
leftovers. The leak guard (`bash scripts/leak-guard.sh`) is the same idea
as a standalone command: it fails on a placeholder that survived the rename,
on a secret-shaped string, and on a file that grew past the instruction
budget (`scripts/instruction-budget.sh`: 40 000 characters per file, 120 000
in total).

## What to delete in your project

The `notes` slice is a worked example, not your feature. Delete
`src/features/notes/`, its two CMake targets, the tests that reference it and
the wiring in `main.cpp`, and start from an empty slice. Keeping it as a
second, unused slice is how templates rot.

## Conventions

| Thing | Convention |
| --- | --- |
| Namespace | `desktop::<slice>::<layer>` — `desktop::notes::domain::Note` |
| Include path | `desktop/<slice>/<layer>/<file>.h`, from the target's `include/` directory |
| Header guard | `#pragma once` |
| Formatting | `.clang-format` (4 spaces, 100 columns); analysis in `.clang-tidy` |
| Naming | Files in `snake_case`, types in `PascalCase`, functions in `camelCase` |
| Comments | English, and only where the code is not self-evident |
| Language | English only, in code, comments, UI copy, tests and docs |

## See also

- [Vertical Slicing](05-vertical-slicing.md) — what goes in `src/features/`.
- [Testing & CI](12-testing-and-ci.md) — the gates and the suites.
- [`CMakeLists.txt`](../CMakeLists.txt) · [`Makefile`](../Makefile)
