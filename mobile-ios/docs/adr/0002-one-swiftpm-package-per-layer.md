# ADR 0002 — One SwiftPM package per layer

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the module structure of the native iOS template

## Context

An iOS codebase has one easy way to lose its architecture: every layer
compiles into one app target, so every layer can see every other layer. The
result is a domain that imports SwiftUI, a screen that calls a repository
directly, and a test that needs a simulator to check a business rule.

The usual defences are conventions and reviews:

- a folder naming convention that separates the layers;
- an architecture test that scans the source for a forbidden import;
- a SwiftLint custom rule;
- a review checklist item.

All four are **detections**. They run after the fact, they can be bypassed
with a comment, a `// swiftlint:disable` or an ignore list, and they are
checked only on the files somebody remembered to touch.

SwiftPM offers something the other approaches do not: the **package
dependency itself** is the rule. A package's `dependencies` array is the
complete list of what it can import, and the compiler enforces it on every
build, on every machine, with no allowlist to maintain.

For a pure package, the enforcement is total: if SwiftUI is not declared as a
dependency, `import SwiftUI` is not a review comment — it is a **compile
error**.

## Decision

**One SwiftPM package per layer, and the package's kind is the enforcement
mechanism.**

The template ships six packages plus one app target:

| Package | Kind | What it holds |
| --- | --- | --- |
| `Kernel` | Pure Swift | `Outcome`, `AppError`, `ErrorCode`, `NoteID` |
| `UICore` | Pure Swift | `UiStatus`, the closed status set |
| `NotesContracts` | Pure Swift | `NotesRoute`, the typed routes |
| `NotesDomain` | Pure Swift | The entity, the port, the use case |
| `NotesData` | Adapter | The in-memory repository |
| `NotesUI` | SwiftUI | The screen, the `@Observable` model, `NotesViewState` / `NotesEvent` / `NotesEffect` |
| `<APP>` app target | Application | The `@main` entry, `AppEnvironment`, `Router`, `RootView` |

The rules that make it hold:

- **A pure package has no UI framework in its manifest.** Its `Package.swift`
  declares no SwiftUI, no UIKit and no feature package above it. The
  consequence is the whole point: the domain *cannot* reach the platform.
- **Only the app target is an application.** The packages produce libraries;
  the `.xcodeproj` has exactly one `type: application`.
- **The dependency list of a package is its architectural contract.** It is
  one array in one manifest, with no wildcard and no transitive shortcut
  (`NotesUI` lists `NotesDomain` explicitly even though `NotesData` also does).
- **The graph is checked by a script, not by a convention.**
  `scripts/check-architecture.sh` asserts the forbidden edges the compiler
  cannot see (`UI → Data` and `Data → UI` are both buildable in SwiftPM), the
  SwiftUI/UIKit import ban in the pure packages, and that only the app target
  is an application.
- **A package that needs a new capability gets a new package**, or a new
  dependency in its manifest — never an import from a layer that should not
  know about it.

The consequence that matters most: **the layering is a build fact, not a
detection.** A forbidden import does not reach review; it fails the compile.
The script covers the edges the compiler cannot see, but it is the second
line of defence, not the first.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One app target, layers as folders | Every layer sees every layer. The layering exists only in folder names, and a folder name is a convention with no enforcement. |
| One package per *feature* only | A feature package contains the domain, the data and the UI together, so the domain can still import SwiftUI. The split that matters is *kind*, not *feature*. |
| One package per feature **and** per layer | The full grid duplicates every layer for every feature: one slice already needs four packages, and the second slice needs four more instead of reusing the shared core. One feature is enough to demonstrate the pattern. |
| A single target plus an architecture **test** | The test is a detection: it runs in CI, not on every keystroke; it needs an allowlist for legitimate exceptions; and it can be forgotten in a build that skips the check. The compiler never forgets. |
| Xcode project frameworks instead of SwiftPM packages | A `.framework` target is declared in the project and built by Xcode; SwiftPM packages are declared in text, testable from the command line, and portable to a future Swift package. The package is the lighter boundary. |
| Binary frameworks / prebuilt XCFrameworks | Distribution artefacts, not an architecture tool. They add a build-and-release step for code that is in the same repository. |

## Consequences

- **The layering is proven by the compiler.** A domain that imports SwiftUI
  does not compile. No review has to catch it.
- **Adding a layer is a manifest change**, not a refactor across the tree: a
  new package directory, a new `packages:` entry in `project.yml`, a new row
  in the dependency register.
- **The domain is testable off-device.** A pure package runs `swift test` on
  macOS with no simulator; the model test needs the main actor but no app
  host.
- **The build is slower than a single target.** Six packages means six
  modules to resolve and compile. SwiftPM caches the result, and the native
  tier runs it only on macOS.
- **The graph is a small, reviewable artifact.** Seven nodes and their edges
  fit in a diagram — the one in [03](../03-packages-layers-and-udf.md).
- **Cross-slice access has a single door.** A slice publishes its `Contracts`
  package and nothing else ([04](../04-vertical-slicing.md)).
- **The cost is real**: a developer who wants to put a helper "just next to
  the caller" must either add a package or accept a forbidden edge. That
  friction is the feature — it is the conversation the architecture should
  force, once, instead of in every review.

## See also

- [ADR 0001 — XcodeGen-generated project](0001-xcodegen-generated-project.md)
- [ADR 0003 — Explicit AppEnvironment](0003-explicit-app-environment.md)
- [Packages, Layers & UDF](../03-packages-layers-and-udf.md)
- [Vertical Slicing](../04-vertical-slicing.md)
- [Project Layout & Build Logic](../05-project-layout-and-build-logic.md)
