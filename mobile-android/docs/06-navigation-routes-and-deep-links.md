# Navigation Routes & Deep Links

Navigation3 replaces the route-string paradigm: a route is a **typed value**,
a `@Serializable` `NavKey`. There is no path string to mistype, no argument
bundle to guess, and no `findNavController().navigate("notes/{id}")` that
compiles until the argument is missing.

## 1. Typed routes

A route is a `@Serializable` type that implements `NavKey`. In the
template it is a **sealed interface** with one implementation per screen:

```kotlin
// :feature:notes:contracts — a pure JVM module
@Serializable
sealed interface NotesRoute : NavKey {
    @Serializable data object List : NotesRoute
    @Serializable data class Detail(val id: String) : NotesRoute
}
```

Four properties follow from that declaration:

| Property | Why |
| --- | --- |
| The compiler checks the argument | `NotesRoute.Detail("x")` cannot be called with the wrong argument type |
| The route is serializable | It survives process death and can be written to a `PendingIntent` |
| The route carries no `Context` or `Bundle` | It lives in a **pure JVM** module, so it stays testable and leak-free |
| The screen is matched by **type**, not by string | A renamed screen is a compile error in the `entryProvider`, not a runtime crash |

## 2. Where routes live

The route types live in the slice's **`contracts`** module
([04](04-vertical-slicing.md)), which is pure JVM. Two reasons:

1. The `:app` module must be able to register the screen in its
   `entryProvider`, so it needs the route type.
2. A route that other slices link to (a deep link into a screen owned by
   another slice) must be reachable without depending on that slice's
   `domain` or `data`.

The route type is **published**, the screen is **not**: `:app` holds the
`NavKey` and the composable in the same `entryProvider` block, and nothing
outside that block knows both.

## 3. The back stack is state you own

Navigation3 does not hide the back stack. The app holds it.

```kotlin
// The shape of the host. The back stack is a remembered value, not a
// controller's private field, and the saved-state configuration tells it
// how to serialize the polymorphic route hierarchy.
@Composable
fun AppNavHost() {
    val backStack = rememberNavBackStack(
        SavedStateConfiguration {
            serializersModule = SerializersModule {
                polymorphic(NavKey::class) {
                    subclass(NotesRoute.List::class, NotesRoute.List.serializer())
                    subclass(NotesRoute.Detail::class, NotesRoute.Detail.serializer())
                }
            }
        },
        NotesRoute.List,
    )
    NavDisplay(
        backStack = backStack,
        onBack = { backStack.removeLastOrNull() },
        entryProvider = entryProvider {
            entry<NotesRoute.List> { NotesScreen(state = state, onEvent = viewModel::onEvent) }
            entry<NotesRoute.Detail> { route ->
                NoteDetailScreen(state = state, noteId = NoteId(route.id))
            }
        },
    )
}
```

The `SavedStateConfiguration` is not optional boilerplate: with a sealed
route hierarchy, the runtime has to know every concrete subtype to write
the back stack into the saved state. A subtype that is missing from the
`polymorphic` block is a subtype that does not survive process death.

