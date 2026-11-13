---
name: review
description: "Delegate to the review skill — review a <app> Android diff through the correctness, module-boundary, platform and security lenses, and emit severity-prefixed findings with the real gate output."
division: core
---

# /review

## Delegate

Delegate to the **review** skill. Activate when a change in `<app>` is ready
for a second pair of eyes.

Steps:

1. Read the whole diff before writing anything; a partial read produces
   findings about code a later hunk already fixes.
2. Apply the four lenses in order: Kotlin and coroutine correctness, the
   module map and the UDF contract (`UiState` down, `UiEvent` up, one-shot
   `UiEffect`), the Android platform surface (lifecycle, permissions,
   manifest, background work), and the security posture.
3. Anchor every finding to a `file:line` and state the concrete failure and
   the symptom the user would see. Drop any finding that cannot be
   reproduced.
4. Run the gates and capture the real output: `./gradlew :app:assembleDebug`,
   `./gradlew test`, `./gradlew :evals:test`, `bash
   scripts/check-architecture.sh`.
5. Emit the report: findings by severity, the lenses that produced none, the
   gate results and the verdict `APPROVE` or `REVISE`.
