---
name: ship
description: "Delegate to the ship skill — run the gate ladder and the release checks of the <app> Android app (R8, signing, baseline profile, 16 KB alignment, target SDK, merged manifest) and report the artifacts."
division: core
---

# /ship

## Delegate

Delegate to the **ship** skill. Activate when a change in `<app>` is ready to
be released or handed over.

Steps:

1. Run the gate ladder and quote the real output: `./gradlew
   :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test`, `bash
   scripts/check-architecture.sh`. A failing gate blocks the release; fix the
   cause, never the check.
2. Build the release variant (`./gradlew :app:assembleRelease`) and verify the
   release configuration: R8 full mode, resource shrinking, and signing
   presence — the signing material itself is never printed or copied.
3. Check the release surface: the merged manifest of the release variant with
   its exported components and permissions, the target SDK level against the
   store requirement, and the build-type field `<APP>_ENV` in `BuildConfig`
   that tells the shipped binary which environment it is.
4. Check the native surface: baseline profile freshness and the 16 KB
   page-size alignment of every shipped native library.
5. Report the artifacts, the results and — explicitly — what this environment
   could not verify.
