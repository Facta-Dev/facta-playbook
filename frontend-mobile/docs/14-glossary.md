# Glossary

Every term used in this set, defined once. Where a term has a platform
meaning and a kit meaning, the kit meaning is the one that counts here.

**Adapter** — An implementation of a port that talks to the outside world.
`packages/feature-notes/src/adapters/out/in-memory-notes-repository.ts`
implements `NotesRepositoryPort`.

**Adapters (out)** — The layer of a slice that may import `expo-*` or a
native module. The only layer allowed to touch the device.

**Android channel** — The category a notification belongs to on Android. It
is created at install time and carries its own importance and sound.

**App link / Universal link** — A `https://` link the OS routes to the app,
verified through `assetlinks.json` or `apple-app-site-association`. Preferred
over a custom scheme, which is not verified.

**APNs** — Apple's push notification service. The iOS counterpart of FCM.

**Barrel** — The `index.ts` of a slice: its only public face. The app imports
from the barrel, never from a layer inside it.

**Cold start** — The app launching from nothing: the process starts, the
bundle loads, `App.tsx` renders. A deep link that arrives at this moment is
**queued** until the bootstrap completes.

**Composition root** — The file that knows everything and wires it together:
`app/App.tsx`. It contains no rules.

**Conflict** — The server refusing a mutation because the state it was based
on is no longer current. It is a typed result, resolved in the view model.

**Contract** — A zod schema in `packages/contracts/` that both ends of a
wire validate with. The contract bounds what may travel; the domain bounds
what may exist.

**Correlation id** — An identifier generated per user action and propagated
through the requests, the logs and the error screens. Safe to log; it
identifies an action, not a person.

**Deep link** — A path that opens the app, from a browser or from a
notification. **Untrusted input**: it is parsed against the route table
before anything else happens to it.

**Device tier** — `low`, `mid` or `high`: the class the device belongs to.
Derived from the hardware hints, then lowered — never raised — by the
measured performance.

**Domain** — The rules of a feature: entities, invariants and their errors.
Pure: no device, no framework, no storage.

**Domain rule vs contract bound** — The contract says "a title is at most
120 characters" (what may travel on the wire); the domain says the same, and
adds "no markup" (what may be a note). Both exist on purpose: the contract
bounds the wire, the domain bounds the entity.

**Exponential backoff** — The retry delay doubles per attempt, with jitter.
Used by the outbox and by any retrying client.

**FCM** — Firebase Cloud Messaging, the Android push service. The **HTTP v1**
API is the current one; the legacy endpoint is retired.

**Forward-only migration** — A schema change that only moves up. The old
shape is read, the new shape is written; there is no down path.

**Hysteresis** — The delay before the quality governor steps **up** again.
Without it, the profile flaps and the user sees it.

**Idempotency key** — A key generated once per mutation and sent on every
retry. The server deduplicates on it, so a retry is safe.

**JS thread** — The thread that runs the JavaScript bundle: rendering, event
handlers, promises. The scarce resource of a React Native app.

**Metro** — The bundler. In development it serves the bundle; in a release
build it produces the bytecode that ships.

**Native module** — A native implementation reached over the bridge:
`expo-secure-store`, `expo-notifications`, `expo-linking`.

**New Architecture** — The React Native architecture this template runs on.
A library that does not support it does not belong in the app.

**Optimistic row** — A row shown in the UI before the server accepted the
mutation. A presentation choice, not a fact.

**Outbox** — The queue of mutations the server has not accepted yet. It is
durable: it survives the app being killed.

**Port** — An interface the consuming slice declares, so the use-case
depends on the abstraction and not on the device.

**Projection** — The local database's content: what the server said, cached.
The server is the authority; the projection may be wiped at any time.

**Quality governor** — The pure function that derives the `QualityProfile`
from the tier, the thermal state, the memory pressure and the battery saver.

**Quality profile** — The knobs the governor moves: render scale, frame
budget, shadows, texture resolution, particle budget.

**Result** — A value-typed outcome: `{ ok: true, value }` or
`{ ok: false, error }`. Expected failures are values, not exceptions. It
lives in `packages/core/src/result.ts`.

**Route table** — The typed list of routes in
`packages/contracts/src/routes.ts`. The only place a path is written down.

**Rule** — A statement about what a domain may contain. Enforced by the
domain functions and by the strict schemas at the boundaries.

**Sealed slice** — A slice that never imports another slice. The
architecture test enforces it.

**Sync cursor** — The server's "everything after this point" marker, stored
in the local database. It is what makes an incremental sync incremental.

**UiStatus** — The eight values a view model's status may take: `loading`,
`ready`, `empty`, `forbidden`, `not-found`, `offline`, `error`,
`policy-blocked`.

**Universal link** — See _App link / Universal link_.

**View model** — The presentation state and the commands that change it. In
this kit it is a framework-free object; React enters through the hook in
`views/`. It holds no device API.

**Vertical slicing** — Cutting the code by feature instead of by technical
layer, so one feature is one package with all of its layers.

**Workspace package** — A package in the pnpm workspace. A slice is one;
its `main` and `types` point at `src/index.ts`
([ADR 0002](adr/0002-source-only-workspace-packages.md)).

## See also

- [Overview & Principles](00-overview-and-principles.md)
- [Playbooks](13-playbooks.md)
