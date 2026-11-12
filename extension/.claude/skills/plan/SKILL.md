---
name: "<app> Plan"
description: "Writes the <app> plan document under docs/plans/ — problem, constraints, design, files, risks, tests and gate commands — as an ordered, verifiable sequence that ends at `make ci`."
division: core
---

# <app> Plan

A reusable workflow for producing the plan document of a change in the `<app>` Chrome extension. The plan is a sequence of gate-checked steps, not a wish list: each step names the files it touches, the order it must land in, and the command that proves it. The output is the plan document itself plus the ordered step list.

## When to Activate

- A feature request needs to be broken into ordered steps before any code is written.
- A change touches more than one layer — a slice, the contracts, the platform or the storage.
- A change adds a permission, a message type or a storage version, and the ordering matters.
- A reviewer asks "what is the order here?" before approving the work.

Do not activate for a single-file, single-layer change that one engineer can sequence in their head — the plan for that is the diff.

## Core Principles

- Every step is verifiable. A step without a named gate command is not a step; "test it" is not a command.
- Order is part of the plan. A schema lands before the code that parses it, a migration lands before the reader, a protocol entry lands before the handler that depends on it.
- The blast radius is stated up front: which layers are touched, which are deliberately not, and what the permission and storage consequences are.
- The smallest correct change. The plan extends the existing seams — the existing port, the existing slice, the existing contract — instead of introducing a new package, port or dependency.
- The plan ends at `make ci`. Typecheck, lint, test and build are the definition of done; a plan that stops earlier is incomplete.

## Rules

- Anti-hallucination: every file path in the plan must exist or be created by a named step; never cite a path that was not read.
- Every claim needs a source: cite the local file or playbook for a repository rule; cite the official Chrome documentation for a platform behaviour.
- Do not emit the final plan as a fenced code block when raw output is expected — the document is plain Markdown text.
- Each step names exactly one gate command, so a failure points at the step that caused it.
- A plan never proposes a `chrome.*` call outside `src/platform/`, never a cross-slice import, and never a new permission without the register entry as an earlier step.
- A plan that changes a durable record includes the version bump, the migration and the fixture of the previous version as explicit steps.
- Do not schedule work the request does not need: no speculative refactors, no future-proofing, no second abstraction.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| The service worker is torn down when idle, so durable state must survive a restart | Chrome extension service worker basics | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/basics |
| `chrome.alarms` persists across restarts and is the right primitive for periodic work | `chrome.alarms` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/alarms |
| The Vite build emits the extension bundle and the manifest | `vite.config.ts` | 2024 | local file |
| The layer rules and the dependency direction | `PLAYBOOK_CHROME_EXTENSION.md` | 2024 | local file |
| The gate commands and what each one proves | `Makefile`, `package.json` | 2024 | local file |
| The enforced architecture rules | `dependency-cruiser.config.cjs`, `eslint.config.mjs` | 2024 | local file |

## Output Format

The plan document, written to `docs/plans/<slug>.md` (the directory is created on demand), then the ordered step list. Plain Markdown — not a fenced block:

```
# Plan — <title>

## Problem
One paragraph: what is missing today and what changes for the user.

## Constraints
The rules that bound the solution: the layer boundaries, the permission
posture, the protocol version, the storage version, the gates.

## Design
The chosen approach and the alternatives rejected, with the reason.

## Files
The files created and edited, with one line each.

## Risks
What can go wrong, how it is detected, and what happens if it does.

## Tests
The tests to write, per tier, and the red-first order.

## Steps
1. <file> — <what changes>
   Gate: <command>
2. ...

## Definition of done
make ci green; pnpm run evals green; docs updated where the public
surface changed.
```

Summary line: `plan: <slug> — N step(s), M risk(s) — verdict: READY | NEEDS-CLARIFICATION`.
