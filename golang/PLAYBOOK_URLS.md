# PLAYBOOK_URLS.md — URL and route naming playbook (Go backend)

Route-naming contract for the `<app>` HTTP API. The one rule that carries the
whole file: **the URL never authorizes.** The prefix (`/admin`, `/auth`) is an
experience namespace; permissions come from the session token and are enforced
in the backend.

Premise for actors and slices: `MEMORY.md`.

## 1. The model

`<app>` has no `{userType}` segment and no tenant segment.

- **Two API namespaces plus shared auth.** `/api/v1/admin/...` serves the
  admin console, `/api/v1/...` serves the user console and other clients,
  `/api/v1/auth/...` is shared by every client.
- **One backend role per user**: `ADMIN`, `MODERATOR`, `USER`, `GUEST`
  (`internal/shared/rolex/rolex.go`). There is no organization switch and no
  tenant id to carry in a path, header or query.
- `users.user_type` is a signup/profile classification, not a route
  namespace; RBAC reads `role`. Do not map `user_type` onto URL segments or
  use it as a guard.
- **Route builders use Go 1.22+ ServeMux patterns**: `{id}` placeholders,
  `GET /path/{id}` method-first spelling. The mux is declared in each slice's
  `module.go` / `http.go`.

## 2. API: `/api/v1/{namespace}/{module}/{resource...}`

```text
/api/v1/admin/{module}/{resource}                              admin list
/api/v1/admin/{module}/{resource}/{id}                         admin detail
/api/v1/admin/{module}/{resource}/{id}/{subresource}           admin nested
/api/v1/{module}/{resource}[/{id}[/{subresource}]]             user surface
/api/v1/auth/{action}                                          shared auth
```

### 2.1 Route inventory (shipped in the skeleton)

```text
Auth (shared)
POST   /api/v1/auth/signup
POST   /api/v1/auth/verify
POST   /api/v1/auth/verify/resend
POST   /api/v1/auth/login
POST   /api/v1/auth/refresh
POST   /api/v1/auth/logout
GET    /api/v1/auth/me
PUT    /api/v1/auth/me
GET    /api/v1/auth/sessions
GET    /api/v1/auth/sessions/current
POST   /api/v1/auth/sessions/revoke-current
POST   /api/v1/auth/sessions/revoke-all
POST   /api/v1/auth/forgot-password
POST   /api/v1/auth/validate-code
POST   /api/v1/auth/reset-password
POST   /api/v1/auth/change-password
POST   /api/v1/auth/upgrade-to-admin
POST   /api/v1/auth/google
GET    /api/v1/auth/github/login
GET    /api/v1/auth/github/callback
GET    /api/v1/auth/github/connect-url

Admin surface
GET    /api/v1/admin/users
GET    /api/v1/admin/users/{id}
PUT    /api/v1/admin/users/{id}
GET    /api/v1/admin/auth-session/users/{id}/sessions
POST   /api/v1/admin/auth-session/users/{id}/revoke-all
GET    /api/v1/admin/auth-session/anomalies

User surface
GET    /api/v1/widgets
POST   /api/v1/widgets
GET    /api/v1/widgets/{id}
PUT    /api/v1/widgets/{id}
DELETE /api/v1/widgets/{id}
GET    /api/v1/widgets/events          SSE

Transport
GET    /api/v1/ws                      WebSocket (?token=)

Health (outside auth)
GET    /healthz
GET    /readyz
```

A new slice adds its routes to the same shape:

```text
GET    /api/v1/orders
GET    /api/v1/orders/{id}
POST   /api/v1/orders/{id}/cancel     domain action, not CRUD
GET    /api/v1/admin/orders
```

Rules:

- **Plural resources.** `/users`, `/sessions`, `/widgets`. A singular segment
  is an action, not a collection: `POST /api/v1/auth/login`.
- **kebab-case** for multi-word segments (`auth-session`, `audit-logs`) unless
  the existing slice name is a single word (`adminusers` → `/admin/users`).
- **IDs in the path, filters in the query.** `/admin/users?role=ADMIN&limit=50`,
  never `/admin/users/by-role/ADMIN`.
- **No verbs when the method says it.** `DELETE /api/v1/widgets/{id}`, not
  `POST /api/v1/widgets/{id}/delete`. Reserved exceptions are domain actions
  that are not CRUD: `/auth/sessions/revoke-all`,
  `/admin/auth-session/users/{id}/revoke-all`, `/orders/{id}/cancel`.
- **One version segment**, `/api/v1`, shared by every slice. No per-surface
  version and no version in a header. A breaking change creates `/api/v2` and
  a deprecation window; additive changes stay in `v1`.
- **No trailing slash** in declared patterns.
- **Pagination** on every list: `limit` and `offset` (or a cursor), with a
  hard maximum page size. Never an unbounded list.
- **The prefix classifies; it never authorizes.** Every user-surface route
  mounts behind `authx.RequireAuth`, and `/api/v1/admin/*` additionally
  requires `authx.RequireRole(rolex.Admin)`. Owner-scoped reads answer `404`,
  not `403`, for a foreign or unowned resource.

## 3. Streaming routes

Streams are declared paths, never query flags that change the response shape:

```text
GET /api/v1/widgets/events        SSE, auth via RequireAuth
GET /api/v1/ws                    WebSocket upgrade, auth via ?token=
```

- SSE uses the normal bearer path; EventSource clients may use cookie mode
  (`<APP>_AUTH_COOKIE`) where the browser cannot set a header.
- The WebSocket upgrade cannot carry headers from a browser, so it reads
  `?token=` and accepts only the short-lived access token. The server must not
  log the query string on that route. See `PLAYBOOK_SECURITY.md` §4.2.
- A stream never widens visibility: it delivers only data the authenticated
  identity could already read.

## 4. Authorization rules

- The `admin` prefix selects the experience; it does not grant access. Every
  admin route is registered behind `authx.RequireRole(rolex.Admin)`, and the
  use case re-checks the actor.
- The backend checks the token, the role, and the resource ownership. The
  user-facing path `/api/v1/widgets/{id}` does not mean "my widget"; the
  handler resolves the caller's `authx.Identity` and rejects other users'
  rows.
- An admin route and its user route are separate registrations, not one
  handler switching on prefix.
- A client may hide a link for a role; that is cosmetic and never the control.

Error envelope — this project's shape:

```json
{
  "detail": "forbidden: admin role required",
  "code": "FORBIDDEN"
}
```

Codes come from `internal/shared/apperr`; `detail` is human-readable and
carries no internal detail. A foreign or unowned resource answers `404` with
the not-found code.

## 5. Checklist

- [ ] Admin API routes live under `/api/v1/admin/...`; user routes under
      `/api/v1/...`; shared auth under `/api/v1/auth/...`.
- [ ] No `{userType}` segment, no `{tenantId}`, no `X-Tenant-Id` header.
- [ ] Plural resources, kebab-case segments, IDs in path, filters in query.
- [ ] Lists are paginated with a hard maximum page size.
- [ ] Domain actions that are not CRUD are verbs on a sub-resource.
- [ ] Authorization is enforced by the backend guard and the use case; the URL
      prefix is never the check.
- [ ] A foreign row answers `404`, never `403`.
- [ ] Errors use `{ detail, code }` from `apperr`.
- [ ] Streams are declared paths (`/events`, `/ws`), not query flags.
- [ ] The WebSocket route never logs its `?token=` query string.
