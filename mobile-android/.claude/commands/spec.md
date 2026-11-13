---
name: spec
description: "Delegate to the spec skill — write docs/specs/<slug>.md for a <app> Android behaviour: user-visible rules, acceptance criteria, UiStatus/UiEvent/UiEffect, route, data boundary and out-of-scope."
division: core
---

# /spec

## Delegate

Delegate to the **spec** skill. Activate when a request for `<app>` is too
vague to implement and the behaviour has to be written down first.

Steps:

1. Restate the request as a user-visible problem, and ask for the missing
   acceptance criteria instead of inventing them.
2. Describe the behaviour as observable facts: what the user sees, what the
   app guarantees, what happens offline and what happens on error.
3. Pin the state vocabulary: the `UiStatus` set the screen can show, the
   `UiEvent`s it accepts and the one-shot `UiEffect`s it emits.
4. State the route, if the behaviour has a screen: the Navigation 3 `NavKey`,
   its position in the back stack and its deep-link path.
5. State the data boundary: what the server owns, what the local cache
   mirrors, what stays on the device.
6. List the implied permissions and whether each needs a runtime request, then
   write the file to `docs/specs/<slug>.md` with the out-of-scope list and the
   open questions.
