# PLAYBOOK_URLS.md — URL and route contract (Rust backend)

Route-naming contract for the `<app>` HTTP API. The one rule that carries the
whole file: **the URL never authorizes.** The prefix (`/admin`, `/auth`) is an
experience namespace; permissions come from the verified JWT and are enforced
by middleware plus a handler-side `can(role, action)` check.

Premise for actors and roles: `MEMORY.md`.

---

## 1. The model

`<app>` has no `{userType}` segment and no tenant segment.

- **Two API namespaces plus shared auth.** `/api/v1/admin/...` serves the
  admin console; `/api/v1/...` serves every other client; `/api/v1/auth/...`
  is shared.
- **One role set per user**, ordered
  `Viewer < Editor < Admin < Owner` (`<app>-core/src/rbac.rs`). The DB enum
  `GUEST/MODERATOR/USER/ADMIN` maps onto it (`GUEST → Viewer`); RBAC reads the
  role, never `user_type`.
- **Path syntax is axum 0.8**: `{id}` placeholders, one `Router::route` call
  per path with method chaining (`get(..).post(..)`). The older `:id` syntax
  panics in axum 0.8 — do not use it.
- **The spec is generated from the code.** Every handler carries
  `#[utoipa::path(...)]` and is registered in `ApiDoc`'s `paths(...)` list in
  `<app>-api/src/router.rs`. A route missing from that list is invisible to
  `/docs` and to the committed `openapi.json`; `just gen-check` is the guard.

---

## 2. Canonical shape

```text
/api/v1/admin/{module}/{resource}                     admin list
/api/v1/admin/{module}/{resource}/{id}                admin detail
/api/v1/admin/{module}/{resource}/{id}/{sub}          admin nested
/api/v1/{module}/{resource}[/{id}[/{sub}]]            user surface
/api/v1/auth/{action}                                 shared auth
/healthz, /readyz                                     health, outside the API
/api/v1/ws                                            WebSocket upgrade
```

### 2.1 Route inventory (shipped)

```text
Health (outside auth)
GET    /healthz
GET    /readyz

Transport
GET    /api/v1/ws                          WebSocket, auth AFTER upgrade (?token=)

Auth — public
POST   /api/v1/auth/signup
POST   /api/v1/auth/verify
POST   /api/v1/auth/verify/resend
POST   /api/v1/auth/login
POST   /api/v1/auth/refresh
POST   /api/v1/auth/forgot-password
POST   /api/v1/auth/validate-code
POST   /api/v1/auth/reset-password
POST   /api/v1/auth/google
GET    /api/v1/auth/github/login
GET    /api/v1/auth/github/callback
POST   /api/v1/auth/upgrade-to-admin       bootstrap whitelist only

Auth — authenticated
POST   /api/v1/auth/logout
GET    /api/v1/auth/me
PUT    /api/v1/auth/me
GET    /api/v1/auth/sessions
GET    /api/v1/auth/sessions/current
POST   /api/v1/auth/sessions/revoke-current
POST   /api/v1/auth/sessions/revoke-all
POST   /api/v1/auth/change-password
GET    /api/v1/auth/github/connect-url

User surface — authenticated
GET    /api/v1/widgets
POST   /api/v1/widgets
GET    /api/v1/widgets/{id}
PUT    /api/v1/widgets/{id}
DELETE /api/v1/widgets/{id}
GET    /api/v1/widgets/events              SSE

Admin surface — authenticated + Admin rank
GET    /api/v1/admin/users
GET    /api/v1/admin/users/{id}
PUT    /api/v1/admin/users/{id}
GET    /api/v1/admin/auth-session/users/{id}/sessions
POST   /api/v1/admin/auth-session/users/{id}/revoke-all
GET    /api/v1/admin/auth-session/anomalies

Infrastructure
GET    /metrics                            feature `metrics`, unguarded — see §7
GET    /docs, /docs/openapi.json           Swagger UI, unguarded — see §7
```

