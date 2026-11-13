---
name: add-slice
description: "Delegate to the add-slice skill — cut a new <app> feature slice with its domain, application, adapters/out, view model, view and colocated tests, wired into the composition root."
division: core
---

# /add-slice

## Delegate

Delegate to the **add-slice** skill. Activate when `<app>` needs a new
feature slice — a bounded capability with its own domain model, its own use
cases and its own screens.

Steps:

1. Name the slice, its noun, its aggregate and the user story that justifies
   it. A slice that needs a second aggregate is two slices.
2. Create the workspace package `packages/feature-<name>/` with the layers
   in order: `domain/`, `application/`, `adapters/out/`, `viewmodels/`,
   `views/`, plus the `package.json` and the barrel `src/index.ts`.
3. Write the domain first, then the use cases, then the view model with the
   eight statuses, then the views. The view model never imports an adapter.
4. Add the colocated tests under `tests/<slice>/unit/<layer>/` and a fake for
   every port the slice consumes.
5. Wire the slice into `app/App.tsx` — the app imports the package barrel
   and never a file inside it.
6. Run `pnpm run typecheck`, `pnpm run lint` and `pnpm run test` and report the
   real output.
