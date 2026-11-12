---
name: "plan"
description: "Produces a written implementation plan for a scoped <app> change: the problem, the constraints, the design per layer, the files to touch, the risks and the test plan, saved under docs/plans/."
division: core
---

# `<app>` plan skill

> Reusable workflow for turning a scoped change into a written plan before any
> code is written. An agent invokes this skill; the requester supplies the
> change, the skill supplies the interrogation, the structure and the file.

## When to Activate

Activate this skill when a change to `<app>` is large enough to warrant a
written plan before implementation: a new slice, a cross-layer change, a
migration, a packaging change, or a request whose acceptance criteria are
still fuzzy.

Do NOT activate this skill when:

- The change is a single-file fix with an obvious location — implement it with
  a red test first.
- A plan already exists for the same change; update it instead of writing a
  second one.
- The request is to cut a release (use `ship`) or to file an issue (use
  `spec`).

## Core Principles

- **Interrogate before writing.** The problem, the motivation, the scope
  boundary and the acceptance criteria are elicited and written down; an
  unstated assumption is a defect in the plan.
- **Design per layer.** Each layer that changes says what it gains, and the
  dependency direction stays legal: domain → application → adapters →
  presentation → app.
- **Name the files.** A plan that says "update the storage layer" is not a
  plan; it names the paths and the symbols.
- **Plan the test first.** The plan starts from the failing test that will
  prove the behaviour, not from the implementation.
- **Write it down, then stop.** The plan is a decision record, not an
  invitation to start coding in the same breath.

## Rules

- No fabricated paths, symbols, commands or test results: every referenced
  path exists in the tree today, or the plan says it will be created.
- Every plan states what is **out of scope**; scope creep lives in the
  unstated part.
- The plan respects the layer law: no GUI type in the domain or the
  application layer, no OS header outside `src/platform/`.
- Every risk is named with its mitigation, and every unknown with the
  question that would resolve it.
- The plan names the gate that proves each step, and never plans around a
  failing gate.
- The file is written to `docs/plans/<slug>.md`, one plan per change.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The layer map and the permitted dependency direction | `PLAYBOOK_DESKTOP_QT.md` §3 | - | `PLAYBOOK_DESKTOP_QT.md` |
| The workflow order — spec, plan, red test, implementation, verification | `PLAYBOOK_CLAUDE_CODE.md` | - | `PLAYBOOK_CLAUDE_CODE.md` |
| Per-target dependencies make a layer violation a link error | CMake documentation | 2025 | https://cmake.org/cmake/help/latest/command/target_link_libraries.html |
| CMake presets are the supported way to configure the project | CMake documentation | 2025 | https://cmake.org/cmake/help/latest/manual/cmake-presets.7.html |

## Output Format

The plan file `docs/plans/<slug>.md`:

```
# <title>

## Problem
<what is wrong or missing, and for whom>

## Constraints
<the rules that bound the solution: layer law, gates, no new dependency>

## Design
<per layer: what changes and why>

## Files to touch
- <path> — <change>
- <path> — <change>

## Test plan
- <suite> — <case> — <expected failure first, then pass>

## Risks
- <risk> — <mitigation>

## Out of scope
- <explicitly excluded>

## Steps
1. <step> — gate: <command>
```

And the response reports the plan path plus the open questions, if any.
