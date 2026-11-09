# Auth Architecture

The full authentication/authorization core the skeleton ships. Parity design
with the reference identity core: bcrypt+pepper passwords, JWT access tokens
over rotating opaque refresh sessions, verification and reset codes, a role
hierarchy, rate limits, and an audit trail.

## 1. Passwords: bcrypt over a pepper pre-hash

`platform/authx` hashes with **bcrypt(cost 10)** over `base64(HMAC-SHA256(password
bytes, PEPPER))`:

```go
pre := hmac.New(sha256.New, []byte(pepper))
pre.Write([]byte(password))
hash, err := bcrypt.GenerateFromPassword(encodeB64(pre.Sum(nil)), 10)
```

Why: the HMAC pre-hash normalizes to exactly 32 bytes — sidestepping bcrypt's
72-byte input limit and keeping bcrypt's slowness. **Pepper is optional**: an
empty `PEPPER` means the pre-hash is just the raw password bytes (bcrypt's
72-byte limit then applies as documented) — the pepper layer is disabled, not
secret-zero-ed. The pepper lives in env config, never in the DB, so a
database dump alone cannot verify passwords.

## 2. Tokens and sessions: JWT + rotating refresh

| Token | Form | TTL | Stored? |
|---|---|---|---|
| Access | JWT HS256, claims `sub/email/role/exp/jti` (`golang-jwt/jwt/v5`) | **15m** | never |
| Refresh | opaque base64url 48B from `crypto/rand` | **720h** | `sessions.refresh_token` (unique) |
| Verify code | 6 digits | **24h** | `email_verifications` |
| Reset code | 6 digits | **30m** | `password_resets` |

Response shape: `{access_token, refresh_token, token_type: "bearer", expires_in}`.
`platform/authx` owns the `Minter` interface (`Mint(identity, ttl) → token, exp`)
and `HS256Verifier.Parse`. Bearer `Authorization` header is primary;
`<APP>_AUTH_COOKIE` switches on the HttpOnly/Secure/SameSite=strict cookie
mode (off = header only).

## 3. Signup & verification flow

1. `POST /api/v1/auth/signup` — create user unverified, enqueue verification
   email job, 201. **Idempotent**: re-signup of an existing *unverified* email
   re-issues the code and re-enqueues the mail (201); an already-verified
   email is a conflict (`AUTH_EMAIL_TAKEN` / `AUTH_EMAIL_TAKEN_ACTIVE`).
2. `POST /api/v1/auth/verify` — 6-digit code against `email_verifications`
   (24h, single-use), user becomes `is_verified`. Resend issues a fresh code.

## 4. Login, gating, refresh, logout

Login order (reference parity): the password verifies FIRST — a wrong
password answers one constant 401 `AUTH_INVALID_CREDENTIALS` and increments
`retry_count` even on gated accounts, so the state codes below are
unreachable with a wrong password and no probe can infer an account's
existence or state (spec invariant). A wrong password on an account whose
`google_id`/`github_id` is set instead answers 403
`AUTH_SIGNED_UP_WITH_SOCIAL` (after the attempt is counted — brute-forcing a
social account still locks); a password that was later set still wins on a
correct try. Only then the state gates fire, each a stable code in the
envelope: `is_active=false` → 403
`AUTH_ACCOUNT_INACTIVE`; `is_blocked` → 403 `AUTH_ACCOUNT_BLOCKED`;
`!is_verified` → 403 `AUTH_EMAIL_NOT_VERIFIED`. Success resets the counter,
sets `last_login`, mints tokens and inserts a session row (IP + user agent for the rotation path). Refresh
**rotates in place** (reference `update_tokens` parity): the session row keeps
its id and `expires_at` — the absolute lifetime is fixed at login and never
slides — while the refresh token, ip/ua and `last_activity` swap under a
compare-and-swap on the presented token, so a replayed token mints nothing.
The access token's `jti == session id`, so the identity survives rotation
unchanged. Logout revokes the session; reset (below) revokes *all*.

## 5. Password reset & change

