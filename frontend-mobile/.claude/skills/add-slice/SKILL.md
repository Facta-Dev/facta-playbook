---
name: "add-slice"
description: "Scaffolds a new vertical slice in the <app> mobile tree — domain, application, adapters/out, viewmodels, views, the barrel, the routes and the tests — with the layer discipline that keeps the hexagon closed."
division: core
---

# `<app>` add-slice skill

> Reusable workflow for adding one vertical slice to `<app>`. An agent
> invokes this skill; the requester supplies the feature, the skill supplies
> the scaffolding, the layer discipline and the gate ladder.

## When to Activate

Activate this skill when the request is a new feature that owns its own
domain — a new kind of entity, screen or workflow that will grow: a new
"tasks" slice, a new "settings" slice, a new "cart" slice.

Do NOT activate this skill when:

- The change adds a field, a rule or a screen to an **existing** slice —
  that is a change inside `packages/feature-<slice>/src/`, not a new slice.
- The change is a bug fix (fix it in place, with a red test first).
- The request is a cross-cutting refactor of the composition root.

## Core Principles

- **Vertical, not horizontal.** A slice owns one feature end to end: its
  domain rules, its use cases, its adapter, its view model, its view and its
  tests. Nothing about the feature is scattered across another package.
- **One slice, one workspace package.** The slice lives in
  `packages/feature-<slice>/` with its own `package.json`, published under
  the `@<scope>/feature-<slice>` name, and imported by the app only through
  its barrel.
- **Ports are consumer-defined.** The interface the slice needs lives in the
  slice's `application/ports.ts` — for the shipped slice, `packages/feature-notes/src/application/ports.ts`; the adapter that satisfies it lives in
  `adapters/out/` and is the only file allowed to name an `expo-*` package.
- **The composition root wires, it does not decide.** `app/App.tsx` builds
  the adapter, hands it to the view model and renders the view.
- **Mirror the `notes` slice.** The template already ships a worked example;
  copy its shape rather than inventing a new one.

## Rules

- No fabricated paths: every file created must be listed with its real path,
  and every existing path referenced must exist in the tree.
- The domain and the application layer carry **no** `react`, `react-native` or
  `expo-*` import; a device type in those layers is a defect, not a style
  choice.
- The view model is framework-free: React enters through the hook adapter in
  `views/`, and React Native never enters the slice at all.
- The slice ships with at least one test per layer: a domain test, an
  application test against the fakes in `tests/_shared/fakes/`, a view-model
  test per status, and the architecture suite for the new import rules.
- The slice is sealed: it never imports another `packages/feature-*`, and the
  shared code that two slices need moves to `packages/core` or
  `packages/contracts`.
- The routes of the slice are added to `packages/contracts/src/routes.ts` and
  built with the route builder, never written by hand.
- No new third-party dependency without a written reason; the Expo SDK
  packages already enabled are enough for a slice.
- No step is "done" until `pnpm run typecheck`, `pnpm run lint`,
  `pnpm run test` and `pnpm run export:android` are green; report the real
  output.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The slice anatomy and the ownership of each asset | `docs/04-vertical-slicing.md` | 2026 | local file |
| The layer law and the permitted dependency direction | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1 | 2026 | local file |
| The worked slice to mirror | `packages/feature-notes/src/` | 2026 | local file |
| The import rules the linter and the architecture test enforce | `eslint.config.mjs`, `tests/_platform/architecture/import-boundaries.test.ts` | 2026 | local file |
| The route table and the builders | `packages/contracts/src/routes.ts` | 2026 | local file |
| The gate ladder and what each one proves | `package.json`, `Makefile` | 2026 | local file |
| A workspace package resolves by its published name | pnpm workspaces documentation | 2026 | https://pnpm.io/workspaces |

## Output Format

```
Slice: <name> — path: packages/feature-<name>/
Created:
  domain/        <files> — <the rules, pure>
  application/   <files> — <port, use cases, Result errors>
  adapters/out/  <files> — <the port implementation>
  viewmodels/    <files> — <state, commands, the status set>
  views/         <files> — <the React Native surface and the hook adapter>
  index.ts       <the barrel the app imports>
  package.json   — name @<scope>/feature-<name>
Routes: <the entries added to packages/contracts/src/routes.ts>
Tests: <files> — <cases>
Gates:
  pnpm run typecheck → <result>
  pnpm run lint → <result>
  pnpm run test → <result>
  pnpm run export:android → <result>
Not done: <anything deliberately left out>
```
