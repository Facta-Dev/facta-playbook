# ADR 0001 — Expo with a plain `App.tsx` entry

|                |                                                               |
| -------------- | ------------------------------------------------------------- |
| **Status**     | Accepted                                                      |
| **Date**       | 2026-11-12                                                    |
| **Scope**      | The framework and the navigation model of the mobile template |
| **Supersedes** | —                                                             |

## Context

The mobile client needs three things at once:

1. **A build pipeline that produces a store artifact.** Android and iOS
   builds, signing, native modules, and a continuous native generation step
   that keeps the native projects reproducible.
2. **A navigation model that survives a deep link.** A path that arrives from
   the OS must resolve to a screen, and the resolution must be the same in
   the app, in a test and in a push payload.
3. **A route grammar shared with the other clients.** The path shape comes
   from the URL playbook:
   `/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]`.

The framework decision and the navigation decision are one decision,
because in Expo the navigation library also owns the native link handling
and the URL scheme configuration.

The candidates for the navigation model were:

| Option                | What it is                                                                                                        |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Expo Router**       | A file-based router on top of React Navigation; it owns the deep-link configuration and can generate typed routes |
| React Navigation      | The library underneath Expo Router; imperative and declarative, no file convention                                |
| A hand-written router | The app holds the navigation state and resolves a path itself                                                     |

The Expo platform decision itself was made before this ADR and is not
reopened here: **Expo SDK 58 with Continuous Native Generation and
development builds**, escaping to bare only for a native need that Expo
cannot serve.

## Decision

**Expo SDK 58, React Native 0.88.1, React 19.3.0, TypeScript, and a plain
`App.tsx` entry. No `expo-router`, no file-based routing.**

The entry is `app/index.ts`, which registers `app/App.tsx` as the root
component. The app resolves a path against the typed route table in
`packages/contracts/src/routes.ts` (`parseRoute`) and renders the matching
screen.

### Why not `expo-router`

`expo-router` is the recommendation of the mobile playbook, and it is a good
library. It was rejected here for three reasons, in order of weight.

**1. The file convention would be a second source of truth.** With a
file-based router, the route table is the directory tree: `app/notes/[id].tsx`
_is_ the route `/notes/:id`. The template needs the route table to be
**typed, shared with the contracts package and validated by a contract test**,
because the same routes are referenced by the push payload, the deep-link
parser and the server. A convention-derived table cannot be shared across a
package boundary without duplicating it, and a duplicated table drifts.

**2. The template's route grammar is not expressible as a directory.** The
grammar is `/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]`
— three, four or five segments. As a directory tree that is
`app/[userType]/[module]/[resource]/[id]/[sub]` — a dynamic segment at three
levels, resolved at runtime for every navigation, with the parameter names
coming from the folder names. The typed table expresses it directly: one
entry per route, with the parameters named and typed.

**3. The convention costs more than it saves at the size of this kit.** The
router earns its keep when a team has dozens of screens, a shared layout
convention and a large group of people who all know the convention. The
template ships one slice and two screens. A file tree with a dynamic segment
at three levels is more machinery than the whole navigation surface.

### What the plain entry buys

| Property                          | Why it matters here                                |
| --------------------------------- | -------------------------------------------------- |
| The route table is one typed file | It is shared with the contracts package and tested |
| A builder is compile-time checked | `notePath(id)` cannot lose its parameter           |
| The navigation state is a value   | It can be serialised, asserted on and restored     |
| A screen is a component           | Adding a screen is adding a file and a table entry |
| The deep-link path is explicit    | Nothing is mounted by path by accident             |

### What it costs

The price is honest and is paid in the shell:

- **Layouts are yours.** A nested layout is a React component in the
  navigation state, not a directory. The template's shell has one level.
- **The route → component mapping is a table in the app.** It is a `switch`
  or a map, and it must be kept in step with the route table.
- **No typed routes generated from the file tree.** The typed routes come
  from the contracts package instead, which is where they belong here.

## The fallback path

Introducing a file-based router later is admissible only when **all** of
these hold:

1. The route count and the layout depth have outgrown the flat table.
2. The route table stays the single source of truth — the file tree
   generates it, or the file tree is validated against it in a test.
