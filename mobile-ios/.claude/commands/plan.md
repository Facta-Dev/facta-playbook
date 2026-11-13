---
name: plan
description: "Delegate to the plan skill — write the implementation plan for a <app> iOS change: packages, files, test-first sequence, risks, gates and rollback, from an approved spec."
division: core
---

# /plan

## Delegate

Delegate to the **plan** skill. Activate when a spec exists and the change is
non-trivial: more than one package, a route, a capability, a migration or a
concurrency-sensitive path.

Steps:

1. Read the approved spec at `docs/specs/<slug>.md`; finish `spec` first when
   it has an open blocking question.
2. Order the phases test-first: the failing test is a step and comes before
   the implementation it pins; each phase ends with the command that must be
   green.
3. Name every file, marking the ones the step creates; keep one package per
   step where possible.
4. Give a package dependency change its own step, naming the `Package.swift`
   edge.
5. Give a capability change (`Info.plist`, entitlement, privacy manifest) its
   own step with the review note.
6. Write the risks with mitigations and the rollback before the first edit.
7. Write the plan to `docs/plans/<slug>/plan.md` and list what it deliberately
   does not do.
