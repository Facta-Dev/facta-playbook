---
name: add-package
description: "Delegate to the add-package skill — add a Swift package to the <app> iOS template with its Package.swift in Swift 6 mode, its purity rule, its test target and its project.yml reference."
division: core
---

# /add-package

## Delegate

Delegate to the **add-package** skill. Activate when `<app>` needs a new
shared module — a design-system package, a shared adapter, or a capability
wrapper used by more than one slice.

Steps:

1. Decide the layer: `Packages/Core/` for shared, `Packages/Feature/<slice>/`
   for owned by one slice.
2. Write `Package.swift`: one product equal to one target equal to the
   directory name, `platforms: [.iOS(.v18), .macOS(.v15)]`,
   `swiftLanguageModes: [.v6]`, and only inward dependencies.
3. Create `Sources/<Name>/` and `Tests/<Name>Tests/`; the test target is part
   of the package.
4. State the purity rule: a package the domain may use imports no SwiftUI or
   UIKit.
5. Add the package to `project.yml` under `packages:` and to the app target
   only when the app uses it.
6. Update `docs/dependency-register.md` when the package pulls a remote
   dependency.
7. Run `swift test` in the package (macOS), `bash
   scripts/check-architecture.sh` and `make ci`; report the real output or
   the visible SKIP.
