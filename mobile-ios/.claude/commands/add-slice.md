---
name: add-slice
description: "Delegate to the add-slice skill — scaffold a new vertical slice in the <app> iOS template: Contracts, Domain, Data and UI packages under Packages/Feature/<name>, the use case, the port, the Observable model, the SwiftUI screen, and the wiring in the app target."
division: core
---

# /add-slice

## Delegate

Delegate to the **add-slice** skill. Activate when a `<app>` feature needs its
own vertical slice rather than another field inside the existing `notes` slice.

Steps:

1. Name the slice and its four packages:
   `Packages/Feature/<Name>/Contracts`, `Domain`, `Data` and `UI`.
2. Write the domain first: entities, the use case, the port it needs and
   `Outcome`-shaped signatures — pure Swift, no SwiftUI or UIKit.
3. Write the contract: the route enum other slices may use.
4. Write the adapter in `Data` and the presentation in `UI` — an
   `@Observable @MainActor` model with view state, events and an `AsyncStream`
   of effects, and a view that is a function of that state.
5. Wire it by hand in the app target: `AppEnvironment` constructs the
   implementation, and `Router`/`navigationDestination` gain the route.
6. Add the package references in `project.yml` and the edges in each
   `Package.swift`; both declare Swift 6 mode.
7. Add the tests (domain, model against a fake port, route round-trip), then
   run `bash scripts/check-architecture.sh`, `node --test
   evals/contracts.test.mjs` and `make ci`, and report the real output.
