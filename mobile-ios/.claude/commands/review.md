---
name: review
description: "Delegate to the review skill — review a <app> iOS diff through the Swift/concurrency, package-boundary, Apple-platform and security lenses, and emit severity-prefixed findings with the real gate output."
division: core
---

# /review

## Delegate

Delegate to the **review** skill. Activate when a change in `<app>` is ready
for a second pair of eyes.

Steps:

1. Read the whole diff before writing anything; a partial read produces
   findings about code a later hunk already fixes.
2. Apply the four lenses in order: Swift and concurrency correctness (Swift 6
   mode, `Sendable`, `MainActor` isolation), the package map and the UDF
   contract (state down, events up, effects as a stream, view purity), the
   Apple platform surface (lifecycle, capabilities, privacy manifest,
   background work), and the security posture.
3. Anchor every finding to a `file:line` and state the concrete failure and
   the symptom the user would see. Drop any finding that cannot be
   reproduced.
4. Run the gates and capture the real output: `bash
   scripts/check-architecture.sh`, `node --test evals/contracts.test.mjs`,
   `make ci`; report the native tier as run or SKIP with the reason.
5. Emit the report: findings by severity, the lenses that produced none, the
   gate results and the verdict `APPROVE` or `REVISE`.
