---
name: "spec"
description: "Turns a request for the <app> Android app into a written specification: the user-visible behaviour, the acceptance criteria, the state vocabulary (UiStatus/UiEvent/UiEffect), the route table entry and the out-of-scope list."
division: core
---

# `<app>` spec skill

> Reusable workflow for stating what the `<app>` Android app must do before
> anything is built. An agent invokes this skill; the requester supplies the
> request, the skill supplies the questions, the structure and the file.

## When to Activate

Activate this skill when a request is too vague to implement directly: a new
feature described in one sentence, a change whose acceptance criteria are
unwritten, a behaviour that touches more than one screen, or a request whose
scope is still being negotiated.

Do NOT activate this skill when:

- The request is already written down as a plan (use `plan`).
- The change is a defect with a reproducible symptom — fix it with a red test
  first.
- The request is a release step (use `ship`).

## Core Principles

- **Write the behaviour, not the implementation.** The spec says what the user
  sees and what the app guarantees; it does not name classes.
- **Pin the state vocabulary.** Every screen states its `UiStatus` set, the
  `UiEvent`s it accepts and the `UiEffect`s it can emit, using the shared
  vocabulary of `:core:ui-core`.
- **State the route.** If the behaviour has a screen, the spec names its
  Navigation 3 `NavKey` and where it sits in the back stack.
- **State the data boundary.** Which data comes from the server (the
  authority), which comes from the local cache, and which is device-local.
- **Name the acceptance criteria as observable facts.** A criterion a tester
  can check without reading the code.
- **Say what is out of scope.** The unstated part is where scope creep lives.

## Rules

- No fabricated endpoints, flags, metric names or test results: every
  referenced name is read from the tree or the request.
- The spec never states an implementation detail that the user cannot observe,
  and never promises a library that is not in `gradle/libs.versions.toml`.
- The spec lists the permissions it implies and whether each needs a runtime
  request.
- The spec states the offline and error behaviour, not only the happy path.
- Secrets never appear in a spec; a value is named by its purpose, not by its
  value.
- The file is written to `docs/specs/<slug>.md`, one spec per behaviour.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The shared status set and the unidirectional flow contract | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4.2 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| Compose screens are a function of state and react to events | Android Developers — Compose architecture | 2026 | https://developer.android.com/develop/ui/compose/architecture |
| Navigation 3 uses typed keys with an application-owned back stack | AndroidX Navigation release notes | 2026 | https://developer.android.com/jetpack/androidx/releases/navigation3 |
| Deep links and notification taps are untrusted inputs | Android Developers — App links | 2026 | https://developer.android.com/training/app-links |
| Runtime permissions must be requested at the point of use | Android Developers — Permissions | 2026 | https://developer.android.com/training/permissions/requesting |

## Output Format

The spec file `docs/specs/<slug>.md`:

```
# <title>

## Problem
<the user-visible problem and who has it>

## Behaviour
<what the app does, as observable facts>

## Acceptance criteria
- [ ] <criterion a tester can check>

## State
- status: <UiStatus values this screen can show>
- events: <UiEvent variants>
- effects: <UiEffect variants>

## Route
<NavKey, back-stack position, deep-link path or none>

## Data
<server authority, local cache, device-local>

## Permissions
<permission> — <runtime request required | no>

## Out of scope
<explicitly excluded>

## Open questions
- <question> — <who answers it>
```

And the response reports the spec path plus the open questions, if any.
