# ADR 0003 — Manual dependency injection in the app module

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the composition root of the native Android template

## Context

The template has to decide how the object graph is built. On Android the
usual options are:

- **a DI framework** (Hilt, Koin, Dagger, and the rest) with its
  annotations, its code generation and its own build plugin;
- **manual construction**, in one place, with plain constructor calls;
- **a service locator**, where any code can reach for a global;
- **nothing**, with singletons scattered across the tree.

The template's graph is small. Eight modules, one slice, and an object
graph that a human can hold in their head:

```text
App() — the composable composition root in :app
├── InMemoryNoteRepository
├── LoadNotes(repository)
└── NotesViewModel(loadNotes, dispatcher)
```

The four ViewModels in a real app and the three ports under them do not
change the shape of that decision.

At the same time, three constraints from
[ADR 0001](0001-one-gradle-module-per-layer.md) push the same way:

1. **The composition root is `:app`**, and `:app` is the only module that
   sees every layer. A DI framework does not change that: the graph is
   still declared in `:app`, and the framework's annotations still live
   there.
2. **The pure JVM modules must stay pure.** A framework's runtime is an
   Android library; the domain would gain a dependency it does not need.
3. **The template must be runnable after `rename.sh` and a Gradle build**,
   with the smallest possible dependency surface. A DI framework adds a
   plugin, an annotation processor and a runtime library to the build of a
   project that has one repository and one use case.

## Decision

**Build the object graph by hand, in `:app`, and pass the dependencies in
through constructors. No DI framework, no service locator.**

The shape, as the template ships it:

```kotlin
// MainActivity.kt — the composition root. There is no container class and
// no Application subclass: the graph is one expression, at the point where
// it is needed.
val viewModel: NotesViewModel = viewModel {
    NotesViewModel(LoadNotes(InMemoryNoteRepository()))
}
```

The rules that come with the decision:

- **One composition root.** `:app` builds the graph, in one place. In the
  template that place is the `App()` composable; a project with per-process
  work moves it to an `Application` subclass
  ([01](../01-platform-model-and-android-runtime.md) §4).
- **Dependencies are constructor parameters.** A class declares what it
  needs; it never reaches for a global.
- **The composition root is the only place that chooses an implementation.**
  The domain declares the port; `:app` decides that the port is currently an
  `InMemoryNoteRepository`.
- **The dispatcher is injected.** The ViewModel receives a
  `CoroutineDispatcher` instead of importing `Dispatchers.IO` inline. That
  is what makes the ViewModel testable with a `TestScope` dispatcher
  ([12](../12-testing-and-ci.md)).
- **No `Context` in the graph.** The objects are plain, so they are
  testable without a device.
- **The composition root is not a service locator.** Nothing reaches into
  it to fetch something at runtime; the dependencies arrive at
  construction.

## Alternatives considered

| Option | Why not |
| --- | --- |
| **Hilt** (or Dagger) | The strongest option for a large app: compile-time graph validation, `@HiltViewModel`, assisted injection. It is also a build plugin, an annotation processor, a runtime library, a KSP or KAPT dependency, and a set of conventions to learn. For a graph of one repository and one use case, it is more machinery than the problem. |
| **Koin** (or another runtime container) | No code generation, but a runtime lookup: the graph is discovered when the app runs, not when it compiles. The failure moves from the build to the first user. |
| **A service locator** | Any code can fetch anything, at any time. The dependency graph becomes invisible, and the test needs the locator wired. The composition root is already a locator in miniature — keeping it a constructor argument is the whole difference. |
| **Singleton objects** (`object InMemoryNoteRepository`) | The dependencies are invisible, the tests share state, and the order of initialization becomes a hidden contract. The same problem as the service locator, with less control. |
| **A DI framework in `:app` only** | The viable middle ground: the framework stays out of the pure JVM modules and the annotations live in the composition root. It is the right move **later**, when the graph outgrows a human. It is not the right move now. |
| **A container class in `:app`** | The shape a project adopts as soon as the graph grows. The template does not ship one, because a class with one field that nothing reads is a class that only hides where the wiring happens. |
| **Koin or Hilt in `:app` from the start** | The "later" case, decided now: every class written before the switch is a class the migration has to touch, and the migration is the expensive part. |

## Consequences

- **The graph is a file a reviewer can read.** Eight modules and one
  composition root: the whole wiring is on one screen, in plain Kotlin, with
  no annotation to interpret.
- **A missing dependency is a compile error.** The constructor signature
  says what is needed; the call site says what is provided. There is no
  injection graph to validate at runtime.
- **A test builds what it needs.** A use-case test constructs the use case
  with a fake repository and a test dispatcher — no container, no
  framework, no `Application`. This is why the fakes in
  [12](../12-testing-and-ci.md) are hand-written: a framework's scope
  annotations would add noise to every test.
- **The wiring is duplicated in the tests.** This is the accepted cost:
  a test that builds its own dependencies is a test that states its own
  dependencies.
- **Adding a dependency is a two-line change**: a constructor parameter and
  a line in the composition root. The compiler names every call site that
  needs updating.
- **There is no scoped lifecycle.** No `@Singleton`, no `@ActivityScoped`,
  no request scope: an object that must outlive a screen is a field of the
  composition root, and that root lives as long as the process.
- **The composition root grows.** It starts as one expression and becomes a
  container class with one parameter per dependency; a long parameter list is
  the signal that the graph has outgrown manual wiring.
- **The migration path is written down** in the table below, and it is a
  refactor of `:app` only.

### When to adopt a DI framework

| Signal | What it means |
| --- | --- |
| The composition root passes the same object to more than three classes | The object has a lifecycle the root is guessing at |
| A feature needs a **scoped** object — one per session, one per screen, one per request | Scoping is a real concept now, not a convenience |
| The graph is built **dynamically** — a plugin, a variant, a feature flag | A static constructor cannot express it |
| The team asks for the graph to be **validated at compile time** across a large tree | That is what the framework is for |
| The composition root passes **more than about fifteen** dependencies | The wiring is bigger than a human should hold |

When one of those signals fires, the migration is: add the framework to
`:app` only, keep the pure JVM modules free of it, and move the
composition root's contents into the framework's module one constructor at a
time. The constructor signatures do not change, so the tests keep working.

## See also

- [ADR 0001 — One Gradle module per layer](0001-one-gradle-module-per-layer.md)
- [Layers, Modules & UDF](../03-layers-modules-and-udf.md)
- [Testing & CI](../12-testing-and-ci.md)
- [Dependency Register](../dependency-register.md)
