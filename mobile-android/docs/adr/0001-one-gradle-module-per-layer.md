# ADR 0001 — One Gradle module per layer

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the build structure of the native Android template

## Context

An Android codebase has one easy way to lose its architecture: every layer
compiles into one module, so every layer can see every other layer. The
result is a domain that imports `android.content.Context`, a screen that
calls a repository directly, and a test that needs a device to check a
business rule.

The usual defences are conventions and reviews:

- a package naming convention that separates the layers;
- an architecture test that scans the source for a forbidden import;
- a lint rule, or a dependency-cruiser-style check;
- a review checklist item.

All four are **detections**. They run after the fact, they can be bypassed
with a comment, an annotation or an ignore list, and they are checked only
on the files somebody remembered to touch. On a codebase with eight layers
and a fast-moving team, the detection eventually loses to the pressure.

Gradle offers something the other stacks do not: the **module dependency
itself** is the rule. A module's `dependencies` block is the complete list
of what it can see, and the compiler enforces it on every build, on every
machine, with no allowlist to maintain.

For a pure JVM module, the enforcement is total: if the Android SDK is not
on the classpath, `import android.content.Context` is not a review comment
— it is a **compile error**.

## Decision

**One Gradle module per layer, and the layer's kind is the enforcement
mechanism.**

The template ships eight modules under a single root build:

| Module | Kind | What it holds |
| --- | --- | --- |
| `:app` | Android application | The composition root, the manifest, the `NavDisplay`, the manual wiring |
| `:core:kernel` | Pure JVM | `Outcome`, the id types, the error types |
| `:core:ui-core` | Android library | `UiStatus`, the closed status set |
| `:feature:notes:contracts` | Pure JVM | The `@Serializable` `NavKey` routes, the cross-slice interfaces |
| `:feature:notes:domain` | Pure JVM | The entity, the ports, the use cases |
| `:feature:notes:data` | Android library | The in-memory repository adapter |
| `:feature:notes:ui` | Android library | The Compose screen, the ViewModel, `NotesState` / `NotesEvent` / `NotesEffect` |
| `:evals` | Pure JVM | The JUnit contract tests for the `.claude` layer |

The rules that make it hold:

- **A pure JVM module has no `android.*` on its classpath.** Its build file
  applies `kotlin("jvm")` and nothing else. The consequence is the whole
  point: the domain *cannot* reach the platform.
- **An Android library module has no launcher.** It produces a library, not
  an app; only `:app` has an `applicationId` and a manifest entry.
- **The dependency list of a module is its architectural contract.** It is
  declared in one block, with `alias(libs.…)` and nothing else.
- **The graph is checked by a script, not by a convention.**
  `scripts/check-architecture.sh` asserts the forbidden edges the compiler
  cannot see (`ui → data` and `data → ui`), the `android.*` import ban in
  the kernel and the domain, and that only `:app` is an application.
- **A module that needs a new capability gets a new module**, or a new
  dependency in the version catalog — never an import from a layer that
  should not know about it.

The consequence that matters most: **the layering is a build fact, not a
detection.** A forbidden import does not reach review; it fails the
compile. The script covers the edges the compiler cannot see, but it is the
second line of defence, not the first.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One module for the whole app | Every layer sees every layer. The layering exists only in package names, and a package name is a convention with no enforcement. |
| One module per *feature* only | A feature module contains the domain, the data and the UI together, so the domain can still import `android.*`. The split that matters is *kind*, not *feature*. |
| One module per feature **and** per layer | The full grid duplicates every layer for every feature: one slice already needs four modules, and the second slice needs four more instead of reusing the shared core. The edges multiply with every feature added. One feature is enough to demonstrate the pattern. |
| A single module plus an architecture **test** | The test is a detection: it runs in CI, not on every keystroke; it needs an allowlist for the legitimate exceptions; and it can be forgotten in a build that skips the check. The compiler never forgets. |
| A single module plus an **ArchUnit**-style rule set | The same objection as above, plus a new test dependency, plus a second language (the rule DSL) to learn. |
| Convention plugins instead of plain build files | A shared plugin is the next step, not this one: it removes repetition at the cost of a layer of indirection. Eight modules with three shapes do not justify a build system of their own yet ([05](../05-project-layout-and-build-logic.md) §8). |
| KSP / annotation processing for the boundary | A processor that fails the build on a forbidden import is a compiler plugin in disguise. The module boundary already does the job, with less machinery. |

## Consequences

- **The layering is proven by the compiler.** A domain that reaches for a
  `Context` does not compile. No review has to catch it.
- **Adding a layer is a build change**, not a refactor across the tree: a
  new module, a new entry in `settings.gradle.kts`, a new row in the
  dependency register.
- **The build is slower than a single-module build.** Eight modules means
  eight configurations and eight compile tasks. This is the price of the
  enforcement, and it is paid in parallel by Gradle's configuration cache
  and build cache.
- **A pure JVM module is fast to test.** The domain and the contracts run
  on the JVM with `./gradlew test` — no emulator, no device, no
  `Robolectric`.
- **The two test tasks are not interchangeable.** `./gradlew test` runs the
  JVM modules; `./gradlew testDebugUnitTest` runs the Android modules. A CI
  job that runs only one of them runs half the suite
  ([12](../12-testing-and-ci.md)).
- **The graph is a small, reviewable artifact.** Eight nodes and their
  edges fit in a diagram — the one in [03](../03-layers-modules-and-udf.md).
- **Cross-slice access has a single door.** A slice publishes its
  `contracts` module and nothing else ([04](../04-vertical-slicing.md)).
- **The cost is real**: a developer who wants to put a helper "just next to
  the caller" must either add a module or accept a forbidden edge. That
  friction is the feature — it is the conversation the architecture should
  force, once, instead of in every review.

## See also

- [ADR 0002 — Typed NavKey routes](0002-navigation3-typed-navkeys.md)
- [ADR 0003 — Manual DI in the app module](0003-manual-di-in-the-app-module.md)
- [Layers, Modules & UDF](../03-layers-modules-and-udf.md)
- [Vertical Slicing](../04-vertical-slicing.md)
- [Project Layout & Build Logic](../05-project-layout-and-build-logic.md)
