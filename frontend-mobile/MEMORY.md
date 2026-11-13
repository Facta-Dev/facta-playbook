# MEMORY.md — `<app>` premise

Durable, versioned premise for every session. Keep it under ~100 lines: facts
an agent cannot derive quickly from the code, not a copy of the code.

## Product

`<app>` is a React Native + Expo mobile app: one plain `App.tsx` entry, a
pnpm workspace (the app plus the shared packages), the hexagonal layers of
`PLAYBOOK_FRONTEND_MOBILE.md` and a device-tier quality governor. It talks
to a backend through an SDK that is not part of this template. The app is a
**public client**: no secret ships in the bundle, only `EXPO_PUBLIC_` names.

## Slices

A slice is `packages/feature-<slice>/src/` with `domain/` · `application/` ·
`adapters/out/` · `viewmodels/` · `views/`, and a barrel `index.ts` the
composition root imports from.

| Slice   | Owns                                | Notes                                            |
| ------- | ----------------------------------- | ------------------------------------------------ |
| `notes` | the notes collection of the example | **example slice** — replace with the real domain |

Slices never import each other. A shape shared by two slices moves to
`packages/core/` (pure types) or `packages/contracts/` (zod + routes), never
a cross-slice import edge.

## Ports

`packages/core/src/ports.ts` declares the platform capabilities as plain
TypeScript interfaces, with **no implementation**:

| Port               | Owns                                                            |
| ------------------ | --------------------------------------------------------------- |
| `PushPort`         | permission, device registration, foreground notifications, taps |
| `SecureStorePort`  | the secure key/value store — secrets only                       |
| `NetworkStatePort` | the connectivity of the device                                  |
| `DeepLinkPort`     | the cold-start link and the links that arrive while running     |

The playbook's other ports (`BiometricsPort`, `LocationPort`,
`DeviceCapabilityPort`, `SceneEnginePort`, `AudioPort`, `PaymentsPort`,
`RealtimePort`) are not declared yet: add them when a slice needs one, with
the adapter that implements it.

## Device tier and the quality governor

`DeviceTier` is `low | mid | high` (`packages/core/src/device-tier.ts`).
The tier comes from the low-RAM flag, the RAM size and the OS level; the
first-run measurement (the p95 frame time of a reference scene) may **lower
the hint, never raise it**. Without a signal the tier is `mid`.

`governQuality` (`packages/core/src/quality-governor.ts`) is a **pure
function**: the tier plus the thermal state, the memory pressure and the
battery saver in, a `QualityProfile` out (render scale, frame budget,
shadows, texture resolution, particle budget). The baseline is the tier; the
signals only step it down. Stepping up again needs a sustained healthy
period (`mayStepUp`, 30 s) — the hysteresis lives in the runtime loop, not
in the pure function.

## Routes

One table, `packages/contracts/src/routes.ts`, one grammar:
`/{userType}/{module}/{resource}[/{resourceId}]`, plus the session-less
routes `/onboarding`, `/login`, `/register`, `/forgot-password`,
`/reset-password`, `/verify-email`.

| userType | module          | resource      | by id |
| -------- | --------------- | ------------- | ----- |
| `app`    | `notes`         | `entries`     | yes   |
| `app`    | `market`        | `products`    | yes   |
| `app`    | `notifications` | `preferences` | no    |
| `admin`  | `tenants`       | `users`       | yes   |

`notes/entries` is the example slice. Screens never write a path: they call
`notesListPath()`, `notePath()` or `resourcePath()`, and `parseRoute()` is
the only parser. The `{userType}` segment is an experience namespace, never
an authorization.

## Version pins that must not drift

Node **≥ 24** · pnpm **10.x** · Expo **~58.0** (SDK 58) · React **19.3.0**
(exact — 19.3.1 fails `expo-doctor`) · React Native **0.88.1** (exact) ·
TypeScript **~6.0.3** (exact — the SDK 58 pin) · Vitest **~5.1.0** ·
ESLint **^9.39.0** + typescript-eslint **^8.72.2** · Prettier **^3.10.1** ·
zod **^4.6.7** · `@types/react` **~19.3.0**. `.npmrc` sets
`node-linker=hoisted`: React Native needs one copy of `react` in the tree.

## Test tiers

| Tier           | Where                             | Scope                                            |
| -------------- | --------------------------------- | ------------------------------------------------ |
| unit           | `tests/<slice>/unit/domain/`      | entities and their invariants                    |
| unit           | `tests/<slice>/unit/application/` | use cases against a fake port                    |
| unit           | `tests/<slice>/unit/viewmodels/`  | states and commands against a fake repository    |
| kernel         | `tests/core/`                     | the tier and the quality governor                |
| contract       | `tests/contracts/`                | the route table: builders against the parser     |
| architecture   | `tests/_platform/architecture/`   | the layer boundaries, read from the source files |
| contract evals | `evals/contracts.test.mjs`        | the `.claude` layer, the fixtures, the posture   |

No test touches the network, a real device or a real identity provider. A
unit test imports the module under test by a relative path, so a node test
never pulls React Native into the process.

## Gates

`make ci` (typecheck + lint + test + android export) · `pnpm run evals`
(`node --test evals/contracts.test.mjs`) · `scripts/verify-template.sh`
(template factory only: it stages a renamed copy and runs every gate on it).

## Where the contracts live

`PLAYBOOK_FRONTEND_MOBILE.md` (architecture, device classes, the governor,
navigation, push, testing) · `PLAYBOOK_TESTS.md` · `PLAYBOOK_SECURITY.md` ·
`PLAYBOOK_FRONTEND.md` · `PLAYBOOK_URLS.md` (the route grammar) ·
`PLAYBOOK_AUDIT.md` · `PLAYBOOK_CLAUDE_CODE.md` · `AGENTS.md` (agent
workflow).
