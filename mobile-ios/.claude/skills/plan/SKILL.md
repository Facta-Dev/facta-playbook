---
name: "plan"
description: "Writes the implementation plan for a <app> iOS change — packages, files, test-first sequence, risks, gates and rollback — from an approved spec."
division: core
---

# `<app>` plan skill

> The plan turns an approved spec into ordered, verifiable steps. This skill
> writes it; the next session executes it without another design decision.

## When to Activate

Activate when a spec exists and the change is non-trivial: more than one
package, a route, a capability, a migration, or a concurrency-sensitive path.

Do NOT activate when:

- The change is a one-file fix covered by an existing test.
- The spec is still open (finish `spec` first).

## Core Principles

- **Test-first order.** The failing test is a step, and it comes before the
  implementation it pins.
- **One package per step where possible.** A step that touches four packages
  is a plan that hides a design decision.
- **Real paths.** Every file is named; a file that does not exist yet is
  marked as created by that step.
- **The gate proves each phase.** Each phase ends with the command that must
  be green before the next begins.
- **Rollback is written before the first edit.** What to revert, and what
  state survives.

## Rules

- The plan lives at `docs/plans/<slug>/plan.md` (or `docs/plans/<slug>.md`
  when it is one file).
- Every step names its test file and its gate.
- A package dependency change is its own step, with the `Package.swift` edge
  named.
- A capability change (`Info.plist`, entitlement, privacy manifest) is its own
  step, with the review note.
- Open questions are answered or marked blocking.
- The plan lists what it deliberately does not do.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Plan artifacts and their location | `PLAYBOOK_CLAUDE_CODE.md` | - | `../PLAYBOOK_CLAUDE_CODE.md` |
| The test tiers a plan must cover | `PLAYBOOK_TESTS.md` | - | `../PLAYBOOK_TESTS.md` |
| The package graph the plan must respect | `PLAYBOOK_MOBILE_IOS_SWIFT.md` §4 | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |

## Output Format

```
Plan: docs/plans/<slug>/plan.md
Spec: docs/specs/<slug>.md
Phases:
  1. <test file> — <failing assertion> — gate: <command>
  2. <implementation file> — <symbol> — gate: <command>
  ...
Packages: <package> — <new/changed edge>
Capabilities: <Info.plist / entitlement / privacy manifest — or none>
Risks: <risk> — <mitigation>
Rollback: <what to revert>
Open: <question — blocking>
```
