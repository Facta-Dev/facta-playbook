---
name: spec-plan
description: Write plan.md for a clarified spec — architecture per layer, data model, contracts, test strategy, risks. Use between spec-clarify and spec-tasks.
when_to_use: "plan the spec", "/spec-plan", "design specs/<slug> before implementation"
argument-hint: <slug>
disable-model-invocation: true
---

# Plan a spec

Resolve the template: `specs/_templates/plan.md`, then
`~/.claude/templates/sdd/plan.md`.

## Procedure

1. Read `specs/<slug>/spec.md` and `clarify.md`. Require `status: clarified`;
   otherwise stop and tell the user to run `/spec-clarify`.
2. Read the surrounding code that the change touches before designing — the
   plan must reuse existing patterns, not invent parallel ones.
3. Write `specs/<slug>/plan.md` from the template:
   - architecture per layer (domain / application / adapters);
   - ports and adapters the change needs;
   - data model plus expand/contract migration steps;
   - contract changes and backward compatibility;
   - test strategy matrix: every AC mapped to a level and a test path;
   - rollout and feature flags;
   - risks and mitigations;
   - constitution check with a justification for every deviation.
4. Set `status: planned` in the spec only when every AC has a planned level.

## Constitution gate

- Plan respects the project's architecture, security, API/URL conventions,
  test strategy, and compliance mapping.
- Deviations are listed with reasons, not silently allowed.

## Never

- Do not write code or create task files here; that is `/spec-tasks`.
- Do not plan around a requirement you had to reinterpret — change the spec.
