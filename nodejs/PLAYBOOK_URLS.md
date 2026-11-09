# PLAYBOOK_URLS.md — URL and route contract (Node backend)

Route-naming contract for the `<app>` HTTP API. The rule that carries the
whole file: **the URL never authorizes.** The prefix (`/admin`, `/auth`) is an
experience namespace; permissions come from the verified JWT and are enforced
by Fastify `preHandler` guards plus a handler-side ownership/rank check.

Premise for actors and roles: `MEMORY.md`.

---

## 1. The model

`<app>` has no `{userType}` segment and no tenant segment.

- **Two API namespaces plus shared auth.** `/api/v1/admin/...` serves the
  admin console; `/api/v1/...` serves every other client; `/api/v1/auth/...`
  is shared.
- **One role set per user**, ordered `GUEST < USER < MODERATOR < ADMIN`
  (`src/shared/rolex.ts`). `requireRole(min)` uses `atLeast` (rank ≥ min).
  Authorization reads the role, never a `user_type` claim.
- **Path syntax is Fastify 5**: `:id` placeholders. One `app.get/post/...`
  call per method+path, grouped in the slice's `module.ts`.
- **The spec and the code can drift.** `specs/openapi.yaml` is the contract,
  but the routes are hand-mounted in each `module.ts`; `make generate` only
  refreshes `src/gen/api` models. There is no route-registration guard — the
  review checklist below is the guard. Update the spec in the same PR as the
  route.

---

## 2. Canonical shape

```text
/api/v1/admin/{module}/{resource}                     admin list
/api/v1/admin/{module}/{resource}/:id                 admin detail
/api/v1/admin/{module}/{resource}/:id/{sub}           admin nested
/api/v1/{module}/{resource}[/:id[/{sub}]]             user surface
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
GET    /api/v1/widgets/:id
PUT    /api/v1/widgets/:id
DELETE /api/v1/widgets/:id
GET    /api/v1/widgets/events              SSE

Admin surface — authenticated + ADMIN rank
GET    /api/v1/admin/users
GET    /api/v1/admin/users/:id
PUT    /api/v1/admin/users/:id
GET    /api/v1/admin/auth-session/users/:id/sessions
POST   /api/v1/admin/auth-session/users/:id/revoke-all
GET    /api/v1/admin/auth-session/anomalies
```

A new slice extends the same shape:

```text
GET    /api/v1/orders
GET    /api/v1/orders/:id
POST   /api/v1/orders/:id/cancel           domain action, not CRUD
GET    /api/v1/admin/orders
```

Rules:

- **Plural resources.** `/users`, `/sessions`, `/widgets`. A singular segment
  is an action: `POST /api/v1/auth/login`.
- **kebab-case** for multi-word segments (`auth-session`).
- **IDs in the path, filters in the query.**
  `/api/v1/admin/users?role=ADMIN&limit=50`, never `/users/by-role/ADMIN`.
- **No verbs when the method says it.** Reserved exceptions are domain actions
  that are not CRUD: `/auth/sessions/revoke-all`,
  `/admin/auth-session/users/:id/revoke-all`, `/orders/:id/cancel`.
- **One version segment**, `/api/v1`, shared by every slice. Additive changes
  stay in `v1`; a breaking change creates `/api/v2` with a deprecation window.
- **No trailing slash.**
- **Pagination on every list**: `limit` (1–100, default 20) and `offset` (≥0)
  through `httpx.queryPage`. The shipped `GET /widgets` uses it; a new list
  that returns `execute()` unbounded is a defect.
- **The prefix classifies; it never authorizes.** Every protected route is
  mounted behind `requireAuth`; `/api/v1/admin/*` additionally behind
  `requireRole(Admin)`, and the handler re-checks ownership/rank.
- **Status codes**: `201` + created body on create, `204` with no body on a
  successful delete, `409` on a uniqueness conflict, `422` on shape
  validation (`httpx.decode`), `401` unauthenticated, `403` authenticated but
  forbidden, `404` for a row the caller may not see.

---

## 3. Auth postures — pick deliberately

Every route declares one posture at the mount point:

| Posture       | Mount                                                                   | Used by                                                    |
| ------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------- |
| Public        | no `preHandler`                                                         | signup, login, refresh, forgot/validate/reset, OAuth entry |
| Authenticated | `{ preHandler: auth }` where `auth = requireAuth(verifier, cookieName)` | `/auth/me`, sessions, change-password, widgets, SSE        |
| Admin         | `{ preHandler: [auth, requireRole(Admin)] }`                            | `/api/v1/admin/**`                                         |

`requireAuth` accepts `Authorization: Bearer <jwt>`, and when
`<APP>_AUTH_COOKIE=true` also the `<app>_access_token` cookie. It attaches
`req.identity` (`userId`, `role`, `sessionId`) and answers `401
AUTH_UNAUTHORIZED` on failure. Choose it for anything a browser calls.

Authorization is never only the route guard:

- `/api/v1/admin/users/:id` uses `requireRole(Admin)` **and** the shared
  `authz` guard: `canManageSelf` requires the token role to still rank
  at-or-above the row's DB role; `canManageSubordinate` requires a **strict**
  `>`. An admin cannot manage another admin, and a stale token cannot
  outrank a demoted row.
- Widget routes rely on ownership predicates in the SQL
  (`where owner_user_id = :owner`), so another user's row reads as `404`,
  never `403`.
- A multi-role future must allow the action if **any** held role permits it;
  the current single-role model ranks, it does not union.