3. The deep-link parser still validates against the same table; the router
   does not become a second parser.
4. The typed route builders keep their signatures; the consumers do not
   change.
5. **This ADR is updated in the same pull request.**

Step 5 is the one that matters. The switch to a file-based router changes
the shape of every screen import, so it is a decision with a record, not a
gradual drift.

## Alternatives considered

### Expo Router (file-based)

**Rejected**, for the three reasons above. It remains the recommended
option for a product that grows past the flat table, and the fallback path
describes how to get there without breaking the contracts.

### React Navigation, used directly

**Rejected for now.** It solves the harder half of the problem (the native
navigation UI, the stack, the gestures) and leaves the routing decisions —
the path shape, the parameter names, the deep-link validation — to the
app, which is the part this template wants to type and test. It is also a
reasonable second step: the navigation state is already a value, so
swapping the renderer does not change the routes.

### A hand-written router with no library

**Rejected.** The template would have to implement the platform back
gesture, the header, the animation and the Android hardware back button.
That is a library's job, not a template's.

### Bare React Native without Expo

**Rejected.** The build pipeline, the config plugins and the continuous
native generation are the expensive parts, and the template needs them for
the per-variant native modules ([06](../06-variants-and-monorepo.md)). Bare
is the right answer only for a native need Expo cannot serve.

### One binary with all three variants

**Rejected.** It would carry the game engine in the market app, and the
download size and the memory are paid on the low tier
([Device Support Matrix](../device-support-matrix.md)).

## Consequences

**Accepted:**

- One route table, typed, shared with the contracts package and covered by
  a contract test.
- The navigation state is a plain value the shell owns.
- A deep link is parsed by the same code in every situation: cold start,
  running app, notification tap.
- The app has no dependency on a routing convention.

**Accepted, with a known gap:**

- **No typed routes are generated from the filesystem**, because there is no
  filesystem convention. The typed builders in `packages/contracts` are the
  substitute, and they must be extended by hand when a route is added
  ([Playbooks §4](../13-playbooks.md#4-add-a-route)).
- **The shell does not exist yet.** The template ships the route table, the
  builders and the parser; the navigation surface that renders a screen for
  a parsed route is an adoption step.
- **No nested layouts.** A screen that needs a persistent header, a tab bar
  or a modal stack is written in the shell. When the app needs two levels of
  layout, this ADR is revisited.
- **The route → screen mapping lives in the app**, not in the contracts
  package, because a React component cannot be shipped from a package that
  must not depend on the UI. The mapping is one file and it is tested.
- **The route table declares no subresource.** The shared grammar has an
  optional fifth `{subresource}` segment; the typed `RouteSpec` has no such
  field, so a path with five segments is refused as an unknown route until
  the app adds the row
  ([Navigation, Routes & Deep Links](../07-navigation-routes-and-deep-links.md)).

**Consequences for the code:**

- `app/index.ts` registers `app/App.tsx`; there is no `expo-router` plugin
  in `app/app.json`.
- The route table, the builders and the parser are in
  `packages/contracts/src/routes.ts`.
- The app's shell resolves a `RouteSpec` to a component.

## Revisit trigger

Revisit this ADR when **any** of the following happens:

1. The app has more than about twenty screens, or a nested layout that the
   flat shell cannot express.
2. Two people build screens in parallel and the file convention would
   remove a class of mistakes.
3. The product adopts a router for a feature the template does not have
   (a tab navigator, a native modal stack, a shared web view screen).
4. Expo's typed-routes generation becomes mature enough to generate the
   contracts table rather than duplicate it.

At that point the choice is between Expo Router and React Navigation used
directly, and the criteria above — one table, one parser, one set of typed
builders — are what must survive whichever is chosen.

## See also

- [Navigation, Routes & Deep Links](../07-navigation-routes-and-deep-links.md)
- [Variants & Monorepo](../06-variants-and-monorepo.md)
- [ADR 0002 — Source-only workspace packages](0002-source-only-workspace-packages.md)
- [Playbook Frontend Mobile §1, §6](../../PLAYBOOK_FRONTEND_MOBILE.md)
