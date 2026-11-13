---
name: plan
description: "Delegate to the plan skill — produce docs/plans/<slug>.md for a scoped <app> Android change: problem, constraints, design per Gradle module, files to touch, risks and test plan."
division: core
---

# /plan

## Delegate

Delegate to the **plan** skill. Activate when a change to `<app>` is large
enough to warrant a written plan before the implementation begins.

Steps:

1. Read the request and interrogate the gap: problem, motivation, scope
   boundary, acceptance criteria. An unstated assumption is a defect.
2. Map the design to the Gradle modules: `:core:kernel`, `:core:ui-core`,
   `:feature:notes:contracts`, `:feature:notes:domain`,
   `:feature:notes:data`, `:feature:notes:ui` and `:app`. Note what each one
   gains and keep every dependency inward.
3. List the files to touch with their real paths, and name the `UiStatus`
   values, `UiEvent`s, `UiEffect`s and `NavKey`s the change introduces.
4. Write the test plan first: the failing test, the suite it lives in
   (`:feature:notes:domain`, `:feature:notes:ui` or `:evals`), and the Gradle
   command that runs it.
5. Name the gate per step — `./gradlew :app:assembleDebug`, `./gradlew test`,
   `./gradlew :evals:test` or `bash scripts/check-architecture.sh` — and never
   plan around a failing gate.
6. Write the file to `docs/plans/<slug>.md`, state what is out of scope, and
   report the path plus the open questions.
