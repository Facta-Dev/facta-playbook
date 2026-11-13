---
name: "spec"
description: "Writes the behaviour spec for a <app> iOS change — acceptance criteria, non-goals, route and capability surface, open questions — before any implementation."
division: core
---

# `<app>` spec skill

> The spec is the contract the tests are written against. This skill writes it
> before the first line of code.

## When to Activate

Activate when a feature, a behaviour change or a platform capability needs a
shared definition of done: what the user sees, what the app does, what it
deliberately does not do.

Do NOT activate when:

- The change is a one-line fix with an obvious acceptance criterion.
- The request is the implementation plan (use `plan`).

## Core Principles

- **Behaviour, not implementation.** The spec says what the user observes and
  what the system guarantees; it does not name a package unless the change is
  structural.
- **Acceptance criteria are testable.** Each criterion is a sentence a test can
  assert, with the input and the expected result.
- **Non-goals are written.** The scope the request does not include is as
  binding as the scope it does.
- **The surface is named.** Routes, deep links, `Info.plist` keys,
  entitlements, push categories and permissions the change adds or changes.
- **Open questions are blocking.** A spec with a silent assumption is rejected.

## Rules

- The spec lives at `docs/specs/<slug>.md`.
- Every acceptance criterion has an ID (`AC-1`) a test name can cite.
- The route and capability surface section says "none" explicitly when the
  change adds none.
- Error and empty states are part of the behaviour: every new screen names its
  `UiStatus` coverage.
- No secret, token, customer name or personal data enters the spec.
- The spec is reviewed by `review` or the human before `plan` starts.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Spec-driven development is the repo's workflow | `PLAYBOOK_CLAUDE_CODE.md` | - | `../PLAYBOOK_CLAUDE_CODE.md` |
| Acceptance criteria are the test contract | `PLAYBOOK_TESTS.md` | - | `../PLAYBOOK_TESTS.md` |
| Routes and deep links follow the URL grammar | `PLAYBOOK_URLS.md` | - | `../PLAYBOOK_URLS.md` |

## Output Format

```
Spec: docs/specs/<slug>.md
Behaviour: <one paragraph>
Acceptance criteria:
  AC-1 <input> -> <expected observable result>
  ...
Non-goals: <what is out of scope>
Surface: routes <...> · capabilities <Info.plist/entitlement keys or none> · status coverage <...>
Open questions: <question> — <blocking | answered>
```
