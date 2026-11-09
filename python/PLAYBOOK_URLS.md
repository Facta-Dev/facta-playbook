# PLAYBOOK_URLS.md — URL and route contract (Python backend)

Route-naming contract for the `<app>` HTTP API. The rule that carries the
whole file: **the URL never authorizes.** The prefix (`/admin`, `/auth`) is
an experience namespace; permissions come from the verified JWT and are
enforced by FastAPI dependency guards plus a handler-side ownership/rank
check.

Premise for actors and roles: `MEMORY.md`.

---

## 1. The model

`<app>` has no `{userType}` segment and no tenant segment.

- **Two API namespaces plus shared auth.** `/api/v1/admin/...` serves the
  admin console; `/api/v1/...` serves every other client; `/api/v1/auth/...`
  is shared.
- **One role set per user**, ordered `guest < user < moderator < admin`
  (`Role` `StrEnum` in `src/app/shared/rolex.py`; member names are uppercase,
  wire values lowercase). `require_role(min)` uses `at_least` (rank ≥ min).
  Authorization reads the role, never a `user_type` claim.
- **Path syntax is FastAPI/Starlette**: `{id}` placeholders, snake_case, and
  the placeholder name must match the handler argument
  (`@router.get("/admin/users/{user_id}")` → `async def get_user(user_id: str, ...)`).
  One `@router.get/post/put/patch/delete` per method+path, grouped in the
  slice's `http.py` and mounted by its `module.py`.
- **Every slice mounts under the same prefix.** Each `module.py` calls
  `app.include_router(build_router(service), prefix="/api/v1")`; `/healthz`
  and `/readyz` are registered directly on the app in `src/app/bin/api.py`.
  The WebSocket is the one exception: the realtime slice declares it as
  `@app.websocket("/api/v1/ws")` because a WebSocket is not a router route.
- **The spec is generated from the app, not hand-written.**
  `specs/openapi.yaml` is produced by `make generate` and committed;
  `make gen-check` fails when the committed file is stale, so the contract
  cannot drift from the code silently. WebSockets never appear in the
  OpenAPI document — `/api/v1/ws` is covered by the acceptance tier and by
  `docs/09-realtime-and-streaming.md` instead.

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

Thirty-five HTTP operations over thirty paths, plus one WebSocket endpoint.

```text
Health (outside auth)
GET    /healthz
GET    /readyz

Transport
WS     /api/v1/ws                               WebSocket, auth AFTER upgrade (?token=)

Auth — public
POST   /api/v1/auth/signup                      201
POST   /api/v1/auth/verify
POST   /api/v1/auth/verify/resend               202
POST   /api/v1/auth/login
POST   /api/v1/auth/refresh
POST   /api/v1/auth/forgot-password             202
POST   /api/v1/auth/validate-code
POST   /api/v1/auth/reset-password
POST   /api/v1/auth/google
GET    /api/v1/auth/github/login                302 to the authorize URL
GET    /api/v1/auth/github/callback             302 to the frontend
POST   /api/v1/auth/upgrade-to-admin            bootstrap whitelist only

Auth — authenticated
POST   /api/v1/auth/logout                      204, body {refresh_token}
GET    /api/v1/auth/me
PATCH  /api/v1/auth/me
GET    /api/v1/auth/sessions                    ?refresh_token=
GET    /api/v1/auth/sessions/current            ?refresh_token=
POST   /api/v1/auth/sessions/revoke-current     body {refresh_token}
POST   /api/v1/auth/sessions/revoke-all
POST   /api/v1/auth/change-password
GET    /api/v1/auth/github/connect-url

User surface — authenticated
GET    /api/v1/widgets                          ?page=&page_size=
POST   /api/v1/widgets                          201
GET    /api/v1/widgets/events                   SSE
GET    /api/v1/widgets/{widget_id}
PUT    /api/v1/widgets/{widget_id}
DELETE /api/v1/widgets/{widget_id}             204

Admin surface — authenticated + ADMIN rank
GET    /api/v1/admin/users                      ?page=&page_size=
GET    /api/v1/admin/users/{user_id}
PATCH  /api/v1/admin/users/{user_id}
GET    /api/v1/admin/auth-session/users/{user_id}/sessions
POST   /api/v1/admin/auth-session/users/{user_id}/revoke-all
GET    /api/v1/admin/auth-session/anomalies
```

A new slice extends the same shape:

