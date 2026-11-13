---
name: "add-port"
description: "Adds a device capability to <app> as a port: the plain interface, the fake in tests/_shared/fakes/, the adapter in adapters/out/, the wiring in the composition root and the test that proves the seam."
division: security
---

# `<app>` add-port skill

> Reusable workflow for giving `<app>` a new device capability without
> letting the device leak into the layers. An agent invokes this skill; the
> requester supplies the capability, the skill supplies the port, the fake,
> the adapter and the wiring.

## When to Activate

Activate this skill when the request is to talk to the device from the app: a
secure store read, a push registration, a network-state observation, a deep
link, a location fix, a camera frame, an audio capture, a payment sheet.

Do NOT activate this skill when:

- The capability is a pure computation with no device side — it belongs in
  the domain, no port needed.
- The request is to change an existing port's signature; that is a change to
  the interface and its callers, reviewed as such.
- The capability is a network call to the backend through the SDK; that is an
  outbound adapter of a data source, not a device port.

## Core Principles

- **The port is a plain interface over plain values.** It speaks domain
  types and returns `Result` or typed events; it never mentions `expo-*`, a
  native module or a React type.
- **The consumer owns the port.** A capability every slice needs goes to
  `packages/core/src/ports.ts`; a capability one slice needs goes to that
  slice's `application/ports.ts` — for the shipped slice,
  `packages/feature-notes/src/application/ports.ts`.
- **The fake comes before the adapter.** The test runs against a
  hand-written fake in `tests/_shared/fakes/`; the adapter is written
  against that contract.
- **The adapter is the only file that names the device.** It lives in
  `adapters/out/` and imports the `expo-*` package there.
- **The composition root wires it.** `app/App.tsx` constructs the adapter and
  hands it to the view model; nothing else knows both.
- **Untrusted input is parsed.** A deep link, a notification payload and a
  device token are validated before they are used.

## Rules

- No fabricated paths: every file created is listed with its real path.
- The port file contains no `react`, `react-native` or `expo-*` import; the
  ESLint rule and the architecture suite both enforce it.
- The port returns `Result<T, E>` for an expected failure and never throws
  for one.
- The fake implements the port completely and stays hand-written; no mocking
  library is introduced.
- The adapter is written after the failing test and makes it pass without
  changing the port.
- Secrets never travel through a port that is not the secure store; a token
  is written to `SecureStorePort` and read from it, never held in a plain
  preference.
- The wiring is added to `app/App.tsx` and to nothing else.
- No step is "done" until `pnpm run typecheck`, `pnpm run lint`,
  `pnpm run test` and `pnpm run export:android` are green; report the real
  output.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The four core ports and their signatures | `packages/core/src/ports.ts` | 2026 | local file |
| The layer law: the device is an adapter | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1, §4.2 | 2026 | local file |
| The port-and-adapter recipe the kit follows | `docs/13-playbooks.md` §3 | 2026 | local file |
| Where the hand-written fakes live and that no mocking library is used | `docs/12-testing-and-ci.md` | 2026 | local file |
| The import rules enforced on `adapters/out/` | `eslint.config.mjs`, `tests/_platform/architecture/import-boundaries.test.ts` | 2026 | local file |
| The keystore the secure store adapter wraps | Expo SecureStore documentation | 2026 | https://docs.expo.dev/versions/latest/sdk/secure-store/ |
| The push registration the push adapter wraps | Expo Notifications documentation | 2026 | https://docs.expo.dev/versions/latest/sdk/notifications/ |
| The deep links the link adapter parses | Expo Linking documentation | 2026 | https://docs.expo.dev/versions/latest/sdk/linking/ |

## Output Format

```
Port: <name> — path: packages/core/src/ports.ts | packages/feature-<slice>/src/application/ports.ts
Signature:
  <the interface, in full>
Fake: tests/_shared/fakes/<file>.ts — <the behaviours it simulates>
Test: tests/<path>.test.ts — <the cases>
Adapter: packages/feature-<slice>/src/adapters/out/<file> — <the expo-* package it imports>
Wiring: app/App.tsx — <how the adapter reaches the view model>
Untrusted input: <what is parsed and validated>
Gates:
  pnpm run typecheck → <result>
  pnpm run lint → <result>
  pnpm run test → <result>
  pnpm run export:android → <result>
Not done: <anything deliberately left out>
```
