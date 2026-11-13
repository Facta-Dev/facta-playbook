---
name: "add-module"
description: "Adds a new Gradle module to the <app> Android build — a shared :core module or a new feature family — with its build file, namespace, dependencies from the version catalog, settings.gradle.kts registration and the architecture-script rule that keeps it legal."
division: core
---

# `<app>` add-module skill

> Reusable workflow for adding one Gradle module to the `<app>` build. An
> agent invokes this skill; the requester supplies the module, the skill
> supplies the build wiring, the dependency direction and the gate ladder.

## When to Activate

Activate this skill when the request is a new **Gradle module**: a shared
`:core:<name>` module (a design system, a telemetry client, a device
capability layer), a new `:feature:<name>` family, a test-support module, or a
new `:evals` suite.

Do NOT activate this skill when:

- The request is a feature slice inside the existing `:feature:<name>:{contracts,domain,data,ui}`
  family — that is `add-slice`.
- The request is a single new file in an existing module.
- The request is a dependency bump — that is a version catalog edit, not a
  module.

## Core Principles

- **A module exists to enforce a boundary.** If the new module cannot be
  described by one sentence about what may depend on it, it is a package, not
  a module.
- **Dependencies are declared once.** Every external dependency comes from
  `gradle/libs.versions.toml`; every internal dependency from the version
  catalog's project entries.
- **The module graph stays inward.** `:app` depends on features; features
  depend on `:core:*`; `:core:kernel` depends on nothing of ours.
- **Registration is part of the change.** A module that is not in
  `settings.gradle.kts` is invisible, and one that is registered but not
  covered by `scripts/check-architecture.sh` is unguarded.
- **The pins do not move.** Gradle 9.6.0, AGP 9.5.1, Kotlin 2.4.21 and
  Compose BOM 2026.09.00 stay as they are unless the request explicitly
  changes them.

## Rules

- No fabricated paths or coordinates: the module path, the namespace and the
  build file path are written exactly as they will exist.
- The namespace of an Android module matches its package and is unique across
  the build.
- A pure-Kotlin module (`:core:kernel`, and any new module that carries no
  Android type) uses the plain Kotlin plugin, not the Android library plugin.
- No dependency is added without an entry in `gradle/libs.versions.toml`, and
  no version is written inline in a build file.
- The new module is registered in `settings.gradle.kts` and, when it changes
  what is allowed, in `scripts/check-architecture.sh`.
- A new module ships with at least one test and, when it is a feature module,
  with its `:feature:<name>:{contracts,domain,data,ui}` shape intact.
- The change is not done until `./gradlew :app:assembleDebug`, `./gradlew test`,
  `./gradlew :evals:test` and `bash scripts/check-architecture.sh` are green,
  and the real output is reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Multi-module apps keep the dependency direction enforceable by the build | Android Developers — Feature modules | 2026 | https://developer.android.com/guide/navigation/integrations/feature-modules |
| The module map and the permitted dependency direction of this template | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4.1 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| The version catalog is the single source of versions and project coordinates | Gradle documentation | 2026 | https://docs.gradle.org/current/userguide/platforms.html |
| A module's namespace identifies its generated `R` and `BuildConfig` classes | Android Developers — Configure the app module | 2026 | https://developer.android.com/build/configure-app-module |
| Dependency verification pins the artifacts of the build | Gradle documentation | 2026 | https://docs.gradle.org/current/userguide/dependency_verification.html |

## Output Format

```
Module: <path> — <one sentence: what it owns and what may depend on it>
Created:
  <path>/build.gradle.kts — <plugin, namespace, dependencies>
  <path>/src/... — <the first files>
Registered in:
  settings.gradle.kts — <include line>
  gradle/libs.versions.toml — <entries added>
  scripts/check-architecture.sh — <rule added or unchanged>
Depends on: <module dependencies, inward only>
Gates:
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
Open: <anything left, such as the first slice that will use it>
```
