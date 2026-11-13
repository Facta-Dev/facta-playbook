# Playbooks

Repeatable recipes for the changes you will make most. Each: the steps, the
files touched and the gate that proves it. If a change touches more files
than listed, the architecture regressed — investigate before proceeding.

## 1. Add a slice (a new feature)

1. `packages/feature-<slice>/src/` with `domain/`, `application/`,
   `adapters/out/`, `viewmodels/` and `views/`.
2. **Domain first**: the entity, its invariants, its `Result` errors. No
   `react-native`, no `expo-*`, no storage.
3. `application/ports.ts`: the interface the slice needs, declared by the
   slice.
4. `application/use-cases.ts`: the use-cases, returning `Result`.
5. `adapters/out/`: the implementation of the port.
6. `viewmodels/` and `views/`: the state, the React binding and the surface.
7. `index.ts`: the barrel. The app imports from here and from nowhere else
   inside the slice.
8. `packages/contracts/src/routes.ts`: the route entry and the builder for
   the new screen.
9. The package's `package.json`, with the workspace name
   `@<scope>/feature-<slice>`, `main` and `types` at `src/index.ts`.
10. The tests under `tests/<slice>/unit/`.
11. The ESLint scope and the architecture suite for the new slice: the
    guards are per-directory today, so a new slice must be added to
    `eslint.config.mjs` and to
    `tests/_platform/architecture/import-boundaries.test.ts`.

The `add-slice` skill carries the full recipe; the `/add-slice` command
delegates to it.

**Gate**: `pnpm run ci`.

## 2. Add a screen

1. The screen lives in its slice: `views/` for the component,
   `viewmodels/` for the state. The view model returns a state and commands;
   the view renders the state and calls the commands.
2. The route goes in `packages/contracts/src/routes.ts`: the `RouteSpec`
   row (`userType`, `module`, `resource`, `byId`) and the builder.
3. The screen is rendered from the app's route resolution — the route table
   decides which component the path resolves to.
4. The view renders **all eight statuses**
   ([03](03-hexagonal-layers-and-mvvm.md#5-the-eight-statuses)), including
   `offline` and `policy-blocked`.
5. A test that renders the view model in each status.

The `add-screen` skill carries the full recipe; the `/add-screen` command
delegates to it.

**Gate**: `pnpm run ci`.

## 3. Add a port and its adapter

1. **Decide where the port lives.** A capability every slice needs goes in
   `packages/core/src/ports.ts`; a capability one slice needs goes in that
   slice's `application/ports.ts`.
2. Write the port as an interface over plain values. It must not mention
   `expo-*`, a native module or a React type.
3. Write the fake in `tests/_shared/fakes/` and the test against the fake
   **before** the implementation.
4. Write the adapter in `adapters/out/`. It is the only file that imports
   the native module.
5. Wire the adapter to the port in the composition root (`app/App.tsx`) —
   and only there.
6. If the port is a core port, add it to the core package's exports
   (`packages/core/src/index.ts`).

The `add-port` skill carries the full recipe; the `/add-port` command
delegates to it.

**Gate**: `pnpm run ci` — the architecture suite fails if the adapter is not
the only place that imports the native module.

## 4. Add a route

1. `packages/contracts/src/routes.ts`: the `RouteSpec` row (`userType`,
   `module`, `resource`, `byId`) and the builder.
2. The builder is typed: a missing parameter is a compile error, and
   `resourcePath` refuses a resource id that would change the shape of the
   path.
3. The screen it resolves to is registered in the app's route resolution.
4. A contract test in `tests/contracts/routes.test.ts`: the builder's
   output parses back to the same route, and a path with a bad parameter is
   refused.
5. If the route is reachable from outside — a deep link or a push payload —
   check it against the parameter schema, not with a regular expression in
   the caller.

The `add-route` skill carries the full recipe; the `/add-route` command
delegates to it.

**Gate**: `pnpm run ci`.

## 5. Change a dependency pin

1. **Read the constraint first.** Expo pins the versions of the React Native
   ecosystem; a pin outside the SDK's compatibility set breaks the build in
   a way that looks unrelated.
2. Use the SDK's own tooling to pick the version (`expo install`), so the
   pin lands inside the compatibility set.
3. Run `expo-doctor` and the typecheck. A mismatch between the Expo SDK, the
   React Native version and the React version is the usual failure.
4. Re-measure the budgets on the low tier
   ([02](02-device-tiers-and-the-quality-governor.md)): a patch release that
   changes the engine's memory behaviour is a release, not a patch.
5. Record the change in the dependency register of the product.

**Gate**: `pnpm run ci`, plus the low-tier performance run if the pin
touches the runtime.

> **Known gap.** The template ships no dependency register and no
> `expo-doctor` gate. Both are adoption steps
> ([12 §9](12-testing-and-ci.md#9-the-adoption-steps)).

## 6. Ship a release

1. `pnpm run ci` — the gate on the release tree.
2. `pnpm run verify` — the template stays instantiable.
3. The build profile for the variant and the environment; the credentials
   from the secret store ([Admin Guide](admin-guide.md)).
4. The store checklist: permissions, privacy labels, the age rating, the
   account deletion path, the data safety form.
5. The manual pass on the **low** tier of the device pool
   ([Device Support Matrix](device-support-matrix.md)).
6. The staged rollout, with the health gates and the rollback plan:
   halt the rollout, ship an OTA fix, or flip the kill switch.

**Gate**: `pnpm run ci`, `pnpm run verify` and the release checklist.

## The pre-review checklist

- [ ] The domain owns the rule; the view does not re-implement it.
- [ ] The port speaks domain types; the use-case turns a failure into a
      `Result`.
- [ ] No `expo-*` import outside `adapters/out/` and the composition root.
- [ ] The screen renders all eight statuses.
- [ ] Every path is built by a route builder, never written by hand.
- [ ] No secret, token or note body reaches a log
      ([11](11-telemetry.md)).
- [ ] A schema change has a **new** migration
      ([08](08-offline-outbox-and-sync.md)).
- [ ] A new slice is added to the ESLint scopes and the architecture suite.
- [ ] The new behaviour has a test against the fake.
- [ ] `pnpm run ci` is green.
- [ ] The docs that describe the changed behaviour are updated.

## See also

- [Layers & MVVM](03-hexagonal-layers-and-mvvm.md)
- [Vertical Slicing](04-vertical-slicing.md)
- [Testing & CI](12-testing-and-ci.md)
