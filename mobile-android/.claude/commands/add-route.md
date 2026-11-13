---
name: add-route
description: "Delegate to the add-route skill — add a Navigation3 destination to the <app> Android app: the typed @Serializable NavKey, its entryProvider entry, the back-stack position, the deep-link mapping and the guard."
division: client
---

# /add-route

## Delegate

Delegate to the **add-route** skill. Activate when `<app>` gains a screen, a
dialog destination or a new deep-link target in its Navigation 3 graph.

Steps:

1. Declare the route as a typed `@Serializable` `NavKey` member of the sealed
   hierarchy — no path string anywhere in the client.
2. Register the entry in the `entryProvider` of `:app`; the composable
   receives its dependencies as parameters and reaches for no global.
3. Update every `when` that matches on the route hierarchy: adding a member
   to a sealed hierarchy is a compile-time change for all of them.
4. Define the back-stack position, the title as a string resource and the up
   behaviour; state the guard (authentication, permission) and the
   `UiStatus.Forbidden` path it produces.
5. If the route is reachable from outside, add it to the deep-link parser,
   validate its parameters, and queue it for after sign-in when the user is
   signed out.
6. Run `./gradlew :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test`
   and `bash scripts/check-architecture.sh`, and report the real output.
