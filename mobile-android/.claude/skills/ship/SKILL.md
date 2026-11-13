---
name: "ship"
description: "Cuts a release of the <app> Android app: runs the gate ladder, checks the release build configuration (R8, signing, baseline profile, 16 KB alignment), verifies the merged manifest and reports the artifacts."
division: core
---

# `<app>` ship skill

> Reusable workflow for taking a change from "green on my machine" to "the
> gates are green and the artifact is reproducible". An agent invokes this
> skill; the requester supplies the intent to release, the skill supplies the
> ladder and the checklist.

## When to Activate

Activate this skill when a change is ready to be released or handed over: the
user asks to ship, to cut a build, to prepare a release, or to check that the
tree is in a releasable state.

Do NOT activate this skill when:

- The change is not finished — a failing gate is a fix, not a release.
- The request is only to review the diff (use `review`).
- The request is to write the requirements (use `spec`).

## Core Principles

- **The gates are the definition of done.** `./gradlew :app:assembleDebug`,
  `./gradlew test`, `./gradlew :evals:test` and `bash
  scripts/check-architecture.sh` must pass, and the output is quoted.
- **The release build is a different artifact.** Debug green does not mean
  release green: R8 full mode, resource shrinking and signing must be checked
  on the release variant.
- **The manifest that ships is the one that is reviewed.** The merged manifest
  of the release variant, with its exported components and permissions.
- **Reproducibility over speed.** The pinned toolchain (Gradle 9.6.0,
  AGP 9.5.1, Kotlin 2.4.21, Compose BOM 2026.09.00) and the dependency
  verification file are part of the artifact.
- **Report what happened.** The summary lists the real results, the artifacts
  and what was skipped.

## Rules

- No fabricated build output, artifact name, size or test result: run the
  command or say it was not run.
- A failing gate blocks the release; the skill fixes the cause or reports the
  block, it never relaxes the check.
- Signing material is never printed, never committed and never uploaded; only
  its presence is verified.
- The release checklist covers the baseline profile, the 16 KB page-size
  alignment of native libraries and the target SDK level the store requires.
- Secrets, tokens and personal data never enter a build log or a release
  note.
- The release note states the user-visible change, the risk and the rollback
  path.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| R8 full mode and resource shrinking shrink the shipped artifact | Android Developers — Shrink code | 2026 | https://developer.android.com/build/shrink-code |
| A baseline profile improves startup on real devices | Android Developers — Baseline profiles | 2026 | https://developer.android.com/topic/performance/baselineprofiles/overview |
| Native libraries must be 16 KB page-size aligned | Android Developers — 16 KB page sizes | 2026 | https://developer.android.com/guide/practices/page-sizes |
| The target API level is a store requirement, not a preference | Google Play — Target API level | 2026 | https://developer.android.com/google/play/requirements/target-sdk |
| Dependency verification pins the exact artifacts of the build | Gradle documentation | 2026 | https://docs.gradle.org/current/userguide/dependency_verification.html |
| The release checklist of a native Android client | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §5.1 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |

## Output Format

```
Release: <app> — <variant> — <build type>
Gates:
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
  ./gradlew :app:assembleRelease -> <result>
Release checks:
  R8 / shrinking: <ok | finding>
  signing: <configured | missing> — <no secret printed>
  baseline profile: <ok | stale | missing>
  16 KB alignment: <ok | finding>
  target SDK: <level> — <store requirement ok | finding>
  merged manifest: <exported components and permissions>
Artifacts: <path or name>
Not verified: <what this environment could not check>
```
