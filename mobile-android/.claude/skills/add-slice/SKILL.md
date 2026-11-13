---
name: "add-slice"
description: "Scaffolds a new vertical slice in the <app> Android template — :feature:<name>:contracts, :feature:<name>:domain, :feature:<name>:data and :feature:<name>:ui — with the use case, the port, the ViewModel and the Compose screen, wired by hand in :app."
division: core
---

# `<app>` add-slice skill

> Reusable workflow for adding one vertical slice to `<app>`. An agent invokes
> this skill; the requester supplies the feature, the skill supplies the
> scaffolding, the module discipline and the gate ladder.

## When to Activate

Activate this skill when the request is a new feature that owns its own domain
— a new kind of entity, screen or workflow that will grow: a new "tasks"
slice, a new "settings" slice, a new "reminders" slice.

Do NOT activate this skill when:

- The change adds a field, a rule or a screen to the **existing** `notes`
  slice — that is a change inside `:feature:notes:*`, not a new slice.
- The change is a bug fix (fix it in place, with a red test first).
- The request is a new shared Gradle module rather than a feature slice (use
  `add-module`).

## Core Principles

- **Vertical, not horizontal.** A slice owns one feature end to end: contract,
  domain rules, adapters and presentation. Nothing about the feature is
  scattered across `:core:kernel` or `:app`.
- **Four modules, four responsibilities.** `contracts` (what other slices may
  use), `domain` (pure Kotlin: entities, use cases, ports), `data` (port
  implementations) and `ui` (composables, ViewModel, `UiState`/`UiEvent`/
  `UiEffect`).
- **Ports are consumer-defined.** The interface a use case needs lives in
  `:feature:<name>:domain`, next to the use case; the implementation that
  satisfies it lives in `:feature:<name>:data` and reaches the SDK, the cache
  or the network.
- **The composition root wires, it does not decide.** `:app` constructs the
  implementations and hands them to the ViewModel through an explicit
  `ViewModelProvider.Factory` — this template uses manual DI, so there is no
  generated component to rely on.
- **Mirror the `notes` slice.** The template ships a worked example; copy its
  shape instead of inventing a new one.
- **A route is part of the slice.** A screen means a typed `@Serializable`
  `NavKey` registered in the Navigation 3 `entryProvider` of `:app`.

## Rules

- No fabricated paths: every file created is listed with its real path, and
  every existing path referenced exists in the tree.
- `:feature:<name>:domain` is a pure Kotlin module: no `android.*`, no
  `androidx.compose.*`, no framework type.
- `:feature:<name>:ui` never imports `:feature:<name>:data`; it receives
  collaborators as constructor parameters.
- Every module is registered in `settings.gradle.kts` and its dependencies are
  declared in `gradle/libs.versions.toml`.
- The slice ships with at least one test per module: a domain test, a
  ViewModel test against fakes, a Compose UI test for the screen, and a
  contract check in `:evals` when the slice talks to the server.
- No new third-party dependency without a reason; Compose BOM 2026.09.00 and
  the Kotlin standard library are enough for a slice.
- The slice is not done until `./gradlew :app:assembleDebug`, `./gradlew test`,
  `./gradlew :evals:test` and `bash scripts/check-architecture.sh` are green,
  and the real output is reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The slice anatomy and the permitted dependency direction | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4.1 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| A multi-module app keeps the boundary enforced by the build | Android Developers — Feature modules | 2026 | https://developer.android.com/guide/navigation/integrations/feature-modules |
| A screen is a function of state and reacts to events | Android Developers — Compose architecture | 2026 | https://developer.android.com/develop/ui/compose/architecture |
| Navigation 3 routes are typed keys registered by the application | AndroidX Navigation release notes | 2026 | https://developer.android.com/jetpack/androidx/releases/navigation3 |
| Compose UI tests assert on semantics, not on the node tree | Android Developers — Compose testing | 2026 | https://developer.android.com/develop/ui/compose/testing |

## Output Format

```
Slice: <name> — modules: :feature:<name>:{contracts,domain,data,ui}
Created:
  contracts/  <files> — <public interfaces, events, NavKey>
  domain/     <files> — <entities, use cases, ports>
  data/       <files> — <port implementations>
  ui/         <files> — <ViewModel, UiState/UiEvent/UiEffect, composables>
  tests/      <files> — <domain, ViewModel, UI cases>
Wiring: <what :app now constructs and injects>
Nav key: <NavKey type> — registered in <entry provider>
Gates:
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
Open: <anything left for the next slice>
```
