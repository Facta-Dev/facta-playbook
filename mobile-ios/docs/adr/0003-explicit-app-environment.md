# ADR 0003 — Explicit AppEnvironment

- **Status**: Accepted
- **Date**: 2026
- **Scope**: dependency wiring in the native iOS template

## Context

A DI container is the standard answer to "who constructs the object graph?".
In Swift the options are frameworks such as Factory, Swinject or Resolver, or
a property-wrapper-based container. Each buys automatic resolution and each
costs something:

- a runtime dependency and a registration step per type;
- resolution errors that surface at runtime, not at compile time;
- a container type that leaks into every initializer, so a domain type knows
  a framework exists;
- a test that constructs half a container to build one model.

The template is one app target with six packages and one feature slice. There
is exactly one graph, it is small, and the compiler already knows how to
compose it: a struct that holds the objects and a closure that builds them.

## Decision

**`AppEnvironment` is the composition root. Wiring is explicit, compile-time
checked and framework-free.**

```swift
@MainActor
struct AppEnvironment {
    let makeNotesModel: @MainActor () -> NotesModel

    static let live = AppEnvironment {
        NotesModel(loadNotes: LoadNotes(repository: InMemoryNoteRepository()))
    }
}
```

The rules that make it hold:

- `AppEnvironment` lives in the app target, which is the only place that may
  see every package.
- It holds factories, not singletons: a `() -> NotesModel` lets a screen own
  its model without sharing mutable state.
- `AppEnvironment.live` is the production graph; a test builds the same model
  with a stub repository and never touches the environment.
- A new dependency is a new line in `static let live` — visible in one diff,
  impossible to register twice.
- The graph is checked at compile time: a missing argument is a build error,
  not a runtime "unregistered type" failure.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Hilt/Koin-style framework (Factory, Swinject, Resolver) | A runtime dependency plus a registration file per layer, and resolution errors surface at runtime. The template's graph fits in one struct; the framework is more machinery than the problem. |
| A service locator (`ServiceLocator.shared.resolve(...)`) | The dependency disappears from the initializer signature, so a domain type can reach anything and the compiler cannot see the edge. It hides the graph instead of declaring it. |
| Property wrappers on every type (`@Injected var repo`) | Same locator problem with nicer syntax, plus reflection at init. Tests then need the locator configured, which is the thing they were avoiding. |
| A global `AppDependencies` singleton | Mutable global state, untestable in parallel, and the singleton outlives a test that meant to replace one adapter. The factories in `AppEnvironment` keep the graph a value. |
| Construct everything inside each view | The view would need to know the adapter, which is exactly the `UI → Data` edge the architecture forbids. |

## Consequences

- **The graph is one file.** A reviewer sees the whole object graph in
  `AppEnvironment.swift`.
- **The compiler checks the wiring.** A missing adapter is a build error, not
  a runtime crash on the first screen.
- **Tests are direct.** A model test constructs `NotesModel(loadNotes: ...)`
  with a stub; no container is built, no registration is reset.
- **A deep view hierarchy passes dependencies explicitly.** SwiftUI's
  `Environment` carries the model factory from `RootView`; a screen that needs
  a new collaborator receives it in the initializer. This is the intended
  friction: the dependency is visible at the call site.
- **Adding a real adapter is one line.** When `InMemoryNoteRepository` is
  replaced by a network adapter, the change is in `AppEnvironment.live` (and
  the adapter package), not across the screens.
- **No framework migration cost.** If the app grows to need a container, the
  decision can be revisited with a concrete graph to justify it; nothing in
  the domain depends on the current wiring.

## See also

- [ADR 0002 — One SwiftPM package per layer](0002-one-swiftpm-package-per-layer.md)
- [Packages, Layers & UDF](../03-packages-layers-and-udf.md)
- [Project Layout & Build Logic](../05-project-layout-and-build-logic.md)
