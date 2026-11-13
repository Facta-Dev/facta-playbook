# ADR 0002 — Typed NavKey routes with Navigation3

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the navigation model of the native Android template

## Context

Android navigation has two established shapes.

The **string-route** shape is the classic one: a route is a string
(`"notes/{noteId}"`), the arguments travel in a `Bundle`, and the screen
reads them with a key like `arguments["noteId"]`. It is flexible and it is
where most navigation bugs come from:

- a **typo in the argument key** is a runtime `ClassCastException` or a
  silent `null`;
- the **argument type is not in the type**: a `String` that was supposed to
  be a `Long` arrives as a `String`;
- a **route and its screen can drift**: renaming a route string does not
  touch the screen that reads it;
- a **deep link** is parsed by hand, at runtime, from a string — the most
  common source of externally-triggered crashes.

The **typed-route** shape removes the string from the arguments and the
arguments from the string: a route is a value, and the value carries its
own type.

The template also had to satisfy three specific constraints:

1. **Deep links and push notifications are first-class.** Both arrive from
   outside the app and both must resolve to a route, with the same
   validation.
2. **The route types must live in a pure JVM module.** The route
   declarations are a *contract* between slices, and a contract module that
   depends on the Android SDK is not a contract module.
3. **The back stack must be state the app owns**, so that a deep link can
   be turned into a stack with a real parent, and so that a process
   restart can restore it.

## Decision

**Use Navigation3 (`androidx.navigation3:navigation3-runtime`, 1.2.1) with
`@Serializable` `NavKey` types as the route types, and keep the route
declarations in the slice's pure-JVM `contracts` module.**

The shape:

```kotlin
// :feature:notes:contracts — a pure JVM module
@Serializable
sealed interface NotesRoute : NavKey {
    @Serializable data object List : NotesRoute
    @Serializable data class Detail(val id: String) : NotesRoute
}
```

A sealed hierarchy is the natural shape here: a route set is closed, and the
`entryProvider` must handle every subtype, so the compiler's exhaustiveness
check is worth more than the extra line.

The rules that come with the decision:

- **A route is a type, not a string.** There is no route string in the
  codebase; a route is a value of a `NavKey` subtype.
- **The route type is `@Serializable`.** That is what lets a route survive
  a process death and be written into a `SavedStateHandle`.
- **The route declarations live in `contracts`**, a pure JVM module, so the
  route contract has no Android dependency
  ([ADR 0001](0001-one-gradle-module-per-layer.md)).
- **The route is paired with its screen in exactly one place**: the
  `entryProvider` in `:app`. A screen never registers itself.
- **The back stack is state the app owns**, held in a
  `rememberNavBackStack` and rendered by `NavDisplay`. With a sealed route
  hierarchy, the `SavedStateConfiguration` declares every concrete subtype
  in the polymorphic `SerializersModule`, so the stack survives process
  death.
- **An external URL is parsed into a route by one parser**, in
  `contracts`, against a **route table** that is the single source of the
  URL grammar. The Kotlin route set and the route table are proven to
  match by a parity test.
- **A deep link becomes a synthetic back stack**: the parser returns the
  route *and* its parent chain, so **Up** lands on a real screen instead of
  exiting the app.

The template ships the **typed routes**, the `entryProvider` and the owned
back stack. The **parser** and the **route table** are an adoption step
([06](../06-navigation-routes-and-deep-links.md) §4 and §6): the table is a
cross-kit contract, and the parser follows it.

## Alternatives considered

| Option | Why not |
| --- | --- |
| `androidx.navigation` with string routes | The classic shape. Typed arguments were added later, but the route remains a string and the argument bundle remains untyped; the drift between a route and its screen is still possible. |
| Navigation3 with string routes | The library is used, but the type safety it is chosen for is thrown away by hand. |
| Navigation3 with typed routes, but the route types in the `ui` module | The route becomes an Android type, so `contracts` can no longer publish it to another slice without dragging the UI layer along. It also breaks the pure-JVM rule of [ADR 0001](0001-one-gradle-module-per-layer.md). |
| A hand-rolled navigation model (a sealed `Route` hierarchy plus a `when` over it) | The route types are the same, but the host, the back stack, the saved state and the predictive-back handling are all re-implemented. The library does the parts that are not the app's business. |
| No navigation library: a single screen with an internal state machine | Fine for one screen; it does not survive the first external entry point, because there is no place to put a deep link. |
| A separate route DSL, one string per route, validated at runtime | The validation still happens at runtime; it just moved. |

## Consequences

- **A renamed argument is a compile error.** The parameter name is part of
  the type.
- **A wrong argument type does not compile.** `Long` cannot be passed
  where a `String` is expected, because the argument is a constructor
  parameter of the route.
- **The route contract is publishable.** Another slice can depend on
  `:feature:notes:contracts` and get the route types without the UI layer.
- **A deep link is validated in one place.** The same parser serves the
  App Links, the push notifications and the internal navigation, so there
  is one rule set rather than three.
- **A route can be persisted.** `@Serializable` means a route survives the
  process death and can be restored into the back stack.
- **The route table and the Kotlin routes can drift** — and the parity test
  is what stops it. The test is the only place that knows about both
  sides; it lives in the `contracts` module, which is a pure JVM module, so
  it runs on `./gradlew test`. The table and the test are an **adoption
  step**: the table is a cross-kit contract, and the template ships the
  typed routes without it
  ([06](../06-navigation-routes-and-deep-links.md) §4).
- **There is a cost in the `entryProvider`.** Every route must be paired
  with its screen there, and a new route means a new entry. That is the
  price of having exactly one place that knows the whole graph — and it is
  a list a reviewer can read in one screen.
- **The synthetic back stack is a rule to hold.** A deep link that opens a
  detail screen must declare the parent route, or **Up** will leave the
  app. The parser returns both, so the rule is enforced in one place.
- **The serialized form is part of the contract.** Renaming a route's
  property changes the serialized form. Since the route table is the
  shared grammar, the change goes through the parity test — which is the
  point of having it.

## See also

- [ADR 0001 — One Gradle module per layer](0001-one-gradle-module-per-layer.md)
- [Navigation Routes & Deep Links](../06-navigation-routes-and-deep-links.md)
- [Layers, Modules & UDF](../03-layers-modules-and-udf.md)
- [Vertical Slicing](../04-vertical-slicing.md)
