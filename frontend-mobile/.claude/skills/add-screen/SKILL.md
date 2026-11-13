---
name: "add-screen"
description: "Adds a screen to an existing <app> mobile slice: the view, the view-model state and commands, the route entry and the status coverage, with the accessibility and low-tier posture a React Native screen needs."
division: client
---

# `<app>` add-screen skill

> Reusable workflow for adding one screen to a slice of `<app>`. An agent
> invokes this skill; the requester supplies the screen, the skill supplies
> the state model, the route entry, the status coverage and the gate ladder.

## When to Activate

Activate this skill when the request is a new screen inside an **existing**
slice: a new list, a new detail view, a new form, a new settings panel.

Do NOT activate this skill when:

- The screen belongs to a brand-new slice (use `add-slice`).
- The change is a copy or a restyle of an existing screen.
- The request is a bug fix (fix it in place, with a red test first).

## Core Principles

- **The screen is a function of a state.** The view receives a state and a
  set of commands; it never derives the business rule and never calls the
  repository.
- **The state is framework-free.** The view model returns a state, commands
  and a status, with no `react-native` and no `expo-*` import; React enters
  through the hook adapter in `views/`.
- **Every screen renders every status.** `loading`, `ready`, `empty`,
  `forbidden`, `not-found`, `offline`, `error` and `policy-blocked` are all
  reachable states of the screen, not just the happy one.
- **The path is built, never typed.** The route lives in
  `packages/contracts/src/routes.ts` with its builder, so a link and a test
  resolve to the same path.
- **The screen works on the low tier.** No unbounded list, no full-resolution
  image, no work on the JS thread in the render path.

## Rules

- No fabricated paths: every file created is listed with its real path.
- The view never imports an adapter or a repository; the ESLint rule and the
  architecture suite both enforce it.
- The view model returns `Result` failures as values; the view shows the
  `error` status, it does not catch a thrown error.
- Each new screen ships a test per status and a test for each command.
- Accessibility is part of the screen, not a follow-up: a role, a label and a
  touch target on every action, and a state exposed for a screen reader.
- The route is added to the route table with its `userType`, `module`,
  `resource` and `byId`, and the parameters are validated on the way in.
- No step is "done" until `pnpm run typecheck`, `pnpm run lint`,
  `pnpm run test` and `pnpm run export:android` are green; report the real
  output.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The eight statuses and the view-model contract | `docs/03-hexagonal-layers-and-mvvm.md` | 2026 | local file |
| The layer law for views and view models | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1 | 2026 | local file |
| The view rules the linter enforces on `views/` | `eslint.config.mjs` | 2026 | local file |
| The route table, the builders and the deep-link rules | `packages/contracts/src/routes.ts`, `docs/07-navigation-routes-and-deep-links.md` | 2026 | local file |
| The screen to mirror | `packages/feature-notes/src/views/` | 2026 | local file |
| The low-tier budgets the screen must respect | `docs/02-device-tiers-and-the-quality-governor.md` | 2026 | local file |
| The accessibility metadata a React Native element exposes | React Native accessibility documentation | 2026 | https://reactnative.dev/docs/accessibility |
| The list virtualization rule for a long list | React Native FlatList documentation | 2026 | https://reactnative.dev/docs/flatlist |
| The React rules the view layer follows: state, effects and re-renders | React documentation | 2026 | https://react.dev/reference/react/useState |

## Output Format

```
Screen: <name> — path: packages/feature-<slice>/src/views/<file>
Created:
  views/<file>       — <the component>
  views/<hook file>  — <the React adapter over the view model>
  viewmodels/<file>  — <state, commands, statuses>
Routes: <the entry added to packages/contracts/src/routes.ts>
Statuses rendered: <loading · ready · empty · forbidden · not-found · offline · error · policy-blocked>
Accessibility: <roles, labels, touch targets>
Tests: <files> — <cases>
Gates:
  pnpm run typecheck → <result>
  pnpm run lint → <result>
  pnpm run test → <result>
  pnpm run export:android → <result>
Not done: <anything deliberately left out>
```
