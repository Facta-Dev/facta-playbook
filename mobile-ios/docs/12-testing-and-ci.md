# Testing & CI

The gates are the contract. A change is finished when the gate set is green;
a change to the kit's own agent runtime is finished when the evals pass.

## 1. The test tiers

| Tier | Where it runs | Tooling | What it covers |
| --- | --- | --- | --- |
| **Kernel** | macOS host, `swift test` in `Packages/Core/Kernel` | Swift Testing | `Outcome`, `AppError`, `ErrorCode`, `NoteID` |
| **Domain** | macOS host, `swift test` in `Packages/Feature/Notes/Domain` | Swift Testing | the entity, the port, the use cases |
| **Model** | macOS host, `swift test` in `Packages/Feature/Notes/UI` | Swift Testing, `@MainActor` | `NotesModel`: the state, the events, the effects |
| **Architecture** | The shell | `scripts/check-architecture.sh` | the package graph |
| **Contract** | Any OS | `node --test evals/contracts.test.mjs` | the `.claude` layer — the agents, the skills, the commands — against the schemas; plus the package graph, `project.yml`, the placeholder leaks |
| **UI / integration** | A simulator or a device | XCUITest | the screens, the real iOS APIs — **not shipped** |
| **E2E** | A simulator or a device | XCUITest / Maestro | the user journeys — **not shipped** |

The template ships the three Swift Testing tiers, the architecture gate and
the contract tier. The **UI / integration** and **E2E** tiers are adoption
steps — §10 is where they go, not tests that already exist.

The Swift tests use **Swift Testing**, not XCTest: `import Testing`,
`@Suite`, `@Test` and `#expect`. There is no XCTest target in the tree.

The split between the tiers is the package split: `Kernel`, `NotesDomain`
and `NotesUI` each ship a `testTarget` in their own `Package.swift`, and
`swift test` runs the suite of the package it sits in.

## 2. The per-package test command

This is the detail that trips every new team: there is **no root test task**.
SwiftPM has no workspace, so the suite is run per package.

```sh
cd Packages/Core/Kernel && swift test
cd Packages/Feature/Notes/Domain && swift test
cd Packages/Feature/Notes/UI && swift test
```

`scripts/native-gates.sh` does exactly that, over **all six packages**:

| Package | Test target | What the suite covers |
| --- | --- | --- |
| `Packages/Core/Kernel` | `KernelTests` | the kernel's `Outcome` mapping and its accessors |
| `Packages/Core/UICore` | none | no suite — the package is compiled only |
| `Packages/Feature/Notes/Contracts` | none | no suite — the package is compiled only |
| `Packages/Feature/Notes/Domain` | `NotesDomainTests` | the use case against a stub repository |
| `Packages/Feature/Notes/Data` | none | no suite — the package is compiled only |
| `Packages/Feature/Notes/UI` | `NotesUITests` | the `@Observable` model against a stub repository |

Three of the six packages declare **no** test target, so their part of the
native tier is a compile check and nothing more. A package that ships a
product without a suite is a gap to close, not a package to leave empty: the
`add-package` skill makes the test target part of the package
([13 — Playbooks](13-playbooks.md)).

`make test` is the shortcut for the whole native tier's test half
(`bash scripts/native-gates.sh test`).

## 3. The layout

The tests live **inside the package they test**, because SwiftPM requires
the test target to be there:

```text
Packages/Core/Kernel/Tests/KernelTests/            # the Outcome and the error codes
Packages/Feature/Notes/Domain/Tests/NotesDomainTests/  # the entity and the use case
Packages/Feature/Notes/UI/Tests/NotesUITests/       # the model
evals/contracts.test.mjs                             # the .claude contract tests
```

Three packages have tests. `NotesContracts` has **none** yet, and
`NotesData` has none either: the in-memory adapter is a fixed list of two
notes, and a test of a fixed list proves nothing.

The **route round-trip test** belongs in `NotesContracts` because the routes
live there — and because that package is pure Swift, the test needs no
simulator. That is a direct payoff of the package split. The test itself is
**not implemented**: it lands with the shared route table
([06](06-navigation-routes-and-deep-links.md)).

## 4. Fakes, not mocks

There is no mocking framework in the dependencies. The fakes are
hand-written types that implement the ports — in the shipped tests, a
private struct at the call site:

```swift
// The shape of a fake repository: it implements the port the domain
// declared, and it returns exactly the state the test needs to assert on.
private struct StubRepository: NoteRepository {
    let result: @Sendable () async throws -> [Note]

    func load() async throws -> [Note] {
        try await result()
    }
}
```

A fake that is ten lines long is cheaper than a framework, and it makes the
port boundary visible: if a fake gets awkward, the port is wrong.

