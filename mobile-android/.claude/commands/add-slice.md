---
name: add-slice
description: "Delegate to the add-slice skill — scaffold a new vertical slice in the <app> Android template: :feature:<name>:{contracts,domain,data,ui}, the use case, the port, the ViewModel, the Compose screen and the hand-made wiring in :app."
division: core
---

# /add-slice

## Delegate

Delegate to the **add-slice** skill. Activate when a `<app>` feature needs its
own vertical slice rather than another field inside the existing `notes` slice.

Steps:

1. Name the slice and its four modules: `:feature:<name>:contracts`,
   `:feature:<name>:domain`, `:feature:<name>:data` and
   `:feature:<name>:ui`.
2. Write the domain first: entities, the use case, the port it needs and the
   `Outcome<T>`-shaped signatures — pure Kotlin, no `android.*`.
3. Write the contract: the interfaces, the events and the `@Serializable`
   `NavKey` other slices may use.
4. Write the adapter in `:feature:<name>:data` and the presentation in
   `:feature:<name>:ui` — ViewModel with `UiState`/`UiEvent`/`UiEffect`, and
   a composable that is a function of that state.
5. Wire it by hand in `:app`: construct the implementation, register the
   `NavKey` in the `entryProvider`, and pass the dependencies through the
   ViewModel factory. This template uses manual DI — no generated component.
6. Register the modules in `settings.gradle.kts` and the dependencies in
   `gradle/libs.versions.toml`.
7. Add the tests (domain, ViewModel, Compose UI, and a `:evals` contract
   check when the slice talks to the server), then run `./gradlew
   :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test` and `bash
   scripts/check-architecture.sh`, and report the real output.
