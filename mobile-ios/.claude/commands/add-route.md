---
name: add-route
description: "Delegate to the add-route skill — add a typed route to the <app> iOS template: an enum case in the slice's Contracts package, the navigationDestination mapping, the deep-link parser update and the route test."
division: core
---

# /add-route

## Delegate

Delegate to the **add-route** skill. Activate when a screen becomes reachable
from another screen, a deep link or a push notification and the route does
not exist yet.

Steps:

1. Add the case to the slice's `Contracts` route enum, never to the app
   target; payload fields use domain types (`NoteID`, not `String`).
2. Add the arm to the app's `navigationDestination(for:)` switch and the view
   it renders — the switch is exhaustive, so the build breaks until every
   host handles the case.
3. Update the deep-link parser (when present) so a URL produces the route
   value, and reject unknown paths.
4. Validate a push payload that names the route before appending it to
   `Router.path`.
5. Add the tests: the case round-trips through `Hashable` equality, and the
   parser rejects an unknown path.
6. Run `bash scripts/check-architecture.sh` and `make ci`, and report the
   real output.
