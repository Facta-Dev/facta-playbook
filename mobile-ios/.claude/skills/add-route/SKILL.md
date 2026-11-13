---
name: "add-route"
description: "Adds a typed route to the <app> iOS template: an enum case in the slice's Contracts package, the app's navigationDestination mapping, the deep-link parser update and the route test."
division: core
---

# `<app>` add-route skill

> Routes are values, not strings. This skill adds one with the enum case, the
> destination, the deep link and the test in one change.

## When to Activate

Activate when a screen becomes reachable from another screen, a deep link or a
push notification, and the route does not exist yet.

Do NOT activate when:

- The route exists and only its payload changes — extend the case.
- The screen is the app's root; the root is not a route.

## Core Principles

- **The route enum is the single vocabulary.** `NotesRoute` (or the slice's
  route enum) is the only way a destination is named.
- **The destination switch is exhaustive.** A new case breaks the build until
  every host handles it — that is the point.
- **Deep links map onto routes, not beside them.** The parser produces a route
  value; it never opens a screen directly.
- **The path is app state.** `Router.path` is the single source of the stack;
  no screen owns its own navigation stack.
- **A route change is a contract change.** Other slices and the push payload
  depend on it.

## Rules

- The case is added to the slice's `Contracts` package, never to the app
  target.
- Payload fields are typed values (`NoteID`, not `String`) when the domain has
  a type for them.
- The app's `navigationDestination(for:)` switch gains the case and a view.
- The deep-link parser (when present) gains the host/path mapping and rejects
  unknown paths.
- A push payload that names a route carries the same fields and is validated
  before it is appended to the path.
- Tests: the route round-trips through `Hashable` equality, and the parser
  rejects an unknown path.
- Gates: `make ci`, `bash scripts/check-architecture.sh`.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| NavigationStack drives a typed path | Apple — NavigationStack | 2026 | https://developer.apple.com/documentation/swiftui/navigationstack |
| Deep links and universal links enter through a URL | Apple — Supporting universal links | 2026 | https://developer.apple.com/documentation/xcode/allowing-apps-and-websites-to-link-to-your-content |
| Routes are named by `PLAYBOOK_URLS.md` | `PLAYBOOK_URLS.md` | - | `../PLAYBOOK_URLS.md` |

## Output Format

```
Route: <Case> — <payload>
Contracts: <file> — the enum case
Destination: <file>:<line> — the switch arm and the view
Deep link: <host/path -> route> — <or: not present>
Validation: <unknown-path rejection>
Tests: <file> — <round-trip / rejection>
Gates:
  bash scripts/check-architecture.sh -> <result>
  make ci -> <result>
```
