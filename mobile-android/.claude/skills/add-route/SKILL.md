---
name: "add-route"
description: "Adds a Navigation3 route to the <app> Android app: a typed @Serializable NavKey, its entry in the entryProvider, the back-stack position, the deep-link mapping and the guard that protects it."
division: client
---

# `<app>` add-route skill

> Reusable workflow for adding one screen to the Navigation 3 graph of `<app>`.
> An agent invokes this skill; the requester supplies the screen, the skill
> supplies the typed key, the entry, the back stack and the deep-link wiring.

## When to Activate

Activate this skill when a new screen, dialog or full-screen destination is
added to `<app>`, when an existing screen changes its position in the back
stack, or when a deep link or notification tap must land on a specific
destination.

Do NOT activate this skill when:

- The change is a new field inside an existing screen — that is a change in
  `:feature:notes:ui`, not a new route.
- The change is a new slice with its own screen — that is `add-slice`, which
  registers the route as part of the slice.
- The request is about the route table shared with the backend; that is a
  contract change, not a client route.

## Core Principles

- **Routes are types, not strings.** Each destination is a `@Serializable`
  `NavKey` object; there is no hand-written path string in the client.
- **The back stack is state the app owns.** A route is a push onto that stack,
  and the stack survives configuration change because it is remembered above
  the `NavHost`.
- **One entry provider.** All destinations are declared in the
  `entryProvider` of `:app`; a screen never navigates by constructing a
  back stack of its own.
- **A route has a guard, a title and a way out.** The guard decides what an
  unauthenticated or unauthorised user sees; the title is a resource, not a
  literal; the up behaviour is defined.
- **Deep links are untrusted.** A path arriving from outside is parsed into a
  `NavKey` through a parser that validates it; the parser never trusts the
  path.

## Rules

- No fabricated route names or paths: the `NavKey` type, its serial name and
  the deep-link path are written exactly as they will exist, and the path
  grammar is the one the shared route table defines.
- A new `NavKey` is a sealed hierarchy member; adding one to a sealed
  hierarchy is a compile-time change for every `when` that matches it, and
  those `when`s are updated in the same change.
- Parameters are typed and optional only when the screen genuinely works
  without them; a required parameter is not defaulted to a magic value.
- The entry composable receives its dependencies as parameters — it never
  reaches for a global or constructs a repository.
- The screen's `UiStatus` set is complete: `Loading`, `Ready`, `Empty`,
  `Offline`, `Forbidden`, `NotFound`, `Error` and `PolicyBlocked` as
  applicable, with the `Forbidden` case wired to the guard.
- A deep link that arrives while the user is signed out is queued and resumed
  after authentication; it is never processed as if it were trusted.
- The change is not done until `./gradlew :app:assembleDebug`, `./gradlew test`,
  `./gradlew :evals:test` and `bash scripts/check-architecture.sh` are green,
  and the real output is reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Navigation 3 uses typed keys with an application-owned back stack | AndroidX Navigation release notes | 2026 | https://developer.android.com/jetpack/androidx/releases/navigation3 |
| Navigation in Compose, including multi-back-stack and predictive back | Android Developers — Compose navigation | 2026 | https://developer.android.com/develop/ui/compose/navigation |
| Deep links and App Links map an incoming URL to a destination | Android Developers — App links | 2026 | https://developer.android.com/training/app-links |
| The route grammar of the shared route table | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §6 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| A screen is a function of state and reacts to events | Android Developers — Compose architecture | 2026 | https://developer.android.com/develop/ui/compose/architecture |

## Output Format

```
Route: <NavKey type> — <serial name> — <deep-link path or none>
Created:
  <path> — <NavKey declaration>
  <path> — <entry composable registration>
  <path> — <deep-link parser entry, if any>
Changed:
  <path> — <sealed hierarchy or when-branch updated>
Guard: <authentication / permission behaviour>
Back stack: <where the route is pushed and what it pops>
Gates:
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
Open: <anything left, such as a missing title resource>
```