The rule: **a port is never tested against a real implementation.** The use
cases are tested against the fake; the fake is not the thing under test.

| Test | Against |
| --- | --- |
| The use case | The stub repository |
| The model | The real use case, with the stub repository underneath |
| The screen (simulator) | The real model |

## 5. The architecture check is a gate

`scripts/check-architecture.sh` reads the manifests and the sources and
fails when the package graph is wrong
([03](03-packages-layers-and-udf.md)).

It is a **gate, not a lint**: it exits non-zero, so CI fails. The same
rules are enforced twice on purpose — the compiler catches the impossible
mistakes (an `import SwiftUI` in a pure package), and the script catches the
possible ones (a `NotesUI → NotesData` dependency, a tracked `.xcodeproj`,
a manifest that dropped out of Swift 6 mode).

## 6. The evals

`evals/contracts.test.mjs` is a **Node test suite** that validates the
`.claude` layer: the agents, the skills and the commands against the schemas
in `.claude/schemas/`. It also asserts the iOS posture — the six packages
exist and declare Swift 6 mode, the pure packages import no UI framework,
the UI never reaches the data package, `project.yml` declares exactly one
application target and no credential, and no placeholder survives the
rename.

```sh
node --test evals/contracts.test.mjs
```

Two properties of the evals:

| Property | Why |
| --- | --- |
| They are **pure Node** | The `.claude` layer is text; validating it needs no macOS and no Xcode |
| The leak detector's tokens are **assembled at runtime**, not written literally | A literal token would be rewritten by `scripts/rename.sh` and the detector would be defused |

The evals are the gate for the kit itself. A change to an agent, a skill or
a command is finished when `node --test evals/contracts.test.mjs` is green.

`evals/run-live.sh` is the **live tier**: it drives the `claude` CLI against
a skill and, with `TIER=3`, a judge pass over a canned review. It costs API
budget and needs a live CLI, so it is never in the default gate —
`verify-template.sh` only proves it is a no-op without `EVALS=1`.

## 7. The commands

```sh
bash scripts/check-architecture.sh     # the package graph
node --test evals/contracts.test.mjs   # the .claude contract tests
bash scripts/native-gates.sh           # swift test + xcodegen + xcodebuild (macOS)
bash scripts/native-gates.sh test      # the Swift suites only
bash scripts/native-gates.sh build     # the app target only
```

The `make` targets wrap them:

| Command | What it runs |
| --- | --- |
| `make ci` | `make lint`, the contract evals and the native tier, in one go |
| `make lint` | `check-architecture.sh`; on a renamed tree, also `lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh`, `instruction-budget.sh` |
| `make evals` | `node --test evals/contracts.test.mjs` |
| `make test` | `bash scripts/native-gates.sh test` — the Swift suites |
| `make build` | `bash scripts/native-gates.sh build` — `xcodegen generate` + `xcodebuild` |
| `make generate` | `xcodegen generate` |
| `make verify` | `bash scripts/verify-template.sh` — a staged renamed copy, every gate run on it |
| `make clean` | the generated `.xcodeproj`, `DerivedData` and the SwiftPM `.build` dirs |

The `.claude` linters forbid the very placeholders the template root still
carries, so `make lint` runs them only after the rename — on the un-renamed
template it prints `lint: template placeholders present …` and moves on.
`make verify` runs them on the staged copy. Two scripts are **not** in
`make lint` at all, and that is deliberate: `scripts/leak-guard.sh` scans for
residual placeholders and secret-shaped values and is run by
`verify-template.sh` (`--placeholders-only`), and
`scripts/install-claude-runtime.sh` installs the kit into other agent
runtimes.

`bash scripts/verify-template.sh` is the gate above the gate: it stages a
renamed copy of the kit under a temp dir and runs the whole portable set on
it, so a break in the substitution contract fails before the release, not
after it. The working tree is never mutated.

## 8. The gate order

The order in the CI job, cheapest first:

| # | Step | Why here |
| --- | --- | --- |
| 1 | `bash scripts/check-architecture.sh` | Milliseconds, and it catches the mistake that wastes the rest |
| 2 | `bash scripts/lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh` | Text checks on the `.claude` tree — on a renamed tree; `make verify` runs them on the staged copy |
| 3 | `bash scripts/instruction-budget.sh` | The always-on instruction set must fit the harness budget |
| 4 | `node --test evals/contracts.test.mjs` | Fast, and it needs only Node |
| 5 | `bash scripts/native-gates.sh test` | The Swift suites; needs the Apple toolchain |
| 6 | `bash scripts/native-gates.sh build` | The compile proof against the real SDK; the slowest one |

