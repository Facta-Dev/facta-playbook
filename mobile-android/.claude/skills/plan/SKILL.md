---
name: "plan"
description: "Produces a written implementation plan for a scoped <app> Android change: the problem, the constraints, the design per Gradle module, the files to touch, the risks and the test plan, saved under docs/plans/."
division: core
---

# `<app>` plan skill

> Reusable workflow for turning a scoped change into a written plan before any
> code is written. An agent invokes this skill; the requester supplies the
> change, the skill supplies the interrogation, the structure and the file.

## When to Activate

Activate this skill when a change to `<app>` is large enough to warrant a
written plan before implementation: a new screen and its slice, a change that
crosses Gradle modules, a new dependency or a toolchain bump, a migration of
the local cache, or a request whose acceptance criteria are still fuzzy.

Do NOT activate this skill when:

- The change is a single-file fix with an obvious location — implement it with
  a red test first.
- A plan already exists for the same change; update it instead of writing a
  second one.
- The request is to cut a release (use `ship`) or to state a requirement
  (use `spec`).

## Core Principles

- **Interrogate before writing.** The problem, the motivation, the scope
  boundary and the acceptance criteria are elicited and written down; an
  unstated assumption is a defect in the plan.
- **Design per module.** Each Gradle module that changes says what it gains:
  `:core:kernel` (pure Kotlin primitives), `:core:ui-core` (`UiStatus`, base
  ViewModel, navigation helpers), `:feature:notes:contracts` (the slice's
  public surface), `:feature:notes:domain` (entities, use cases, ports),
  `:feature:notes:data` (port implementations), `:feature:notes:ui`
  (composables, ViewModel, `UiState`/`UiEvent`/`UiEffect`) and `:app` (the
  manual DI graph and the Navigation 3 root).
- **Name the files.** A plan that says "update the domain layer" is not a
  plan; it names the paths and the symbols.
- **Plan the test first.** The plan starts from the failing test that proves
  the behaviour, not from the implementation.
- **Respect the pins.** Gradle 9.6.0, AGP 9.5.1, Kotlin 2.4.21 and Compose
  BOM 2026.09.00 are the toolchain; a plan that needs a newer version says so
  explicitly and names the reason.
- **Write it down, then stop.** The plan is a decision record, not an
  invitation to start coding in the same breath.

## Rules

- No fabricated paths, symbols, commands or test results: every referenced
  path exists in the tree today, or the plan says it will be created.
- Every plan states what is **out of scope**; scope creep lives in the
  unstated part.
- The plan respects the module map: no `android.*` import in
  `:core:kernel` or `:feature:notes:domain`, no `:feature:notes:data`
  import in `:feature:notes:ui`, and no cross-slice import that bypasses
  `:feature:notes:contracts`.
- The plan names the gate that proves each step — `./gradlew
  :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test` or `bash
  scripts/check-architecture.sh` — and never plans around a failing gate.
- A new dependency needs a reason and lands in `gradle/libs.versions.toml`,
  never in a module build file.
- The file is written to `docs/plans/<slug>.md`, one plan per change.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The layer map and the permitted dependency direction of a native Android client | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| Compose guidance separates UI state, UI events and one-shot effects | Android Developers — Compose architecture | 2026 | https://developer.android.com/develop/ui/compose/architecture |
| Navigation 3 routes are typed keys owned by the application | AndroidX Navigation release notes | 2026 | https://developer.android.com/jetpack/androidx/releases/navigation3 |
| The version catalog is the single source of dependency versions | Gradle documentation | 2026 | https://docs.gradle.org/current/userguide/platforms.html |
| Coroutines must be structured and cancellable | Kotlin documentation | 2026 | https://kotlinlang.org/docs/coroutines-basics.html |

## Output Format

The plan file `docs/plans/<slug>.md`:

```
# <title>

## Problem
<what is wrong or missing, and for whom>

## Constraints
<the rules that bound the solution: module map, pins, gates, no new dependency>

## Design
<per module: what changes and why>

## Files to touch
- <path> — <change>

## Test plan
- <suite> — <case> — <expected failure first, then pass>

## Risks
- <risk> — <mitigation>

## Out of scope
<explicitly excluded>

## Steps
1. <step> — gate: <command>
```

And the response reports the plan path plus the open questions, if any.
