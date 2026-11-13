---
name: "add-route"
description: "Adds a route to the <app> mobile route table: the grammar-conforming entry, the typed builder, the parameter validation and the contract test that proves the path round-trips."
division: core
---

# `<app>` add-route skill

> Reusable workflow for adding one route to the typed route table of
> `<app>`. An agent invokes this skill; the requester supplies the screen and
> its parameters, the skill supplies the entry, the builder, the validation
> and the test.

## When to Activate

Activate this skill when the request is to make a screen reachable: a new
resource, a new collection, a new detail path, a new session-less route, or
a change to the parameters of an existing one.

Do NOT activate this skill when:

- The screen does not exist yet — add the screen first (use `add-screen`).
- The request is to change the grammar itself; that is a change to
  `packages/contracts/src/routes.ts` and to the URL playbook together.
- The path is being consumed by the backend only, with no screen on this
  side.

## Core Principles

- **One table, one grammar, one parser.** Every path in the app is written
  down in `packages/contracts/src/routes.ts`; the table, the builders and the
  parser live together.
- **The grammar is fixed.** `/{userType}/{module}/{resource}[/{resourceId}]`,
  with the session-less routes (`/onboarding`, `/login`, `/register`,
  `/forgot-password`, `/reset-password`, `/verify-email`) outside it.
- **The builder is typed.** A missing parameter is a compile error, and an
  invalid resource id is refused at build time, not at click time.
- **The path is never typed by hand.** A notification payload, a view-model
  command and a test all call the same builder.
- **The URL never authorizes.** The `userType` segment is an experience
  namespace; the server decides what a session may do.

## Rules

- No fabricated paths: the route entry names the module, the resource and the
  identifier shape that really exist.
- The entry declares the four fields `RouteSpec` really has — `userType`,
  `module`, `resource` and `byId` — and `byId` says whether the route accepts
  a resource id; a route with `byId: true` and no id validation is
  incomplete. A route with no resource at all belongs to the session-less
  list (`SESSIONLESS_ROUTES`), not to the table.
- The resource id is validated against the same rule the parser uses, so a
  builder and a parser agree.
- An incoming path — a deep link or a push payload — is parsed through the
  table, never trusted because it looks well formed; an unknown route or an
  invalid parameter falls back to the home route.
- A private route opened without a session is queued, not redirected to
  `/login` with the destination lost.
- The contract test proves the round trip: the builder's output parses back
  to the same route, and a path with a bad parameter is refused.
- No step is "done" until `pnpm run typecheck`, `pnpm run lint`,
  `pnpm run test` and `pnpm run export:android` are green; report the real
  output.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The grammar, the table, the builders and the parser | `packages/contracts/src/routes.ts` | 2026 | local file |
| The route rules for the mobile client | `PLAYBOOK_FRONTEND_MOBILE.md` §6 | 2026 | local file |
| The deep-link rules and the untrusted-input rule | `docs/07-navigation-routes-and-deep-links.md` | 2026 | local file |
| The route contract test to extend | `tests/contracts/routes.test.ts` | 2026 | local file |
| The universal link and app link forms | Expo Linking documentation | 2026 | https://docs.expo.dev/versions/latest/sdk/linking/ |
| The URL playbook the grammar comes from | `PLAYBOOK_URLS.md` | 2026 | local file |

## Output Format

```
Route: <module>/<resource> — path: packages/contracts/src/routes.ts
Entry: <userType> · <module> · <resource> · byId: <yes|no>
Builder: <the exported function and its signature>
Parameters: <name> — <validation rule>
Parser behaviour: <what parseRoute returns for this path>
Fallback: <where an unknown or invalid path lands>
Test: tests/contracts/routes.test.ts — <the round-trip and the refusal case>
Gates:
  pnpm run typecheck → <result>
  pnpm run lint → <result>
  pnpm run test → <result>
  pnpm run export:android → <result>
Not done: <anything deliberately left out>
```
