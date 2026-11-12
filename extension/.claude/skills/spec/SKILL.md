---
name: "<app> Spec"
description: "Files a scoped, redaction-scanned issue for the <app> extension — the user-visible behaviour, the acceptance criteria, the non-goals and the open questions — before any plan or code exists."
division: core
---

# <app> Spec

A reusable workflow for writing the specification of a change in the `<app>` Chrome extension. The spec states the behaviour the user will observe, the acceptance criteria that will be tested, and the non-goals that keep the change small. It is written before the plan and long before the code, and it is deliberately free of implementation detail: a spec that names files and functions is a plan wearing a spec's clothes.

## When to Activate

- A request arrives with more than one reading, or an unclear boundary.
- A change touches a user-visible surface — the side panel, the options page, the content script.
- A new capability, permission, message type or storage record is proposed.
- A reviewer asks what exactly is being built before the plan is written.

Do not activate for a well-specified bug fix with an unambiguous expected behaviour — the failing test is the specification in that case.

## Core Principles

- Behaviour, not implementation. The spec describes what the user sees and what the extension guarantees, not which file changes.
- Acceptance criteria are testable. Each one names an observable outcome that a test can assert; "works well" is not a criterion.
- Non-goals are mandatory. The list of what this change deliberately does not do is what keeps the diff small and the review short.
- The permission and privacy surface is part of the spec. If the change touches what the extension can read, store or send, that is stated in the spec, not discovered in review.
- Open questions are surfaced, not guessed. A question the requester must answer is written as a question, and the plan waits for the answer.

## Rules

- Anti-hallucination: every claim about current behaviour cites the file that was read; nothing is asserted about the existing code that was not verified.
- Every claim needs a source: cite the local file for a repository behaviour and the official Chrome documentation for a platform constraint.
- Do not emit the final spec as a fenced code block when raw output is expected — the spec is plain Markdown text.
- Never write a real secret, token, cookie value, host-specific customer data or personal identifier into a spec. Describe the shape, not the value.
- Every acceptance criterion maps to exactly one test tier: `unit`, `viewmodel`, `contract` or `platform`.
- A spec with no non-goals is incomplete; a spec with no open questions must state that there are none.
- No implementation nouns in the acceptance criteria: no file names, no function names, no class names.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| The extension's surfaces are the side panel, the options page, the service worker and the content script | `PLAYBOOK_CHROME_EXTENSION.md` | 2024 | local file |
| The permission posture the spec must respect: storage, sidePanel, alarms, no host permissions | `src/manifest.ts` | 2024 | local file |
| The content script runs in an isolated world and its reach is bounded by its match patterns | Chrome extension content scripts documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts |
| The user-facing copy lives in the locale files | `public/_locales/en/messages.json` | 2024 | local file |
| The test tiers the acceptance criteria map to | `PLAYBOOK_TESTS.md` | 2024 | local file |
| The protocol version and the storage version that bound a spec | `src/contracts/protocol.ts`, `src/storage/notes-store.ts` | 2024 | local file |

## Output Format

The spec document, written to `docs/specs/<slug>.md` (the directory is created on demand). Plain Markdown — not a fenced block:

```
# Spec — <title>

## Status
draft | ready-for-plan | ready-for-implementation

## Context
What exists today, with the file that proves each claim.

## Behaviour
The user-visible outcome, described without naming a file or a function.

## Acceptance criteria
1. <observable outcome>
   Tier: unit | viewmodel | contract | platform
2. ...

## Non-goals
- <what this change deliberately does not do>

## Permission and data surface
What the extension reads, stores and sends as a result of this change, and
whether the manifest posture changes. If it does, the register entry is
named as a prerequisite.

## Open questions
- <question> — needs an answer from: <requester>

## Risks
What could make this spec wrong, and what would reveal it early.
```

Summary line: `spec: <title> — N criteria, N non-goals, N open question(s) — verdict: READY | NEEDS-CLARIFICATION`.
