---
name: add-module
description: "Delegate to the add-module skill — add a Gradle module to the <app> Android build: the build file, the namespace under dev.<scope>.<app>, the version catalog entries, the settings.gradle.kts registration and the architecture rule."
division: core
---

# /add-module

## Delegate

Delegate to the **add-module** skill. Activate when the request is a new
Gradle module in the `<app>` build — a shared `:core:<name>` module or a new
feature family — rather than a slice inside an existing family.

Steps:

1. State the module in one sentence: what it owns and what is allowed to
   depend on it. A module without that sentence is a package, not a module.
2. Create the module build file: the plugin (the plain Kotlin plugin when the
   module carries no Android type), the namespace rooted at `dev.<scope>.<app>`,
   and the dependencies taken from `gradle/libs.versions.toml` — never a
   version written inline.
3. Register the module in `settings.gradle.kts` and add the catalog entries it
   needs, keeping the pinned toolchain (Gradle 9.6.0, AGP 9.5.1, Kotlin
   2.4.21, Compose BOM 2026.09.00) unchanged.
4. Extend `scripts/check-architecture.sh` with the rule the new module needs,
   so the boundary it creates is enforced by the build and not by habit.
5. Add the module's first test, then run `./gradlew :app:assembleDebug`,
   `./gradlew test`, `./gradlew :evals:test` and `bash
   scripts/check-architecture.sh`, and report the real output.
