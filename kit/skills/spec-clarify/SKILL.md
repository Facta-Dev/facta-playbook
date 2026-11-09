---
name: spec-clarify
description: Resolve every open question in a spec and pass the clarity gate. Use after spec-new, before planning.
when_to_use: "clarify the spec", "/spec-clarify", "resolve open questions in specs/<slug>"
argument-hint: <slug>
disable-model-invocation: true
---

# Clarify a spec

## Procedure

1. Read `specs/<slug>/spec.md`. If it is missing, stop and say so.
2. Collect every `[NEEDS CLARIFICATION]` item and every requirement without an
   acceptance criterion.
3. Ask the user focused questions — one batch, not a drip. Propose a default
   for each so a one-word answer is enough.
4. Apply the answers to the spec: rewrite requirements, add or fix AC- items,
   remove the resolved markers.
5. Append a `clarify.md` next to the spec with the question, answer, and date
   for each decision.
6. Set `status: clarified` only when the clarity gate passes.

## Clarity gate

- No `[NEEDS CLARIFICATION]` left.
- Every FR-/NFR- has at least one AC-.
- Non-goals are stated explicitly.

## Done when

- Gate passes, `clarify.md` records the decisions, status is `clarified`.
- If the gate cannot pass, leave status unchanged and list what is blocking.