- `POST /api/v1/auth/upgrade-to-admin` is public by design, gated only by
  `<APP>_ALLOWED_ADMIN_EMAILS` (empty disables it). Remove the route after
  bootstrap or move it behind an operator-only channel.

---

## 4. Error envelope

Every error response — handler, auth guard, 404 fallback — uses one shape:

```json
{ "detail": "widget not found", "code": "NOT_FOUND" }
```

| Field    | Rule                                                              |
| -------- | ----------------------------------------------------------------- |
| `code`   | stable constant from `src/shared/apperr.ts`; clients branch on it |
| `detail` | human-readable; 5xx is generic and never echoes driver internals  |

Codes in the shipped kernel: `AUTH_INVALID_CREDENTIALS`,
`AUTH_ACCOUNT_INACTIVE`, `AUTH_ACCOUNT_BLOCKED`, `AUTH_EMAIL_NOT_VERIFIED`,
`AUTH_INVALID_TOKEN`, `AUTH_INVALID_REFRESH`, `AUTH_UNAUTHORIZED`,
`AUTH_FORBIDDEN`, `AUTH_EMAIL_TAKEN`, `AUTH_EMAIL_TAKEN_ACTIVE`,
`AUTH_CODE_EXPIRED`, `AUTH_CODE_INVALID`, `AUTH_CODE_USED`,
`AUTH_SIGNED_UP_WITH_SOCIAL`, `AUTH_RATE_LIMITED`, `AUTH_PASSWORD_WRONG`,
`VALIDATION_ERROR`, `NOT_FOUND`, `INTERNAL_ERROR`.

The mapper lives in `src/bin/api.ts` (`setErrorHandler` +
`setNotFoundHandler`) and `src/platform/httpx.ts`. A handler throws an
`AppError`; it never builds error JSON. Adapters map driver errors once.

No `WWW-Authenticate` or `Retry-After` is emitted today. Add `Retry-After`
in the same place the `AUTH_RATE_LIMITED` error is created if a client needs
to back off automatically.

---

## 5. Streaming routes

Streams are declared paths, never a query flag that changes the response
shape:

```text
GET /api/v1/widgets/events     SSE, normal bearer-or-cookie auth
GET /api/v1/ws                 WebSocket upgrade, auth after upgrade via ?token=
```

- SSE: `text/event-stream`, `cache-control: no-cache`,
  `connection: keep-alive`, first frame `retry: 2000`, then
  `event: <type>` + `data: <json>`. The handler hijacks the reply and cancels
  its subscription on response `close`. A client disconnect is the normal
  exit — do not log it as an error.
- WebSocket: the browser handshake cannot carry an `Authorization` header, so
  `/api/v1/ws` reads `?token=` **after** the upgrade and closes with
  `4001 / authentication_failed` when the token is missing, invalid, expired
  or the issuer is wrong. The token is the short-lived access token, never
  the refresh token.
- The socket consumes only the caller's room (`user:<id>`); it never widens
  visibility beyond the REST surface.
- **Never log the query string of `/api/v1/ws`.** The shipped Fastify logger
  has no `req` serializer, so its default request line includes `?token=`.
  This is a known deviation (D1 below); fix it before exposing the socket to
  real traffic.
- Streams are single-process (see `PLAYBOOK_BACKEND.md` §8). A second replica
  serves only the clients connected to it.

---

## 6. Template deviations — fix on instantiation

Known defects carried from the template, not design decisions.

| #   | Deviation                                                                                                       | Why it matters                                            | Fix                                                                                                                    |
| --- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| D1  | Fastify's default request log includes the full URL, so `/api/v1/ws?token=…` writes the access token to the log | token theft from logs; token survives in log aggregation  | add a `serializers.req` that logs `method` + pathname only (strip the query), or disable request logging on that route |
| D2  | `specs/openapi.yaml` is not checked against the mounted routes                                                  | the contract and the code drift silently                  | add a test that walks Fastify's route table and compares method+path to the spec                                       |
| D3  | `POST /api/v1/auth/upgrade-to-admin` is public                                                                  | anyone who knows a whitelisted email can promote it       | remove after bootstrap; or require an operator secret                                                                  |
| D4  | No `Retry-After` on `AUTH_RATE_LIMITED`                                                                         | clients must guess the wait                               | add the header where the rate-limit error is emitted                                                                   |
| D5  | No `/metrics` or `/docs` route ships                                                                            | none today; if added, they are unauthenticated by default | gate them behind an internal listener or the admin rank check before production                                        |

---

## 7. Checklist

- [ ] Admin routes under `/api/v1/admin/...`; user routes under `/api/v1/...`;
      shared auth under `/api/v1/auth/...`; no resource at the root.
- [ ] No `{userType}`, no `{tenantId}`, no `X-Tenant-Id` header.
- [ ] Plural resources, kebab-case, IDs in path, filters in query.
- [ ] Lists paginated via `queryPage` with a hard maximum.
- [ ] Non-CRUD domain actions are verbs on a sub-resource.
- [ ] Every protected route is mounted behind `requireAuth`; admin routes
      behind `requireRole(Admin)` as well.
- [ ] Handler-side ownership/rank check present for every mutation, not only
      the route layer.
- [ ] A foreign or unowned row answers `404`.
- [ ] Errors are `AppError` values rendered by the root; no handler-built
      error JSON.
- [ ] `specs/openapi.yaml` updated in the same PR; `make generate` run.
- [ ] Streams are declared paths; the WS `?token=` never reaches the log.
- [ ] `upgrade-to-admin` removed or operator-gated before production.