A new slice extends the same shape:

```text
GET    /api/v1/orders
GET    /api/v1/orders/{id}
POST   /api/v1/orders/{id}/cancel          domain action, not CRUD
GET    /api/v1/admin/orders
```

Rules:

- **Plural resources.** `/users`, `/sessions`, `/widgets`. A singular segment
  is an action: `POST /api/v1/auth/login`.
- **kebab-case** for multi-word segments (`auth-session`, `audit-logs`).
- **IDs in the path, filters in the query.**
  `/api/v1/admin/users?role=ADMIN&limit=50`, never `/users/by-role/ADMIN`.
- **No verbs when the method says it.** Reserved exceptions are domain actions
  that are not CRUD: `/auth/sessions/revoke-all`,
  `/admin/auth-session/users/{id}/revoke-all`, `/orders/{id}/cancel`.
- **One version segment**, `/api/v1`, shared by every slice. Additive changes
  stay in `v1`; a breaking change creates `/api/v2` with a deprecation window.
- **No trailing slash.**
- **Pagination on every list**: `limit` + `offset` (or a cursor) with a hard
  maximum. The shipped `GET /widgets` returns an unbounded `Vec<Widget>` —
  fine for the example slice, not for a real collection; add pagination before
  the collection grows.
- **The prefix classifies; it never authorizes.** Every protected route is
  mounted behind an auth middleware, `/api/v1/admin/*` additionally behind the
  Admin rank gate, and the handler re-checks the action.
- **Status codes**: `201` + the created body on create, `204` on a successful
  delete/revoke, `409` on a uniqueness conflict, `422` is not used — invalid
  input is `400`.

---

## 3. Two auth middlewares — pick deliberately

| Middleware | Carriers | Used by |
|---|---|---|
| `<app>_warden::require_auth` | `Authorization: Bearer` only | routes that are API-client-only |
| `crate::auth::require_auth_cookie` | `Authorization: Bearer`, then the `<app>_access_token` cookie when `<APP>_AUTH_COOKIE` mode is on | identity, admin, SSE |

Both attach a `Principal { id, roles }` to the request extensions and answer a
401 envelope on failure. The cookie fallback is what lets a browser SPA rely on
the `HttpOnly; SameSite=Strict` cookie; a bearer-only route silently excludes
that client. Choose `require_auth_cookie` for anything a browser calls.

Authorization is never only the middleware:

- a handler calls `require_action(&principal, Action::…)` (widgets) or the
  admin rank gate (`require_admin`), which reads the token's roles;
- a multi-role principal is allowed if **any** role permits the action;
- the admin rank gate is strict `>` against the threshold (an actor cannot act
  on a peer of equal rank);
- an authenticated but disallowed caller gets `403`; a foreign or unowned row
  gets `404`, never `403`.

---

## 4. Error envelope

Every error response — API, auth middleware, 404 fallback — uses one shape:

```json
{
  "error": {
    "code": "forbidden",
    "title": "Forbidden",
    "detail": "role(s) [Viewer] may not Create widgets",
    "request_id": "req_0192f3…",
    "extra": {}
  }
}
```

| Field | Rule |
|---|---|
| `code` | stable snake_case string; clients branch on it |
| `title` | short human label for the code |
| `detail` | human-readable; 5xx is redacted to `"An internal error occurred."` |
| `request_id` | present when the request-id layer ran; same value as `x-request-id` |
| `extra` | optional object: `retry_after_secs`, quota counters |

Codes: `not_found`, `invalid_request`, `unauthorized`, `unauthenticated`,
`forbidden`, `conflict`, `rate_limited`, `quota_exceeded`, `upstream_failure`,
`internal_error`.

Additional headers: `401` carries `WWW-Authenticate: Bearer realm="<app>-api"`;
`429` carries `Retry-After` matching `extra.retry_after_secs`.

The mapper lives in `<app>-api/src/error.rs`; the warden re-exports the same
envelope types so the middleware cannot drift. Never hand-build a JSON error in
a handler.