- `forgot-password` — issues a 6-digit code (30m) via the outbox;
  **rate-limited 3 requests / 5 min per email** (`AUTH_RATE_LIMITED`).
- `validate` — check code without consuming.
- `reset` — code + new password: sets it, **auto-verifies the email**, and
  revokes all sessions of the user (rotation hygiene after a compromise).
- `change-password` (authed) — verify current password, set the new one,
  enqueue the `password_changed` notification email, revoke *other* sessions
  (keeping the current one by its `jti`-matched refresh).

## 6. Roles: one hierarchy, one definition

`internal/shared/rolex` is the only place `Role` exists:

```go
type Role string
const (Guest Role = "GUEST"; User = "USER"; Moderator = "MODERATOR"; Admin = "ADMIN")
func AtLeast(r, min Role) bool // ADMIN ⊃ MODERATOR ⊃ USER ⊃ GUEST
```

`authx.Identity{ID, Email, Role rolex.Role}` rides the request context;
`authx.RequireAuth(verifier)` gates, `authx.RequireRole(rolex.Admin)` raises
the floor (`AtLeast` — anything at-or-above). DB columns (`role`, `user_type`)
carry the enum checks; the Go type is the single source for checks.

The write-path rule lives next door in `internal/shared/authz`: to manage an
account the actor must be **self or a strict subordinate** —

```go
z := authz.Context{SubjectID: actor.ID, SubjectRole: actor.Role,
    TargetID: target.ID, TargetRole: rolex.Role(target.Role)}
z.CanManage() // CanManageSelf() || CanManageSubordinate()
```

`CanManageSubordinate` is strict (`Rank(subject) > Rank(target)`) — an ADMIN
cannot reach another ADMIN's row — and `CanManageSelf` re-ranks *the row*: a
token minted before a demotion answers false, so a stale token never outranks
the DB role. Failure is `authz.ErrForbidden` → 403 `AUTH_FORBIDDEN`.

## 7. Admin surface

`GET /api/v1/admin/users?limit&offset` (limit/offset pagination),
`GET /api/v1/admin/users/{id}`, `PUT /api/v1/admin/users/{id}` (`is_active`,
`is_blocked`, `role` via `rolex.Valid`). All sit behind
`RequireAuth + RequireRole(Admin)`, write `audit_logs` rows
(actor, action, entity, payload, ip, ua), and `PUT` additionally passes the
`authz` self-or-subordinate guard (§6) — reaching another row at or above your
rank answers 403.

The auth-session admin surface (`authsession` slice) reads and revokes any
user's sessions: `GET /api/v1/admin/auth-session/users/{id}/sessions`
(`?active_only`, bare array), `POST .../revoke-all` (`{revoked_count}` —
deactivates the ACTIVE rows and keeps the history; the reference deletes
every row, a deliberate deviation so audits survive), and
`GET /api/v1/admin/auth-session/anomalies?window_minutes=` — a bare array of
`{anomaly_type, count, details}` rows over two detectors:
`expired_but_active` (active rows already past `expires_at`, a session
count) and `multi_ip_sessions` — users whose active sessions within the
window span more than 3 distinct IPs (reported with their user ids).
`window_minutes` defaults to 60 and must be 1..43200,
else 422 `VALIDATION_ERROR`.

Own-session endpoints mirror these for the caller:
`GET /api/v1/auth/sessions` (active by default; one row per login — rotation
replaces the refresh token in place, so `active_only=false` carries revoked
and expired sessions, not rotation history), `/sessions/current` (JSON `null`
when no session is active), `/sessions/revoke-current`, `/sessions/revoke-all`.

Bootstrap: `POST /api/v1/auth/upgrade-to-admin` is the one unauthenticated
admin route (reference `upgrade_to_admin` parity). An email listed in
`<APP>_ALLOWED_ADMIN_EMAILS` promotes its own account to `role` + `user_type`
ADMIN and answers `{email, message, upgraded}`; `upgraded: false` reads as
"already an admin". The whitelist is checked before any user lookup, so a
non-whitelisted caller cannot probe for account existence (403), a
whitelisted-but-unknown email is 404, and an empty list disables the endpoint
entirely. It exists to seed the first admin — drop the variable once one
exists.