```text
GET    /api/v1/orders
GET    /api/v1/orders/{order_id}
POST   /api/v1/orders/{order_id}/cancel          domain action, not CRUD
GET    /api/v1/admin/orders
```

### 2.2 Response shapes

The envelope of a success is not uniform, and a client cannot assume one
shape:

| Route                                                    | Body                                     |
| -------------------------------------------------------- | ---------------------------------------- |
| `GET /api/v1/widgets`                                    | `{"items": [...]}` — no total            |
| `GET /api/v1/admin/users`                                | `{"items": [...], "total": n}`           |
| `GET /api/v1/auth/sessions`                              | bare array of session rows               |
| `GET /api/v1/auth/sessions/current`                      | one session row, or `null` when none     |
| `POST /api/v1/auth/sessions/revoke-all`                  | `{"revoked": n}`                         |
| `POST /api/v1/admin/auth-session/users/{id}/revoke-all`  | `{"revoked": n}`                         |
| `POST /api/v1/auth/logout`                               | empty — `204`                            |
| `DELETE /api/v1/widgets/{widget_id}`                    | empty — `204`                            |
| `POST /api/v1/auth/login`, `/refresh`, `/google`         | `{"access_token": ..., "refresh_token": ...}` |

Every session row carries a `current` boolean: it is `true` only when the
row's refresh token equals the `?refresh_token=` the caller supplied. The
admin session views always report `current: false` — the admin is not
holding the session.

### 2.3 Rules

- **Plural resources.** `/users`, `/sessions`, `/widgets`. A singular segment
  is an action: `POST /api/v1/auth/login`.
- **kebab-case** for multi-word segments (`auth-session`, `forgot-password`,
  `reset-password`, `validate-code`, `change-password`, `upgrade-to-admin`,
  `connect-url`).
- **IDs in the path, filters in the query.**
  `GET /api/v1/admin/users?page=2&page_size=50`, never `/users/by-page/2`.
- **No verbs when the method says it.** Reserved exceptions are domain
  actions that are not CRUD: `/api/v1/auth/sessions/revoke-current`,
  `/api/v1/auth/sessions/revoke-all`,
  `/api/v1/admin/auth-session/users/{user_id}/revoke-all`.
- **One version segment**, `/api/v1`, shared by every slice. Additive changes
  stay in `v1`; a breaking change creates `/api/v2` with a deprecation
  window.
- **No trailing slash.**
- **Pagination on every list**: `page` (≥ 1) and `page_size` (1–100, default
  20), both declared as `Query` constraints and re-clamped by
  `pagination()` in `src/app/platform/envelope.py`. The shipped
  `GET /widgets` and `GET /admin/users` use it; a new list that returns an
  unbounded result set is a defect. The shipped session lists are the known
  exception (D2).
- **The prefix classifies; it never authorizes.** Every protected route
  declares `IdentityDep`; `/api/v1/admin/*` additionally declares
  `require_role(Role.ADMIN)`, and the handler re-checks ownership/rank.
- **Status codes**: `201` + created body on create, `202` on work that is
  accepted and finished later (verification resend, password reset request),
  `204` with no body on a successful delete or logout, `409` on a uniqueness
  conflict, `422` on shape validation, `401` unauthenticated, `403`
  authenticated but forbidden, `404` for a row the caller may not see, `429`
  when a verification code has burned all its attempts.

---

## 3. Auth postures — pick deliberately

Every route declares one posture at the mount point:

| Posture       | Mount                                                                    | Used by                                                    |
| ------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| Public        | no identity dependency                                                   | signup, verify/resend, login, refresh, forgot/validate/reset, Google, GitHub entry, `upgrade-to-admin` |
| Authenticated | `identity: IdentityDep`                                                  | `/auth/me`, sessions, change-password, connect-url, widgets, SSE |
| Admin         | `admin: Annotated[Identity, Depends(require_role(Role.ADMIN))]`           | `/api/v1/admin/**`                                         |

`require_auth` (`src/app/platform/authx.py`) accepts
`Authorization: Bearer <jwt>` through `HTTPBearer(auto_error=False)`,
verifies the HS256 signature, the issuer and the expiry, and returns a
frozen `Identity(user_id, email, role)`. A missing header is `401
UNAUTHENTICATED`; a token the verifier rejects is `401 AUTH_INVALID_TOKEN`.
There is **no cookie fallback** — every client sends the bearer header.

