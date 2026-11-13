---
name: "plan"
description: "Produces a written implementation plan for a scoped <app> mobile change: the problem, the constraints, the design per layer, the files to touch, the risks and the test plan, saved under docs/plans/."
division: core
---

# `<app>` plan skill

> Reusable workflow for turning a scoped change into a written plan before any
> code is written. An agent invokes this skill; the requester supplies the
> change, the skill supplies the interrogation, the structure and the file.

## When to Activate

Activate this skill when a change to `<app>` is large enough to warrant a
written plan before implementation: a new slice, a cross-layer change, a new
device capability behind a port, a route change, a dependency-pin change, or
a request whose acceptance criteria are still fuzzy.

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
  dependency direction stays legal: domain → application → `adapters/out` →
  view models → views → app.
- **The device stays behind a port.** A new capability is an interface over
  plain values plus one adapter that names the `expo-*` package; never a
  device call in the domain or a view.
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
- The plan respects the layer law: no `react-native` or `expo-*` import in
  `packages/core/src`, `packages/contracts/src`, the domain, the application
  layer or a view model.
- The plan names the gate that proves each step, and never plans around a
  failing gate.
- The plan does not move the pins: React `19.3.0`, React Native `0.88.1`,
  Expo `~58.0.24` and TypeScript `~6.0.3` stay as they are unless the request
  is explicitly about a dependency change.
- The file is written to `docs/plans/<slug>.md`, one plan per change.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The layers, the permitted dependency direction and the device-as-adapter rule | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1 | 2026 | local file |
| The mobile ports and their plain-value signatures | `packages/core/src/ports.ts` | 2026 | local file |
| The route grammar and the typed route table | `packages/contracts/src/routes.ts` | 2026 | local file |
| The gate commands and what each one proves | `package.json`, `Makefile` | 2026 | local file |
| The enforced architecture rules | `eslint.config.mjs`, `tests/_platform/architecture/import-boundaries.test.ts` | 2026 | local file |
| The workspace layout and the monorepo rules of an Expo project | Expo monorepos guide | 2026 | https://docs.expo.dev/guides/monorepos/ |

## Output Format

The plan file `docs/plans/<slug>.md`:

```
# <title>

## Problem
<what is wrong or missing, and for whom>

## Constraints
<the rules that bound the solution: layer law, gates, pins, no new dependency>

## Design
<per layer: what changes and why>

## Files to touch
- <path> — <change>

## Test plan
- <suite> — <case> — <expected failure first, then pass>

## Risks
- <risk> — <mitigation>

## Out of scope
<explicitly excluded>

## Steps
1. <step> — gate: <command>
```

And the response reports the plan path plus the open questions, if any.
