# Navigation Routes & Deep Links

The route-string paradigm is replaced by a **typed value**: a route is a
case of a `Hashable` enum. There is no path string to mistype, no
argument dictionary to guess, and no `NavigationLink("notes/\(id)")` that
compiles until the argument is missing.

## 1. Typed routes

A route is an enum case. In the template it is a **single enum** with one
case per screen:

```swift
// NotesContracts — a pure Swift package
public enum NotesRoute: Hashable, Sendable {
    case list
    case detail(id: String)
}
```

Four properties follow from that declaration:

| Property | Why |
| --- | --- |
| The compiler checks the argument | `NotesRoute.detail(id: "x")` cannot be called with the wrong type |
| The route is a value, not a token | It can be stored in a path, compared, and carried in a `Sendable` array |
| The route carries no view and no framework type | It lives in a **pure Swift** package, so it stays testable and leak-free |
| The screen is matched by **type**, not by string | A renamed case is a compile error in the destination switch, not a runtime crash |

> **Known gap — do not copy a fiction.** The shipped `detail` payload is a
> `String`, while the domain has a `NoteID` value type for exactly that.
> The `/add-route` rule says a payload is a typed value (`NoteID`, not
> `String`) when the domain has one, and `RootView` currently converts with
> `id.rawValue`. Narrowing the payload is a **contract change**: it touches
> the route enum, the destination and the caller.

## 2. Where routes live

The route cases live in the slice's **`Contracts`** package
([04](04-vertical-slicing.md)), which is pure Swift and depends on nothing
else. Two reasons:

1. The app target must be able to register the screen in its
   `navigationDestination`, so it needs the route type.
2. A route that other slices link to (a deep link into a screen owned by
   another slice) must be reachable without depending on that slice's
   `domain` or `data`.

The route type is **published**, the screen is **not**: the app target holds
the `NotesRoute` and the view in the same
`navigationDestination(for:)` block, and nothing outside that block knows
both.

## 3. The back stack is state you own

`NavigationStack` does not hide the back stack. The app holds it.

```swift
// The shape of the host. The path is app-owned state, not a controller's
// private field.
@MainActor
@Observable
final class Router {
    var path: [NotesRoute] = []
}
```

```swift
// RootView — the stack, the root screen and the exhaustive destination.
NavigationStack(path: $router.path) {
    NotesScreen(model: model) { id in
        router.path.append(.detail(id: id.rawValue))
    }
    .navigationDestination(for: NotesRoute.self) { route in
        switch route {
        case .list:
            EmptyView()
        case .detail(let id):
            NoteDetailPlaceholder(id: id)
        }
    }
}
```

The `switch` is the point. It is **exhaustive over the enum**, so a new
route case does not compile until every host handles it — that is the
whole safety argument, and it is a compile-time one.

| Rule | Why |
| --- | --- |
| The path is app-owned state | It survives a re-render and it can be inspected and reset |
| `router.path.append(…)` is how a screen goes back | The stack is data; pushing is a function call |
| There is no "navigate to a string" | The compiler catches the wrong argument |
| The destination switch is exhaustive | A new case is a build failure until a view exists for it |

Two facts about the shipped destination, read from the source:

| Case | What the destination renders today |
| --- | --- |
| `.list` | `EmptyView()` — the list screen is the **stack root**, not a pushed destination |
| `.detail(let id)` | `NoteDetailPlaceholder(id:)` — a placeholder, because the template has no second screen |

**Multiple back stacks** (one per tab) are the pattern for a tab bar: each
tab owns its own `Router` (or its own path), and switching tabs preserves
each tab's history. That is state the app must hold, which is exactly why
the typed path fits.

> **Known gap.** `NotesRoute` is `Hashable, Sendable` but **not `Codable`**,
> so the path cannot be written into the state restoration system. The
> back stack is in-memory only: a process death loses it. Making the route
> `Codable` is the first step of the state-restoration work
> ([01](01-platform-model-and-ios-runtime.md)).

## 4. The route grammar

The route **grammar** is shared across the clients of the platform — the web
app, the extension and the mobile apps must agree on what a URL means. The
contract lives in [PLAYBOOK_URLS.md](../PLAYBOOK_URLS.md):

