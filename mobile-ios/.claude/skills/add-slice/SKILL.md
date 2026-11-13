---
name: "add-slice"
description: "Scaffolds a new vertical slice in the <app> iOS template — NotesContracts, NotesDomain, NotesData and NotesUI under Packages/Feature/<name> — with the use case, the port, the Observable model and the SwiftUI screen, wired in the app target."
division: core
---

# `<app>` add-slice skill

> Reusable workflow for adding one vertical slice to `<app>`. An agent invokes
> this skill; the requester supplies the feature, the skill supplies the
> scaffolding, the package discipline and the gate ladder.

## When to Activate

Activate this skill when the request is a new feature that owns its own domain
— a new kind of entity, screen or workflow that will grow: a new "tasks"
slice, a new "settings" slice, a new "reminders" slice.

Do NOT activate this skill when:

- The change adds a field, a rule or a screen to the **existing** `notes`
  slice — that is a change inside `Packages/Feature/Notes/*`, not a new slice.
- The change is a bug fix (fix it in place, with a red test first).
- The request is a new shared package rather than a feature slice (use
  `add-package`).

## Core Principles

- **Vertical, not horizontal.** A slice owns one feature end to end: contract,
  domain rules, adapters and presentation.
- **Four packages, four responsibilities.** `Contracts` (routes other slices
  may use), `Domain` (pure Swift: entities, use cases, ports), `Data` (port
  implementations) and `UI` (views, `@Observable` model, state/event/effect).
- **Ports are consumer-defined.** The interface a use case needs lives in
  `Domain`; the implementation that satisfies it lives in `Data` and reaches
  URLSession, SwiftData or files.
- **The composition root wires, it does not decide.** `AppEnvironment`
  constructs the implementation and hands the use case to the model.
- **Mirror the `notes` slice.** Copy its shape instead of inventing a new one.
- **A route is part of the slice.** A screen means a typed enum case in
  `Contracts` registered in the app's `navigationDestination`.

## Rules

- No fabricated paths: every file created is listed with its real path.
- `Domain` is pure Swift: no SwiftUI, no UIKit, no framework type.
- `UI` never imports `Data`; it receives collaborators through its model.
- Every package is declared in `project.yml` and its edges in `Package.swift`.
- The slice ships with at least one test per package: a domain test, a model
  test against a fake port, and a route round-trip test when the slice
  navigates.
- No new remote dependency without a `docs/dependency-register.md` entry.
- The slice is not done until `bash scripts/check-architecture.sh`,
  `node --test evals/contracts.test.mjs` and `make ci` are green, and the real
  output is reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The slice anatomy and the permitted dependency direction | `PLAYBOOK_MOBILE_IOS_SWIFT.md` §4 | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |
| A package boundary is enforced by the build | Apple — Organizing your code with local packages | 2026 | https://developer.apple.com/documentation/xcode/organizing-your-code-with-local-packages |
| A view is a function of observable state | Apple — Observation | 2026 | https://developer.apple.com/documentation/observation |
| Typed navigation paths are checked at compile time | Apple — NavigationStack | 2026 | https://developer.apple.com/documentation/swiftui/navigationstack |

## Output Format

```
Slice: <name> — packages: Packages/Feature/<Name>/{Contracts,Domain,Data,UI}
Created:
  contracts/  <files> — <public routes, events>
  domain/     <files> — <entities, use cases, ports>
  data/       <files> — <port implementations>
  ui/         <files> — <model, view state/event/effect, views>
  tests/      <files> — <domain, model cases>
Wiring: <what AppEnvironment now constructs>
Route: <route case> — registered in <navigationDestination>
Gates:
  bash scripts/check-architecture.sh -> <result>
  node --test evals/contracts.test.mjs -> <result>
  make ci -> <result>
Open: <anything left for the next slice>
```
