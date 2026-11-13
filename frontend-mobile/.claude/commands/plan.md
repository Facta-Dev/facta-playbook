---
name: plan
description: "Delegate to the plan skill — write the implementation plan for a scoped <app> mobile change: the problem, the constraints, the design per layer, the files to touch, the risks and the test plan."
division: core
---

# /plan

## Delegate

Delegate to the **plan** skill. Activate when a `<app>` mobile change needs a
written plan before any code is written — a new slice, a cross-layer change, a
new device capability, a route change or a request whose acceptance criteria
are still fuzzy.

Steps:

1. Read the request and the code it touches; list the layers the change
   touches (domain, application, `adapters/out`, view models, views, app).
2. Elicit the problem, the motivation, the scope boundary and the acceptance
   criteria; write down every assumption instead of filling a gap.
3. Design per layer, keeping the device behind a port and the dependency
   direction legal.
4. Name the files, the first failing test and the gate command that proves
   each step.
5. Write the plan to `docs/plans/<slug>.md` and report the path plus the open
   questions. Stop there — the plan is a decision record, not the start of
   the implementation.