```python
IdentityDep = Annotated[Identity, Depends(require_auth)]
```

Authorization is never only the route guard:

- `PATCH /api/v1/admin/users/{user_id}` uses `require_role(Role.ADMIN)`
  **and** the shared `authz` guard: `can_manage_subordinate(actor.role,
  target.role)` requires the actor to be at least `MODERATOR` and to
  **strictly** outrank the target row's DB role. An admin cannot demote
  another admin, a stale token cannot outrank a demoted row, and an actor
  cannot change their own role. Every accepted mutation writes an audit row.
- Widget routes rely on ownership predicates in the SQL
  (`where owner_user_id = :owner`), so another user's row reads as `404
  WIDGET_NOT_FOUND`, never `403`.
- `POST /api/v1/auth/upgrade-to-admin` is public by design, gated only by
  `<APP>_ALLOWED_ADMIN_EMAILS` (empty disables it). Remove the route after
  bootstrap or move it behind an operator-only channel.

---

## 4. Error envelope

Every error response — handler, dependency guard, unmatched path — uses one
shape:

```json
{ "detail": "widget not found", "code": "WIDGET_NOT_FOUND" }
```

| Field    | Rule                                                               |
| -------- | ------------------------------------------------------------------ |
| `code`   | stable constant from the `Code` enum in `src/app/shared/apperr.py`; clients branch on it |
| `detail` | human-readable; 5xx is generic and never echoes driver internals   |

Codes in the shipped enum: `VALIDATION_FAILED`, `UNAUTHENTICATED`,
`FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `INTERNAL`,
`AUTH_EMAIL_TAKEN`, `AUTH_INVALID_CREDENTIALS`, `AUTH_ACCOUNT_BLOCKED`,
`AUTH_ACCOUNT_INACTIVE`, `AUTH_EMAIL_UNVERIFIED`, `AUTH_INVALID_TOKEN`,
`AUTH_CODE_INVALID`, `AUTH_CODE_EXPIRED`, `AUTH_CODE_ATTEMPTS`,
`AUTH_OFFLINE`, `AUTH_OAUTH_FAILED`, `WIDGET_NOT_FOUND`.

Two enum members are reserved rather than raised: the tree signals a
uniqueness clash as `409 AUTH_EMAIL_TAKEN` and exhausted verification
attempts as `429 AUTH_CODE_ATTEMPTS`, so `CONFLICT` and `RATE_LIMITED` carry
no traffic today.

The mapper lives in `src/app/platform/envelope.py` (`install_error_handlers`)
and is installed once in `src/app/bin/api.py`. It handles exactly four
failure kinds:

- `AppError` → its own `status` and `code`.
- `RequestValidationError` → `422 VALIDATION_FAILED` with the fixed detail
  `request validation failed`; the field-level detail stays server-side.
- `StarletteHTTPException` → keeps its status, mapped to `NOT_FOUND` for
  `404` and `INTERNAL` for everything else.
- catch-all `Exception` → logs the trace once, answers `500
  {"detail": "internal server error", "code": "INTERNAL"}`.

A handler raises `AppError` — or the `E(code, detail, status)` helper — and
never builds error JSON. Adapters map driver errors inward once.

`GET /readyz` also uses the envelope when the database is not ready: `503`
with `code: INTERNAL`. No `WWW-Authenticate` or `Retry-After` is emitted
today.

---

## 5. Streaming routes

Streams are declared paths, never a query flag that changes the response
shape:

```text
GET /api/v1/widgets/events     SSE, bearer auth
WS  /api/v1/ws                 WebSocket upgrade, auth after upgrade via ?token=
```

- SSE: `text/event-stream`, `cache-control: no-cache`,
  `x-accel-buffering: no`. Each event is one frame,
  `data: {"type":"widget.created","widget":"..."}` — the event type travels
  **inside** the JSON payload; there is no `event:` line and no `retry:`
  line. A `: keep-alive` comment is emitted every 15 seconds of silence.
  The handler polls `request.is_disconnected()` and unbinds its hub
  subscription in a `finally:` block; a client disconnect is the normal
  exit — do not log it as an error.
- WebSocket: the browser handshake cannot carry an `Authorization` header, so
  `/api/v1/ws` reads `?token=` **after** the upgrade and closes with
  `4001 / authentication_failed` when the token is missing, invalid, expired
  or the issuer is wrong. The token is the short-lived access token, never
  the refresh token. The handshake still completes (`accept()`) before the
  close, so the client observes the close code instead of an opaque HTTP
  rejection.