## 8. Audit, rate limits, cookies — where each lives

- Audit writes go through a narrow `Audit` port (outbox-style, never blocking
  the request path on vendor latency).
- Rate limiting for reset codes is per-email in the reset slice; a Redis
  limiter would replace it via the same function seam.
- Cookie mode, TTLs, and secrets are env keys only (`<APP>_JWT_SECRET`,
  `<APP>_PASSWORD_PEPPER`, `<APP>_ACCESS_TOKEN_TTL=15m`,
  `<APP>_REFRESH_TOKEN_TTL=720h`, `<APP>_VERIFY_CODE_TTL=24h`,
  `<APP>_RESET_CODE_TTL=30m`, `<APP>_MAX_LOGIN_RETRIES`,
  `<APP>_LOGOUT_ALL_ON_RESET`); `.env.example` documents each.
- Social sign-in is a real slice (§9); the `google_id`/`github_id` columns it
  links by ship in `0001_init.sql`.

## 9. Social sign-in: Google One Tap + GitHub

Two flows, one linking rule (provider id first, else email, else provision):

**Google** — `POST /api/v1/auth/google` with `{credential}`, the browser's One
Tap ID token. `platform/oauthx` verifies the RS256 signature against Google's
published JWKS (cached, key id matched), requires `iss` = Google's issuer,
`aud` = `<APP>_GOOGLE_CLIENT_ID`, and `email_verified`. Unconfigured client id
→ 401 envelope ("google not configured"); `<APP>_GOOGLE_ENABLED=false` is the
hard-off switch. On success the account is linked by `google_id`, else by
verified email, else provisioned (default `user_type=COMPANY`, auto-verified,
no verification email) — and the response is the standard login shape.

**GitHub** — the authorization-code flow. `GET /api/v1/auth/github/login`
answers **302** to GitHub's authorize URL (`client_id`,
`redirect_uri` = `<APP>_AUTH_GITHUB_CALLBACK_URL`, scope `user:email`);
disabled/unconfigured → 403 `AUTH_FORBIDDEN` (`<APP>_GITHUB_ENABLED` ships
`false`). The callback exchanges the code for the identity, prefers
`primary+verified` emails (filtering noreply answers, with the
`id@users.noreply.github.com` fallback), applies the same linking rule by
`github_id`, and — never answering JSON — **redirects** (302) to
`<APP>_AUTH_FRONTEND_SUCCESS#access_token=…&refresh_token=…` on success or
`?error=…&error_description=…` on any failure. A `state` of the form
`link:<jwt payload>` (purpose `github_link`, `sub` = user id) links instead of
logging in. To mint that link state, a bearer-protected
`GET /api/v1/auth/github/connect-url` returns `{url}` — the same authorize
URL with the link JWT as `state` (reference `/account/github/connect-url`
parity).

Both flows perform outbound sockets, so while `<APP>_OFFLINE=true` they refuse
with a config-style error at the request boundary (INV-T7) instead of dialing.
The provider clients sit behind ports so acceptance tests run them offline.

Reciprocal rule: a password login against an account that carries a
`google_id`/`github_id` answers 403 `AUTH_SIGNED_UP_WITH_SOCIAL` when the
password is wrong (guidance to use the provider or reset), while a password
the user later set still logs them in.

## 10. The worker janitor

`cmd/worker` (not the API) runs a janitor loop beside the email outbox: every
`<APP>_JANITOR_POLL` (default 1h) it purges `sessions` rows that are inactive
by expiry and expired `email_verifications` / `password_resets` codes.
Idempotent DELETEs, safe per replica; audit rows are never touched. See
[Request Lifecycle §6](07-request-lifecycle.md).

## See also

- [Request Lifecycle](07-request-lifecycle.md) — how the middleware carries
  identity
- [Errors, Config & Observability](10-error-config-observability.md) — the
  envelope
- [Persistence & Migrations](08-persistence-and-migrations.md) — session and
  code tables