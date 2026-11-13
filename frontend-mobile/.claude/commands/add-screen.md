---
name: add-screen
description: "Delegate to the add-screen skill — add a <app> screen that renders one of the eight view-model statuses, stays accessible on a low tier and imports no adapter."
division: client
---

# /add-screen

## Delegate

Delegate to the **add-screen** skill. Activate when `<app>` needs a new
screen — a view that renders a view model, or a state of an existing one.

Steps:

1. Name the screen, the route that opens it and the view model that backs it.
2. Render one branch per status: loading, ready, empty, forbidden,
   not-found, offline, error and policy-blocked. A missing branch is a
   missing state, not a default.
3. Keep the screen inside the view layer: it imports its view model and its
   contracts, never an adapter, a repository or a device API.
4. Meet the accessibility basics: labelled controls, a 44pt minimum touch
   target, text scaling and a readable contrast ratio.
5. Add the test for the view model branch that the screen renders, and the
   navigation test for the route.
6. Run `pnpm run typecheck`, `pnpm run lint` and `pnpm run test` and report the
   real output.