```text
/{userType}/{module}/{resource}
/{userType}/{module}/{resource}/{resourceId}
/{userType}/{module}/{resource}/{resourceId}/{subresource}
```

The platform-level grammar, as the iOS playbook states it, is the same
shape with the product families filled in:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]
/app/market/products/{productId}   /app/market/cart
/app/game/map                     /app/game/encounters/{encounterId}
/app/avatar/sessions/{sessionId}   /app/notifications/preferences
Session-less: /onboarding  /login  /register  /forgot-password  /reset-password  /verify-email
```

Two rules of that contract that a deep link must not soften:

| Rule | Why |
| --- | --- |
| The `{userType}` segment is an **experience namespace**, never an authorization | `/admin` in a URL grants nothing; the backend validates the token, the tenant, the role and the permissions |
| The API mirrors the frontend shape | `/api/{version}/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]` |

The segment vocabulary is fixed by the same playbook: `/app`, `/admin`,
`/owner`, `/manager`, `/agent`, `/partner`, and `/public` for the
session-less routes. `/user` is forbidden as a generic segment.

### The route table and the parity test

The **route table** (a shared JSON document) is the source of truth for that
grammar. The `NotesRoute` enum must match it, and the **parity test**
proves it.

> **Known gap — do not copy a fiction.** The template ships the **typed
> routes**, not the route table. The table is a **cross-kit contract**: the
> web and mobile clients read the same one, and it belongs to the platform
> contract, not to this kit. The **parity test is not implemented** — there
> is no route test in the tree — and it lands with the table.

| Rule | Why |
| --- | --- |
| **No hand-written path string in the Swift code** | A path string is a typo waiting to happen |
| The route builders live next to the routes | A route knows how to build its own path |
| The parity test compares the route enum against the route table | A route added to the app but not to the table fails CI |

The parity test would be a **package test**: the routes live in a pure
Swift package, so the test needs no device and no simulator. That is a
direct payoff of the package split — a test that would need an instrumented
run in a monolithic app runs in the `swift test` suite.

### The adoption path

| Missing | Where it belongs |
| --- | --- |
| The shared route table document | The platform contract, beside the other clients' route tables |
| The parity test | `Packages/Feature/Notes/Contracts/Tests/` — the package that owns the routes |
| The route builders | Next to the routes in `Contracts` |

Until the table exists, the `notes` routes are the only ones, and the
grammar above is the platform's, not the template's.

## 5. The registration point

A new screen is registered in **one** place: the
`navigationDestination(for:)` switch in `RootView`.

| Step | Where |
| --- | --- |
| Declare the route case | The slice's `Contracts` package |
| Implement the screen | The slice's `UI` package |
| Register the route → view pair | The app target, in the `navigationDestination` switch |
| Add the route to the route table | The shared table document |

The app target is the only place that sees both the route and the view, and
it is the only one allowed to pair them. A screen that registers itself
would need a mechanism to reach the host's path — which is exactly the
global state the architecture forbids.

## 6. Deep links and Universal Links

A deep link is an **untrusted input**. It arrives from outside the app: a
browser, another app, a notification, a QR code.

### Universal Links

Universal Links are the verified form: `https://<domain>/app/…` with an
`apple-app-site-association` file published at the domain and the
**Associated Domains** entitlement listing it. The platform verifies the
domain, and the app opens only on a link it owns.

| Piece | What it does |
| --- | --- |
| The `apple-app-site-association` file on the domain | Proves the app owns the domain |
| The Associated Domains entitlement | Lists the domains the app claims |
| The entitlement's `applinks:` entry | Restricts which paths enter the app |

The same capability covers passkeys (`webcredentials`) and AutoFill, which
is why it is one entitlement and not three.

A custom URL scheme (`myapp://…`) is the fallback for environments where a
domain cannot be verified. It is weaker: any app can claim the scheme, so
the payload must be treated as hostile. The playbook's rule is to avoid
custom schemes except in development.

### Parsing an incoming link

> **Known gap — do not copy a fiction.** The template ships **no deep-link
> parser, no Associated Domains entitlement and no `onOpenURL` handler**.
> `NotesContracts` holds the route cases and the cross-slice interfaces;
> the parser, the entitlement and the published
> `apple-app-site-association` file are the adoption path below.

The link arrives as a `URL`. The app parses it into a typed route through a
**parser with an allowlist**:

| Step | Rule |
| --- | --- |
| 1. Match the path against the route table | A path that is not in the table is dropped, not guessed |
| 2. Extract the parameters | Each parameter is validated against its expected type |
| 3. Build the route value | The typed case, never a string |
| 4. Enforce a size limit | A link with an oversized parameter is rejected |
| 5. Build a synthetic back stack | Up navigation lands on a sensible parent |

The parser is a **pure function** from `(URL) -> NotesRoute?`, in a pure
Swift package. That makes the fuzzing possible: the deep-link test suite
generates malformed paths and asserts that the parser returns `nil` rather
than a route.

### The deep-link adoption path

| Missing | Where it belongs |
| --- | --- |
| The deep-link parser | `Packages/Feature/Notes/Contracts` — pure Swift, so the fuzz test needs no device |
| The Associated Domains entitlement | The app target's entitlement file, referenced from `project.yml` |
| The `apple-app-site-association` file | The domain, published by whoever owns the domain |
| The `onOpenURL` handler | The app target, next to the `NavigationStack` |

> **Known gap.** The template **declares no entitlement at all** — there is
> no `.entitlements` file in the tree, and `project.yml` does not reference
> one. Every capability the playbook asks for (Associated Domains, Push,
> Sign in with Apple, App Groups) is therefore an adoption step, not a
> setting to flip ([09](09-security-and-keychain.md)).

### The synthetic back stack

A deep link is an entry point, not a page in the current history. Opening
`/app/game/encounters/{id}` must not leave the user pressing back into a
half-built stack.

| Rule | Example |
| --- | --- |
| The stack is built from the route hierarchy | `[.list, .detail(id: id)]` |
| The parent is a real screen | Pressing back lands on the list, not on the launcher |
| The stack is replaced, not appended to | A link from outside does not inherit the current history |

## 7. Untrusted parameters

Everything that arrives from outside is untrusted, whatever its source.

| Source | Trust | Rule |
| --- | --- | --- |
| A deep link | **None** | Validate against the route table; reject anything unknown |
| A notification payload | **None** | Same parser as a deep link; a push is a channel, not a data path ([10](10-push-notifications.md)) |
| A restored scene state | **Low** | The cache survived a kill; validate before use |
| A server response | **Low** | Validate the shape before it reaches the domain |
| A value the user typed | **Low** | The domain validates it; the UI validates for feedback, never as the only check |

Three rules that apply to all of them:

1. **Never perform a state-changing action from an incoming link or
   payload.** A link may open a screen that *offers* an action; the user
   confirms it.
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

> **Known gap.** The template ships **no guard**: there is no session, no
> route table and no permission model to guard with. The three guards above
> land with the session slice ([07](07-auth-and-platform-features.md)).

## 9. Push routing

> **Known gap.** There is **no push routing** in the template: no APNs
> registration, no `UNUserNotificationCenter` delegate and no payload
> router. The rule the routing will follow is the one above — a push
> payload is parsed by the **same parser** as a deep link, into a route
> value, never into a screen by name. See
> [Push Notifications](10-push-notifications.md).

## Do / don't

| Do | Don't |
| --- | --- |
| Model a route as an enum case | Write a path string and parse it at runtime |
| Register the route in the app's `navigationDestination` | Let the screen register itself |
| Handle every case of the route enum | Add a `default:` and hide a missing case |
| Validate an incoming link against the route table | Trust the path shape |
| Build a synthetic back stack for a deep link | Append the route to the current history |
| Require a confirmation for a state-changing action from a link | Act on the tap |
| Write the parity test in the package suite | Test the routes on a device |

## See also

- [Playbooks](13-playbooks.md) — the recipe for adding a route.
- [Vertical Slicing](04-vertical-slicing.md) — where a route is published.
- [ADR 0002 — One SwiftPM package per layer](adr/0002-one-swiftpm-package-per-layer.md)
  — why the routes live in a pure package.
- [Push Notifications](10-push-notifications.md) — the other untrusted
  input.
- [PLAYBOOK_URLS.md](../PLAYBOOK_URLS.md) — the route naming contract.
- [Playbook Native iOS Swift §6](../PLAYBOOK_MOBILE_IOS_SWIFT.md) — the
  route grammar and the guards.