---

## 5. Streaming routes

Streams are declared paths, never a query flag that changes the response shape:

```text
GET /api/v1/widgets/events     SSE, normal bearer-or-cookie auth
GET /api/v1/ws                 WebSocket upgrade, auth after upgrade via ?token=
```

- SSE frames are `event: <type>` + `data: <json>` with `retry: 2000`; a
  heartbeat keeps proxies from idling the connection out.
- The WebSocket handshake cannot carry an `Authorization` header from a
  browser, so the route reads `?token=` **after** the upgrade and closes with
  `4001 / authentication_failed` when the token is missing or invalid. The
  token is the short-lived access token, never the refresh token.
- **Never log the query string of `/api/v1/ws`.** The tracing middleware logs
  `uri`, which includes `?token=`; redact it at the span (see
  `PLAYBOOK_SECURITY.md` §4.2).
- A stream never widens visibility: it delivers only what the same identity
  could read through the REST surface.

---

## 6. Template deviations — fix on instantiation

The shipped skeleton does not fully satisfy the contract above. These are
known defects carried from the template, not design decisions. Each has a
mechanical fix; do them in the first instantiation PR.

| # | Deviation | Why it matters | Fix |
|---|---|---|---|
| D1 | Widget CRUD is mounted at `/widgets` and `/widgets/{id}`, not `/api/v1/widgets` (only SSE uses the prefix) | every other resource lives under `/api/v1`; clients and the SDK cannot assume one base path | change the two `Router::route` paths and the `#[utoipa::path(path = …)]` attributes in `routes/widgets.rs`; update the `ROUTE_TABLE` test and regenerate `openapi.json` |
| D2 | `GET /me` is mounted at the root as a legacy seam, duplicating `GET /api/v1/auth/me` | two identity endpoints with different bodies confuse every client | delete the `me` handler, `MeResponse`, `me_routes` and their tests; `auth_me` is the only identity read |
| D3 | Widget CRUD uses bearer-only `require_auth` | cookie-mode SPAs receive 401 on every widget call | mount the widget sub-router behind `require_auth_cookie` (it already has `AppState`) |
| D4 | `GET /widgets` is unpaginated | unbounded response and unbounded DB read | add `limit`/`offset` with a hard maximum before the slice becomes a real collection |
| D5 | `/metrics` and `/docs` are mounted unguarded | leaks traffic shape and the full surface area | gate both behind an internal listener or the admin rank check; disable Swagger in production builds |
| D6 | `docs/02-architecture.md`, `docs/08-persistence-and-migrations.md`, `docs/09-testing-strategy.md` describe a different stack (`query_as!`, `.sqlx/`, testcontainers, wiremock, toolchain 1.85) | an agent following the docs writes code that does not fit the workspace | treat `PLAYBOOK_*.md` + `AGENTS.md` as authoritative; correct the docs in the same PR |

---

## 7. Checklist

- [ ] Admin routes under `/api/v1/admin/...`; user routes under `/api/v1/...`;
      shared auth under `/api/v1/auth/...`; no resource at the root.
- [ ] No `{userType}`, no `{tenantId}`, no `X-Tenant-Id` header.
- [ ] Plural resources, kebab-case, IDs in path, filters in query.
- [ ] Lists paginated with a hard maximum page size.
- [ ] Non-CRUD domain actions are verbs on a sub-resource.
- [ ] Every route registered in `ApiDoc` `paths(...)` and `just gen-check` green.
- [ ] Protected routes mounted behind the middleware a browser can satisfy
      (`require_auth_cookie`).
- [ ] Handler-side `can(role, action)` / rank gate present for every
      mutation, not only the route layer.
- [ ] A foreign or unowned row answers `404`.
- [ ] Errors use `ApiError` and the `{"error": {…}}` envelope.
- [ ] Streams are declared paths; the WS `?token=` never reaches the log.
- [ ] `/metrics` and `/docs` restricted before production exposure.