| Rule | Why |
| --- | --- |
| The back stack is remembered state | It survives a recomposition and it can be saved and restored |
| `removeLastOrNull()` is how a screen goes back | The stack is data; popping it is a function call |
| There is no "navigate to a string" | The compiler catches the wrong argument |
| A synthetic back stack is built explicitly | A deep link must land with a sensible parent ([§6](#6-deep-links-and-app-links)) |

**Multiple back stacks** (one per tab) are the pattern for a bottom
navigation bar: each tab owns its own stack, and switching tabs preserves
each tab's history. That is state the app must hold, which is exactly why
Navigation3's model fits.

## 4. The route table and the parity test

The route **grammar** is shared across the clients of the platform — the
web app, the extension and the Android app must agree on what a URL means.
The grammar is:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]
/app/market/products/{productId}   /app/market/cart
/app/game/map                     /app/game/encounters/{encounterId}
/app/avatar/sessions/{sessionId}   /app/notifications/preferences
Session-less: /onboarding  /login  /register  /forgot-password  /reset-password  /verify-email
```

The **route table** (a shared JSON document) is the source of truth for that
grammar. The Kotlin `NavKey` set must match it, and the **parity test**
proves it.

> **Known gap — do not copy a fiction.** The template ships the **typed
> routes**, not the route table. The table is a **cross-kit contract**: the
> web and mobile clients read the same one, and it belongs to the platform
> contract, not to this kit. The **parity test is not implemented** — it
> lands with the table.

| Rule | Why |
| --- | --- |
| **No hand-written path string in the Kotlin code** | A path string is a typo waiting to happen |
| The route builders live next to the routes | A route knows how to build its own path |
| The parity test compares the `NavKey` set against the route table | A route added to the app but not to the table fails CI |

The parity test is a **JVM test**: the routes are in a pure JVM module, so
the test needs no device and no emulator. That is a direct payoff of the
module split — a test that would need an instrumented run in a monolithic
app runs in the JVM suite.

```kotlin
// The shape of the parity test: every route in the table has a Kotlin
// counterpart, and every Kotlin route is in the table.
@Test fun `every route in the table has a nav key`() { /* ... */ }
```

### The adoption path

| Missing | Where it belongs |
| --- | --- |
| The shared route table document | The platform contract, beside the other clients' route tables |
| The parity test | `:feature:notes:contracts`, in `src/test/` — the module that owns the routes |
| The route builders | Next to the routes in `contracts` |

Until the table exists, the `notes` routes are the only ones, and the
grammar above is the platform's, not the template's.

## 5. The registration point

A new screen is registered in **one** place: the `entryProvider` in `:app`.

| Step | Where |
| --- | --- |
| Declare the route type | The slice's `contracts` module |
| Implement the screen | The slice's `ui` module |
| Register the route → screen pair | `:app`, in the `entryProvider` |
| Add the route to the route table | The shared table document |

`:app` is the only module that sees both the route and the composable, and
it is the only one allowed to pair them. A screen that registers itself
would need a mechanism to reach the host's back stack — which is exactly the
global state the architecture forbids.

## 6. Deep links and App Links

A deep link is an **untrusted input**. It arrives from outside the app: a
browser, another app, a notification, a QR code.

### App Links

App Links are the verified form: `https://<domain>/app/…` with
`android:autoVerify="true"` and an `assetlinks.json` published at the
domain. The platform verifies the domain, and the app opens only on a link
it owns.

| Piece | What it does |
| --- | --- |
| `android:autoVerify="true"` on the intent filter | Asks the platform to verify the domain |
| `assetlinks.json` on the domain | Proves the app owns the domain |
| The intent filter's path pattern | Restricts which paths enter the app |

A custom scheme (`<scheme>://app/…`) is the fallback for environments where
a domain cannot be verified. It is weaker: any app can claim the scheme, so
the payload must be treated as hostile.

### Parsing an incoming link

> **Known gap — do not copy a fiction.** The template ships **no deep-link
> parser and no intent filter**. `:feature:notes:contracts` holds the route
> types and the cross-slice interfaces; the parser, the manifest's intent
> filter and the published `assetlinks.json` are the adoption path below.

The link arrives as an `Intent`. The app parses it into a typed route through
a **parser with an allowlist**:

| Step | Rule |
| --- | --- |
| 1. Match the path against the route table | A path that is not in the table is dropped, not guessed |
| 2. Extract the parameters | Each parameter is validated against its expected type |
| 3. Build the `NavKey` | The typed value, never a string |
| 4. Enforce a size limit | A link with an oversized parameter is rejected |
| 5. Build a synthetic back stack | Up navigation lands on a sensible parent |

The parser is a **pure function** from `(path, parameters) → NavKey?`, in a
pure JVM module. That makes the fuzzing possible: the deep-link test suite
generates malformed paths and asserts that the parser returns `null` rather
than a route.

### The deep-link adoption path

| Missing | Where it belongs |
| --- | --- |
| The deep-link parser | `:feature:notes:contracts` — pure JVM, so the fuzz test needs no device |
| The intent filter with `android:autoVerify="true"` | The `:app` manifest |
| The `assetlinks.json` | The domain, published by whoever owns the domain |
| The custom scheme, if the domain cannot be verified | The `:app` manifest, plus the scheme in the route table |

### The synthetic back stack

A deep link is an entry point, not a page in the current history. Opening
`/app/game/encounters/{id}` must not leave the user pressing back into a
half-built stack.

| Rule | Example |
| --- | --- |
| The stack is built from the route hierarchy | `[EncountersRoute, EncounterDetailRoute(id)]` |
| The parent is a real screen | Pressing back lands on the list, not on the launcher |
| The stack is replaced, not appended to | A link from outside does not inherit the current history |

## 7. Untrusted parameters

Everything that arrives from outside is untrusted, whatever its source.

| Source | Trust | Rule |
| --- | --- | --- |
| A deep link | **None** | Validate against the route table; reject anything unknown |
| A notification payload | **None** | Same parser as a deep link; a push is a channel, not a data path ([10](10-push-notifications.md)) |
| A `SavedStateHandle` restored after process death | **Low** | The bundle is a cache that survived a kill; validate before use |
| A server response | **Low** | Validate the shape before it reaches the domain |
| A value the user typed | **Low** | The domain validates it; the UI validates for feedback, never as the only check |

Three rules that apply to all of them:

1. **Never perform a state-changing action from an incoming link or payload.**
   A link may open a screen that *offers* an action; the user confirms it.
2. **The URL never authorizes.** A link that names a resource the user may
   not read shows the forbidden state — the server decides.
3. **A link received while signed out is queued and resumed** after login
   and bootstrap, not dropped and not replayed blindly.

## 8. Navigation and auth guards

Guards shape the **UX**, never the authorization:

| Guard | When it fires | What it does |
| --- | --- | --- |
| `RequireAuth` | The route requires a session | Redirect to `/login`, then return to the requested route |
| `RequirePermission` | The route requires a capability | Show the forbidden state, or the request-permission flow |
| `RequireUserType` | The route belongs to another role | Show the forbidden state |

The server authorizes every call regardless of what the guard allowed. A
guard that redirects is a courtesy; a server that returns `403` is the
rule.

## Do / don't

| Do | Don't |
| --- | --- |
| Model a route as a `@Serializable` `NavKey` type | Write a path string and parse it at runtime |
| Register the route in `:app`'s `entryProvider` | Let the screen register itself |
| Validate an incoming link against the route table | Trust the path shape |
| Build a synthetic back stack for a deep link | Append the route to the current history |
| Require a confirmation for a state-changing action from a link | Act on the tap |
| Write the parity test in the JVM suite | Test the routes on a device |

## See also

- [Playbooks](13-playbooks.md) — the recipe for adding a route.
- [ADR 0002 — Navigation3 typed NavKeys](adr/0002-navigation3-typed-navkeys.md)
  — why routes are values.
- [Push Notifications](10-push-notifications.md) — the other untrusted input.
- [Playbook Native Android Kotlin §6](../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md)
  — the route grammar and the parity test.
