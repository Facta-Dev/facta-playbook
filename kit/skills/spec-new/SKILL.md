---
name: spec-new
description: Create the next spec folder from the SDD template and start it in draft status. Use when beginning new feature work, before any planning or code.
when_to_use: "new spec", "start a feature spec", "/spec-new <slug>", "document requirements for X"
argument-hint: <slug> [slice]
disable-model-invocation: true
---

# Create a spec

Resolve the template in this order, use the first that exists:

1. `specs/_templates/spec.md` in the current repo
2. `~/.claude/templates/sdd/spec.md`
3. The skeleton in [the template file](../../templates/sdd/spec.md)

## Procedure

1. Take the slug from the arguments; if missing, derive it from the request
   (kebab-case, short).
2. Pick the slice: the argument if given, else the directory that will own the
   change, else `core`.
3. Choose the next free ID: `<slice>-<NNNN>`, scanning existing `specs/` ids.
4. Create `specs/<slug>/spec.md` from the template with `id`, `title`,
   `slice`, `owner`, and `status: draft` filled.
5. Fill Context, Goals/Non-goals, Users, Requirements (FR-/NFR-), and
   Acceptance criteria (AC-) from what the user actually said. Do not invent
   requirements.
6. Ask at most **3** blocking questions. Record the rest as
   `- [NEEDS CLARIFICATION] …` under Open questions.

## Done when

- `specs/<slug>/spec.md` exists with `status: draft` and ≥ 1 acceptance
  criterion per requirement that is already known.
- Open questions list the unknowns and the 3 asked questions are answered or
  listed.

## Never

- No implementation code in a spec.
- Never set status past `draft` here.
