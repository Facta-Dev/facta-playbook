# 12 — Testing & CI

## The suites

The template registers **eight** test executables: six cross-layer suites under
`tests/`, and two colocated with the code they test.

| Suite | Source | What it covers |
| --- | --- | --- |
| `desktop_shared_kernel_tests` | `tests/shared-kernel/tst_shared_kernel.cpp` | `Result`, `ErrorCode` and its wire names, `StrongId`, `Timestamp` |
| `desktop_contracts_tests` | `tests/contracts/tst_contracts.cpp` | The DTOs, the error envelope, the policy schema, the realtime envelope |
| `desktop_notes_tests` | `src/features/notes/tests/tst_notes.cpp` | The `Note` domain: the lengths, the markup rejection, the trimming |
| `desktop_notes_slice_tests` | `tests/features/notes/tst_notes_slice.cpp` | The use cases against a fake repository |
| `desktop_notes_view_model_tests` | `tests/features/notes/presentation/tst_notes_view_model.cpp` | The view model: the states, the commands, the signals |
| `desktop_notes_window_tests` | `tests/app/tst_notes_window.cpp` | The window against the view model |
| `desktop_runtime_tests` | `src/infrastructure/tests/tst_runtime.cpp` | The store, the HTTP transport, the logger and its redaction |
| `desktop_architecture_tests` | `tests/_platform/architecture/tst_architecture.cpp` | The three layering rules, read from the source tree |

A ninth test, `desktop_evals_contracts`, runs with `-DEVALS=ON` (`make evals`).

## The helper and the fake

Every suite is registered with one helper, `desktop_add_test()`, defined in
`cmake/qt.cmake`. It links `Qt6::Test` plus the layer libraries the suite
needs, and **runs the test headless** by setting `QT_QPA_PLATFORM=offscreen`
on it — so a widget test runs in a container with no display server.

`Qt6::Test` is only required when `ENABLE_TESTS` is on, and its absence is a
**configure error**, not a green build with zero tests.

A port is never tested against a real implementation. The shared fake lives in
`tests/_shared/fakes/fake_note_repository.h` and is added to the include path of
the three suites that need it. The fake is the seam that makes the layers
testable in isolation: the use cases, the view model and the window are tested
without a database, and the window without a real view model.

## The architecture suite

The architecture guard reads the **source tree**, not a hand-written list, so a
new file is checked too. It enforces three rules:

1. The domain of a slice never includes a QtWidgets header.
2. An OS-specific header is only included inside `src/platform`.
3. The application layer of a slice never includes an infrastructure header.

The QtWidgets include directory is passed in by the build, so the check works
against the real headers of the Qt in use. This suite is the third guard of the
layering rule ([04](04-hexagonal-layers-and-mvvm.md)); the other two are the
configure-time include check and the link-time rule.

## The commands

```sh
make test           # build with ENABLE_TESTS=ON and run ctest
make evals          # configure build-evals with -DEVALS=ON and run the harness
make lint           # clang-format --dry-run -Werror
make tidy           # clang-tidy -p build
make ci             # lint + verify (the gate)
```

`make lint` and `make tidy` **skip with a printed notice** when `clang-format` or
`clang-tidy` is not installed. A skip is not a pass: read the output.

## The template verification

```sh
bash scripts/verify-template.sh
```

The script stages a throwaway copy of the kit, renames it with
`scripts/rename.sh` (the example values `demo` and `demoorg`), and runs the
gates on the copy. **The working tree is never mutated.** The stages:

| Stage | What it proves |
| --- | --- |
| `lint-agents`, `lint-skills`, `lint-commands` | The `.claude` agents, skills and commands match their schema |
| `instruction-budget` | The always-on instruction set fits the budget (40 000 characters per file, 120 000 in total) |
| `leak-guard` | No placeholder survived the rename (`--placeholders-only`) |
| `shell-syntax` | Every shell script in the kit parses |
| `cmake-configure`, `cmake-build` | The renamed tree configures and builds |
| `ctest` | The whole suite passes headless |
| `evals` | The `.claude` contract harness passes |

> **Note.** The three linter stages read the shipped `.claude/agents/`,
> `.claude/skills/` and `.claude/commands/` trees and check them against the
> schema documents in `.claude/schemas/`. All ten stages pass on the template
> as shipped.

## The evals harness

`evals/contracts_test.cpp` validates the `.claude` agents, skills and commands
against the schema documents in `.claude/schemas/`, and sweeps the tree for
placeholders that survived the rename. The sweep skips `.git/`, `scripts/`,
`evals/`, `node_modules/`, `bin/`, `target/`, `CMakeFiles/` and every
directory starting with `build`, and it detects the rename state from the
CMake `project()` name.

## The CI pipeline

The template ships no `.github/` workflow; the pipeline is the make targets:

| Job | Command | On |
| --- | --- | --- |
| Build and test | `make ci` | Linux, Windows, macOS |
| Static analysis | `make tidy` | Linux |
| Format | `make lint` | Linux |
| Evals | `make evals` | Linux |
| Template verification | `bash scripts/verify-template.sh` | Linux |
| Package | — | **Not implemented** ([11](11-packaging-and-updates.md)) |

The three-OS matrix is not optional: the keystore and the single-instance guard
differ per OS, so a green Linux build says nothing about the Windows stub.

## Writing a test

| Rule | Why |
| --- | --- |
| One behaviour per test function; name the behaviour | A failing test names one thing |
| Use the fake, never the real adapter | The test must not need a disk or network |
| Assert the value, not the log | A test that reads the log breaks when the log changes |
| Use `QTemporaryDir` for a file | Never the user's database |
| No sleep, no wall-clock dependency | A flaky test is a deleted test |

A test that needs the network is an integration test, outside `make test`.

## Do / don't

| Do | Don't |
| --- | --- |
| Register every suite with `desktop_add_test` | Add a `QTEST_MAIN` without the helper |
| Test the port against a fake | Test the use case against SQLite |
| Run the suite headless | Require an X server in CI |
| Run the suite on all three operating systems | Trust a Linux-only green build |

## See also

- [Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md) — the three guards.
- [Playbooks](13-playbooks.md) — how to add a suite.
- [`tests/CMakeLists.txt`](../tests/CMakeLists.txt) ·
  [`cmake/qt.cmake`](../cmake/qt.cmake)
