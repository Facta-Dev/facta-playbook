---
name: spec-tasks
description: Produce tasks.md — ordered, test-first, AC-referenced tasks with acceptance checks. Use after spec-plan, before the TDD loop.
when_to_use: "break the plan into tasks", "/spec-tasks", "tasks for specs/<slug>"
argument-hint: <slug>
disable-model-invocation: true
---

# Produce tasks

Resolve the template: `specs/_templates/tasks.md`, then
`~/.claude/templates/sdd/tasks.md`.

## Procedure

1. Read `spec.md`, `clarify.md`, and `plan.md`. Require `status: planned`;
   otherwise stop and say which step is missing.
2. Write one row per unit of work in `specs/<slug>/tasks.md`:
   `ID | AC | Test (path) | Implementation target | Depends | [P] | Status | Session`.
3. Rules per task:
   - references at least one AC-;
   - names the test file that will fail first and the implementation target;
   - small enough for one TDD cycle (~≤ 1 hour);
   - ordered by dependency; `[P]` only when files are disjoint.
4. Verify coverage of intent: every AC- appears in at least one task.
5. Report the count and any AC that ended up without a task.

## Testability gate

- Every AC- maps to a test level (unit/integration/contract/e2e/security) and
  a concrete file path.

## Done when

- `tasks.md` covers 100% of acceptance criteria, tasks are test-first and
  ordered, and `[P]` flags are set only for disjoint work.