- WebSocket frames: the server greets with
  `{"type":"hello","user_id":"..."}`, answers a client text frame `"ping"`
  with `{"type":"pong"}`, sends its own `{"type":"ping"}` every 30 seconds,
  and pumps each hub event as one JSON frame.
- Both streams consume only the caller's room `user:<id>`; neither widens
  visibility beyond the REST surface.
- **Never log the query string of `/api/v1/ws`.** The JSON logger in
  `src/app/platform/logging.py` emits `ts`, `level`, `logger`, `msg`, `exc`
  and `request_id` — it never prints a request path, so the token has
  nowhere to leak into. Keep it that way.
- Streams are single-process: the `Hub` in
  `src/app/platform/realtime/hub.py` is in-process. A second replica serves
  only the clients connected to it.

---

## 6. Template deviations — fix on instantiation

Known defects carried from the template, not design decisions.

| #   | Deviation                                                                                                  | Why it matters                                                        | Fix                                                                                       |
| --- | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| D1  | `/docs` and `/openapi.json` ship unauthenticated (`docs_url="/docs"`, `redoc_url=None`)                     | anyone who reaches the port reads the whole contract                  | disable `docs_url` in production, or gate both behind an internal listener               |
| D2  | The session lists are not paginated — `GET /api/v1/auth/sessions`, `GET /api/v1/auth/sessions/current` and the admin session view return every active row | an account with many sessions produces an unbounded response | add `page`/`page_size` and the `pagination()` clamp to the three session routes |
| D3  | The list envelope is not uniform: `{"items"}` for widgets, `{"items","total"}` for admin users, a bare array for sessions | a client cannot parse one shape for every list                  | pick one envelope and apply it to every list route                                       |
| D4  | `POST /api/v1/auth/upgrade-to-admin` is public                                                              | anyone who knows a whitelisted email can promote it                   | remove after bootstrap; or require an operator secret                                    |
| D5  | A native browser `EventSource` cannot authenticate against the SSE route                                    | the route requires `Authorization: Bearer` and there is no cookie fallback; `EventSource` cannot set headers | stream through `fetch`, or add a cookie posture to `require_auth`                       |
| D6  | The `StarletteHTTPException` handler maps every non-404 status to `INTERNAL`                                 | a method-not-allowed (405) answers `{"code":"INTERNAL"}` — a client mistake reads as a server fault | map the remaining 4xx statuses to their own codes                                        |
| D7  | No `Retry-After` on the rate-limited path (`429 AUTH_CODE_ATTEMPTS`)                                         | clients must guess the wait                                           | add the header where the error is raised                                                  |
| D8  | The OpenAPI document does not describe the WebSocket                                                         | the socket contract lives only in prose and in the acceptance tier   | keep the socket route in `docs/09-realtime-and-streaming.md` and pin it in an acceptance test |
| D9  | The generated contract declares `200` for the two GitHub redirect routes                                     | the runtime answers `302`; a generated client will read the wrong status | declare the redirect explicitly or keep the redirect routes out of the generated contract |

---

## 7. Checklist

- [ ] Admin routes under `/api/v1/admin/...`; user routes under
      `/api/v1/...`; shared auth under `/api/v1/auth/...`; no resource at the
      root.
- [ ] No `{userType}`, no `{tenantId}`, no `X-Tenant-Id` header.
- [ ] Plural resources, kebab-case, IDs in path, filters in query.
- [ ] Path placeholders are `{snake_case}` and match the handler argument.
- [ ] Lists paginated via `pagination()` with a hard maximum of 100.
- [ ] Non-CRUD domain actions are verbs on a sub-resource.
- [ ] Every protected route declares `IdentityDep`; admin routes declare
      `require_role(Role.ADMIN)` as well.
- [ ] Handler-side ownership/rank check present for every mutation, not only
      the route layer.
- [ ] A foreign or unowned row answers `404`.
- [ ] Errors are `AppError` values rendered by `install_error_handlers`; no
      handler-built error JSON.
- [ ] `specs/openapi.yaml` regenerated in the same PR (`make generate`);
      `make gen-check` green.
- [ ] Streams are declared paths; the WS `?token=` never reaches the log.
- [ ] `upgrade-to-admin` removed or operator-gated before production.
