---
name: add-route
description: "Delegate to the add-route skill — add a <app> route to the contract table, to the navigation and to the deep-link grammar, with the parsing test that proves it."
division: core
---

# /add-route

## Delegate

Delegate to the **add-route** skill. Activate when `<app>` needs a new route,
a new route parameter or a change to the deep-link grammar.

Steps:

1. Name the route, its path, its parameters and whether it is sessionless.
2. Add the route to `packages/contracts/src/routes.ts` — the table, the
   resource path and the parameter extraction — with no user-type or tenant
   segment in the path.
3. Register the screen in the navigation and make the deep-link scheme in
   `app/app.json` reach it.
4. Add the parsing test in `tests/contracts/routes.test.ts`: the accepted
   path, the rejected paths and the error case.
5. Confirm the screen still receives the route as a parameter rather than
   reading the navigation state directly.
6. Run `pnpm run typecheck`, `pnpm run lint` and `pnpm run test` and report the
   real output.