## 9. What a Linux CI can and cannot prove

This is the honest boundary of the gate set, and it matters when the CI
runs on a Linux runner.

| Proven on Linux | **Not** proven on Linux |
| --- | --- |
| The package graph is correct (`check-architecture.sh`) | That any Swift file compiles |
| The `.claude` layer matches its schemas (the contract evals) | That the Swift suites pass |
| The always-on instruction set fits the budget | That the app target builds against the iOS SDK |
| The placeholder sweep on a renamed copy (`verify-template.sh`) | That the deployment target, the bundle identifier and the signing configuration are real |

`scripts/native-gates.sh` on a non-macOS host prints

```text
native-gates: SKIP — macOS + Xcode are required (xcodegen, swift, xcodebuild).
```

and exits **0**. The same is true when the host is macOS but one of
`xcodegen`, `swift` or `xcodebuild` is not on `PATH`. The `SKIP` is
visible, not silent — **report the SKIP, never fake a build**
([AGENTS.md](../AGENTS.md)).

So `make ci` stays green on a Linux runner, but a green `make ci` there
proves the *portable* half of the contract only. The native half — the
Swift suites, the generated project and the app build — needs a macOS
runner with Xcode and XcodeGen.

The `SKIP` is also reported by `verify-template.sh`, one level up:
`verify-template: SKIP native-gates (macOS + Xcode required)`.

## 10. Simulator and device tests

Simulator tests need a **simulator or a device**; they are not part of the
fast gate. The template ships no XCUITest target: the app target has no
test host, and the pure packages need none.

| What | Where | When |
| --- | --- | --- |
| The screens, per `UiStatus` | a new UI test target in the app project | The screen's behaviour is only observable on a device |
| The Keychain | a UI test target | There is no host-side equivalent |
| The notification categories and the APNs token | a UI test target | The API is iOS-only |
| The E2E journeys | a separate target or a CI job | The full flow |

The rules:

| Rule | Why |
| --- | --- |
| A simulator test is a **host-side test that could not be** | If it can run on the host, it runs on the host |
| The E2E suite runs on **real devices** from the pool, not only simulators | The Low tier is the target ([02](02-device-tiers-and-the-quality-governor.md)) |
| No production data in a test | The data is the liability |
| A flaky test is quarantined with an owner and an expiry | A flaky test in the gate trains the team to ignore the gate |
| Every route and every notification type has a contract test | The routes are typed ([06](06-navigation-routes-and-deep-links.md)) |

The first native build needs a network for SwiftPM resolution and an
installed **iOS 18 simulator runtime**
([05](05-project-layout-and-build-logic.md)).

## 11. Writing a test

| Rule | Why |
| --- | --- |
| One behaviour per test; name the behaviour | A failing test names one thing |
| Use the stub, never the real adapter | The test must not need a disk or a network |
| Assert the value, not the log | A test that reads the log breaks when the log changes |
| `#expect`, never a real clock or a real sleep | A test that sleeps is a flaky test |
| A model test is `@MainActor` | The model is `@MainActor`; the test inherits the isolation |
| No test touches a real network, a real clock or a device | The same |

The shipped model test shows the shape:

```swift
@MainActor
@Suite("NotesModel")
struct NotesModelTests {
    @Test func mapsANonEmptyListToReady() async {
        let model = makeModel()
        await model.refresh()
        #expect(model.state.status == .ready)
    }
}
```

## 12. The red test before the code

The loop is the same as in the other kits: write the failing test, watch it
fail **for the right reason** — a missing symbol, not a typo — then write
the smallest change that makes it pass.

```sh
cd Packages/Feature/Notes/Domain && swift test --filter LoadNotesTests
```

`--filter` narrows a suite or a test by name, on any Swift Testing package.

## Do / don't

| Do | Don't |
| --- | --- |
| Put the test in the package it covers | Collect every test in one folder |
| Run the suite in the package, on macOS | Assume a root-level `swift test` exists |
| Test the port against a stub | Test the use case against the real adapter |
| Run the architecture check in CI | Rely on a review to catch the graph |
| Keep the evals green with every `.claude` change | Edit an agent without running the contract evals |
| Report the native `SKIP` as a `SKIP` | Report a build that was never run |

## See also

- [Packages, Layers & UDF](03-packages-layers-and-udf.md) — what the
  architecture check enforces.
- [Playbooks](13-playbooks.md) — the recipe for adding a test.
- [Project Layout & Build Logic](05-project-layout-and-build-logic.md) — the
  package tree and the toolchain.
- [ADR 0002 — One SwiftPM package per layer](adr/0002-one-swiftpm-package-per-layer.md)
  — why the tests are per package.
