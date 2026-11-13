---
name: "spec"
description: "Turns a <app> mobile feature idea into a scoped spec: interrogates the request, states the acceptance criteria and the invariants it touches, redacts secrets, and writes docs/specs/<slug>.md."
division: core
---

# `<app>` spec skill

> Reusable workflow for turning a feature idea into a scoped, reviewable spec
> before any code is written. An agent invokes this skill; the requester
> supplies the idea, the skill supplies the interrogation, the redaction scan
> and the file.

## When to Activate

Activate this skill when the request is to turn an idea, a bug report or a
feature request for `<app>` into a written spec: elicit the problem, draft
the spec, redact it, and file it under `docs/specs/`.

Do NOT activate this skill when:

- The request already has a spec or a plan to work against — implement it.
- The request is to cut a release (use `ship`).
- The draft would need a real secret to make sense (a token, a private URL, a
  password) — stop and ask for a redacted version instead of drafting around
  it.

## Core Principles

- **Interrogate before drafting.** The problem statement, the motivation, the
  scope boundary, the acceptance criteria and the invariants touched are
  elicited; nothing is invented to fill a gap.
- **Redaction is fail-closed.** The scan runs on the **full** draft before
  anything is written or shared. A hit stops the flow and asks for a fix; it
  never masks and continues.
- **One spec per idea.** If the draft overlaps an existing spec, both are
  shown and the work stops for a decision.
- **The spec names the layers.** A mobile change is not a UI change: the spec
  says which layer owns the rule, which port carries the capability and which
  layer renders it.
- **Acceptance criteria are observable.** "The list shows an empty state" is
  testable; "the list feels better" is not.

## Rules

- No fabricated acceptance criteria, invariants, file paths or test results:
  if the requester did not state it and the codebase does not support it,
  ask.
- The redaction scan checks the whole draft (title and body) for private-key
  blocks (`-----BEGIN`), JWT-shaped tokens (`eyJ…`), cloud access keys
  (`AKIA[0-9A-Z]{16}`), `ghp_`/`gho_`/`github_pat_` tokens, bearer and
  authorization header values, `password=`/`secret=`/`token=` assignments,
  e-mail addresses, IPv4 addresses and `.env`-style `KEY=value` secrets.
- A redaction hit stops the workflow: the offending lines are printed masked
  and an explicit re-run is required. Never auto-continue.
- No secret, token or connection string is written into the spec file, even
  in redacted form — reference the file and line where the issue lives.
- The spec keeps to one feature. Two ideas become two specs.
- The spec file is written to `docs/specs/<slug>.md`.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| A mobile feature spans layers; the domain owns the rule and the view renders it | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1 | 2026 | local file |
| The client contract the statuses and the view-model state come from | `packages/feature-notes/src/viewmodels/notes-view-model.ts` | 2026 | local file |
| The eight statuses a view must render | `docs/03-hexagonal-layers-and-mvvm.md` | 2026 | local file |
| Secrets never enter the repository, the logs or the settings files | `PLAYBOOK_SECURITY.md` | 2026 | local file |
| The acceptance criteria of a spec become the red tests of the change | `PLAYBOOK_TESTS.md` | 2026 | local file |
| The order of the workflow: spec, plan, test, code | `PLAYBOOK_CLAUDE_CODE.md` | 2026 | local file |
| The mobile threat model the spec is written against | OWASP Mobile Application Security Verification Standard | 2026 | https://mas.owasp.org/MASVS/ |

## Output Format

```
spec: <title>
File: docs/specs/<slug>.md

Problem: <two sentences>
Motivation: <one paragraph>
Scope: <in scope>
Out of scope: <excluded>
Layers touched: <layer> — <what it gains>
Ports: <port> — <signature, or none>
Acceptance criteria:
  1. <observable criterion>
  2. <observable criterion>
Invariants touched: <invariant> — <why>
Redaction scan: PASS — no hits
Existing overlap: <none, or the candidate spec path>
Open questions: <question, or none>
```
