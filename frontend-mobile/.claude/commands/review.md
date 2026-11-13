---
name: review
description: "Delegate to the review skill — review a <app> mobile change against the layer law, the Result contract, the eight statuses, the route grammar, the secret policy and the gate results."
division: core
---

# /review

## Delegate

Delegate to the **review** skill. Activate when a `<app>` mobile change is
ready to land and must be checked against the mobile rules before it ships.

Steps:

1. Read the whole diff; a file that was not read is reported as not
   reviewed.
2. Run the checks in order: layer law and platform confinement, the `Result`
   contract, the eight view-model statuses, the route grammar, secrets and
   device data, the low-tier performance posture, tests, style.
3. Run the narrowest gate that covers the change — `pnpm run typecheck`,
   `pnpm run lint`, `pnpm run test`, `pnpm run export:android` — and report
   the real output.
4. Report one line per finding with the severity, the file, the line and the
   failure scenario; end with `review: N findings — verdict: APPROVE | REVISE`.
