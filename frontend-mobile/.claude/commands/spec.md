---
name: spec
description: "Delegate to the spec skill — turn a <app> mobile feature idea into a scoped spec with observable acceptance criteria, the layers it touches and a redaction scan."
division: core
---

# /spec

## Delegate

Delegate to the **spec** skill. Activate when a `<app>` mobile feature idea,
bug report or request must become a written, reviewable spec before any code
is written.

Steps:

1. Interrogate the request: the problem, the motivation, the scope boundary,
   the acceptance criteria and the invariants it touches. Nothing is
   invented.
2. Name the layers the feature touches and the port it needs, if any.
3. Run the redaction scan over the full draft. A hit stops the workflow and
   asks for a redacted version; it never masks and continues.
4. Write the spec to `docs/specs/<slug>.md` and report the path, the
   acceptance criteria and the open questions.
