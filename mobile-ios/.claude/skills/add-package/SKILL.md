---
name: "add-package"
description: "Adds a Swift package to the <app> iOS template with its layer rules: a Package.swift in Swift 6 mode, purity constraints, a test target and the project.yml reference."
division: core
---

# `<app>` add-package skill

> A package is a layer decision before it is code. This skill adds one with
> the manifest, the purity rule, the test target and the wiring already right.

## When to Activate

Activate when a new shared module is needed: a second feature slice (use
`add-slice` instead for a full slice), a shared adapter (networking, storage),
a design-system package, or a platform capability wrapper used by more than
one slice.

Do NOT activate when:

- The symbol can live in an existing package without breaking its rule.
- The "package" is one file used by one caller; put it in the caller's
  package.

## Core Principles

- **A package is a boundary with a rule.** State what it may depend on before
  writing the manifest.
- **Pure below the UI.** A package that the domain may use imports no SwiftUI
  or UIKit.
- **One product per package.** The product name equals the target name equals
  the directory name, so the graph is readable in `project.yml`.
- **Swift 6 by default.** Every manifest declares `swiftLanguageModes: [.v6]`
  and every target compiles under strict concurrency.
- **Tests ship with the package.** A test target is part of the package, not
  an afterthought in the app.

## Rules

- The new directory lives under `Packages/Core/` (shared, no feature) or
  `Packages/Feature/<slice>/` (owned by one slice).
- The manifest declares `platforms: [.iOS(.v18), .macOS(.v15)]`,
  `swiftLanguageModes: [.v6]`, and only inward dependencies.
- The package is added to `project.yml` under `packages:` and to the app
  target's `dependencies:` only when the app uses it.
- `scripts/check-architecture.sh` gains the new edge if the package is pure.
- `docs/dependency-register.md` is updated when the package pulls a remote
  dependency.
- The gate: `swift test` in the new package (on macOS), `make ci`, and
  `bash scripts/check-architecture.sh`.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| A local package is a first-class module boundary | Apple — Creating a standalone Swift package | 2026 | https://developer.apple.com/documentation/xcode/creating-a-standalone-swift-package-with-xcode |
| Swift 6 language mode enforces data-race safety | Swift.org — Adopting Swift 6 | 2026 | https://www.swift.org/migration/documentation/swift-6-concurrency-migration-guide/ |
| XcodeGen declares package references in project.yml | XcodeGen — ProjectSpec | 2026 | https://github.com/yonaskolb/XcodeGen/blob/master/Docs/ProjectSpec.md |

## Output Format

```
Package: <name>
Path: Packages/<layer>/<Name>
Kind: pure Swift | UI | adapter
Depends on: <packages, inward only>
Created:
  Package.swift — <products, targets, swiftLanguageModes>
  Sources/<Name>/<files>
  Tests/<Name>Tests/<files>
Wiring: project.yml packages + target dependencies
Purity: <imports allowed / forbidden>
Gates:
  swift test — <result or SKIP: no toolchain>
  bash scripts/check-architecture.sh -> <result>
  make ci -> <result>
```
