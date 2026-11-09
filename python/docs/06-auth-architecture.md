# Auth Architecture

The full authentication/authorization core the template ships. Parity design
with the reference identity core: argon2id+pepper passwords, JWT access
tokens over rotating opaque refresh sessions, verification and reset codes, a
role hierarchy, rate limits, and an audit trail.

## 1. Passwords: argon2id over a concatenated pepper

`app/platform/authx.py` hashes with **argon2id** (`argon2-cffi`'s
`PasswordHasher`) over the password bytes with the pepper appended:

```python
class Passwords:
    """Argon2id hashing; the pepper is optional and config-supplied."""

    def __init__(self, pepper: str = "") -> None:
        self._pepper = pepper

    def hash(self, plain: str) -> str:
        return _hasher.hash(plain + self._pepper)
```

Why the pepper is optional: an empty `<APP>_PASSWORD_PEPPER` means the
concatenation is a no-op — the pepper layer is _disabled_, not
secret-zero-ed. The pepper lives in env config, never in the DB, so a
database dump alone cannot verify passwords. The pepper is applied before
hashing (not as a separate HMAC stage), so it inherits argon2's own input
handling and there is no length cap to reason about.

`verify` swallows argon2's `VerifyMismatchError` / `VerificationError` /
`InvalidHashError` and answers `False` — a malformed stored hash is a failed
verification, never an exception crossing the port. `needs_rehash` is
available for a future parameter bump but is not wired today.

## 2. Tokens and sessions: JWT + rotating refresh

| Token       | Form                                                | TTL      | Stored?                           |
| ----------- | --------------------------------------------------- | -------- | --------------------------------- |
| Access      | JWT HS256, claims `sub/email/role/iss/iat/exp` (PyJWT) | **15m**  | never                             |
| Refresh     | opaque 128-hex-char (`secrets.token_hex(64)`)       | **720h** | `sessions.refresh_token` (unique) |
| Verify code | 6 digits                                            | **24h**  | `email_verifications`             |
| Reset code  | 6 digits                                            | **30m**  | `password_resets`                 |

Response shape: `{"access_token": ..., "refresh_token": ...}` — the pair only;
there is no `token_type`/`expires_in` field in this template.
`app/platform/authx.py` owns the minter/verifier pair — `Tokens.mint_access` /
`Tokens.verify_access` (HS256-only, issuer `<app>`, requiring
`sub`/`exp`/`iss`/`role`) — plus `mint_state`/`verify_state` for the short-
lived OAuth link JWTs. Bearer `Authorization` header is the only transport;
there is no cookie mode in this template.

The refresh token is minted by `IdSource.opaque_token()` (`app/shared/idgen.py`)
— a `Protocol` the use case depends on, so tests inject a deterministic
source instead of reaching for `secrets`.

## 3. Signup & verification flow

1. `POST /api/v1/auth/signup` — create the user unverified, stage the
   verification email job in the same transaction, 201. **Idempotent**:
   re-signup of an existing _unverified_ email re-issues the code and
   re-enqueues the mail (201); an already-verified email is a conflict
   (`AUTH_EMAIL_TAKEN`, 409).
2. `POST /api/v1/auth/verify` — 6-digit code against `email_verifications`
   (24 h, single-use, 5 attempts), user becomes `is_verified`.
   `POST /api/v1/auth/verify/resend` (202) issues a fresh code.

The atomicity is the adapter's job: `PgSignupStore.register` writes the
user, the code and the `email_jobs` row and commits **once** — a crash cannot
separate the account from its verification mail.

## 4. Login, gating, refresh, logout

Login order: the password verifies FIRST — a wrong password answers one
constant 401 `AUTH_INVALID_CREDENTIALS`, and an unknown email is verified
against a fixed `_DUMMY_HASH` so the two branches take the same time (no
account-existence probe). Only then the state gates fire, each a stable code
in the envelope: `is_blocked` → 403 `AUTH_ACCOUNT_BLOCKED`; `is_active=false`
→ 403 `AUTH_ACCOUNT_INACTIVE`; `!is_verified` → 403 `AUTH_EMAIL_UNVERIFIED`.
Success records `last_login`, mints the pair and inserts a session row (IP +
user agent, from `envelope.client_ip`/`envelope.user_agent`, the first
`X-Forwarded-For` hop).

Refresh **rotates in place**: the session row keeps its id while the refresh
token and `expires_at` swap under a compare-and-swap on the presented token —
`WHERE id = :id AND refresh_token = :old_token AND is_active = true AND
expires_at > :now`. A replayed token matches zero rows
and answers 401 `AUTH_INVALID_TOKEN`. Logout deactivates the presented
session; reset (below) revokes _all_.

There is no login lockout counter in this template: `users.retry_count` is
written as `0` at signup and never incremented, and a wrong password always
answers the same constant 401.

## 5. Password reset & change

- `POST /api/v1/auth/forgot-password` — issues a 6-digit code (30 m) via the
  outbox. The route **always answers 202**: an unknown email and an
  over-limit email both return silently, so the endpoint cannot be used to
  enumerate accounts. The limit is 3 requests / 5 min per email
  (`<APP>_RESET_RATE_LIMIT`), enforced by counting `password_resets` rows.
- `POST /api/v1/auth/validate-code` — check the code without consuming it.
- `POST /api/v1/auth/reset-password` — code + new password: sets the hash
  and revokes **all** sessions of the user (rotation hygiene after a
  compromise). It does **not** auto-verify the email — verification stays
  its own flow.
- `POST /api/v1/auth/change-password` (authed) — verify the current password,
  set the new one and revoke **all** sessions, the current one included.

## 6. Roles: one hierarchy, one definition

`src/app/shared/rolex.py` is the only place `Role` exists:

```python
class Role(StrEnum):
    """Account roles, lowest privilege first."""

    GUEST = "guest"
    USER = "user"
    MODERATOR = "moderator"
    ADMIN = "admin"


def at_least(role: Role, floor: Role) -> bool:
    """True when ``role`` is at or above ``floor``."""
```

`authx.Identity{user_id, email, role Role}` is the injected identity;
`require_auth` resolves the bearer token and returns it, `require_role(floor)`
is a dependency factory that checks `at_least` and answers 403 `FORBIDDEN`.
Slices declare it as an alias —
`Admin = Annotated[Identity, Depends(require_role(Role.ADMIN))]`. The DB
column (`users.role`) is TEXT with a server default, not a native enum, so
downgrades and typed queries stay trivial.

The write-path rule lives next door in `src/app/shared/authz`: to manage an
account the actor must be **a strict subordinate of the target** —
`can_manage_subordinate(actor, target)` requires `at_least(actor, MODERATOR)`
and `rank(actor) > rank(target)`, so an ADMIN cannot reach another ADMIN's
row. `authz` also ships `can_manage` (admins manage everyone) and
`can_manage_self`; only `can_manage_subordinate` is wired into the admin
update today — the other two are available for the write paths you add.

## 7. Admin surface

`GET /api/v1/admin/users?page&page_size` (page defaults to 1, `page_size`
to 20, clamped to 100; the response is `{"items": [...], "total": n}`),
`GET /api/v1/admin/users/{id}`, `PATCH /api/v1/admin/users/{id}`
(`is_active`, `is_blocked`, `role` coerced by the `Role` StrEnum). All sit behind
`require_auth + require_role(Role.ADMIN)`, write `audit_logs` rows (actor,
action, entity, payload, ip, ua), and the `PATCH` additionally passes the
`authz` subordinate guard (§6) — reaching a row at or above your rank answers
403, and changing your own role answers 403 too.

The auth-session admin surface (`authsession` slice) reads and revokes any
user's sessions: `GET /api/v1/admin/auth-session/users/{id}/sessions` (bare
array), `POST .../revoke-all` (`{"revoked": n}` — deactivates the ACTIVE rows
and keeps the history, so audits survive), and
`GET /api/v1/admin/auth-session/anomalies` — a bare array of
`{user_id, email, distinct_ips, sessions}` rows for users whose active
sessions span more than one distinct address (`count(DISTINCT s.ip) > 1`).

Own-session endpoints mirror these for the caller:
`GET /api/v1/auth/sessions` (active rows; one per login — rotation replaces
the refresh token in place, so there is no rotation history), `/sessions/current`
(JSON `null` when no session is active), `/sessions/revoke-current`,
`/sessions/revoke-all`. `GET /api/v1/auth/me` and `PATCH /api/v1/auth/me`
(the caller's own profile fields: `first_name`, `last_name`, `language`) round
the surface out.

Bootstrap: `POST /api/v1/auth/upgrade-to-admin` is the one unauthenticated
admin route. An email listed in `<APP>_ALLOWED_ADMIN_EMAILS` promotes its own
account to `role` ADMIN and answers `{email, message, upgraded}`; `upgraded:
false` reads as "already an admin". The whitelist is checked before any user
lookup, so a non-whitelisted caller cannot probe for account existence (403),
a whitelisted-but-unknown email is 404, and an empty list disables the
endpoint entirely. It exists to seed the first admin — drop the variable once
one exists.

## 8. Audit, rate limits — where each lives

- Audit writes go through the admin store's `write_audit` port method (actor,
  action, entity, entity id, payload, ip, ua) — one narrow method, no
  blocking on a vendor.
- Rate limiting for reset codes is per-email in the reset slice, counted from
  the `password_resets` rows themselves — a Redis limiter would replace it
  through the same function seam.
- TTLs and secrets are env keys only (`<APP>_JWT_SECRET`,
  `<APP>_PASSWORD_PEPPER`, `<APP>_ACCESS_TTL_MINUTES=15`,
  `<APP>_REFRESH_TTL_HOURS=720`, `<APP>_VERIFY_CODE_TTL_HOURS=24`,
  `<APP>_VERIFY_MAX_ATTEMPTS=5`, `<APP>_RESET_TTL_MINUTES=30`,
  `<APP>_RESET_RATE_LIMIT=3`); `.env.example` documents each. Secrets are
  pydantic-validated and fail closed at `app/platform/config.py` — the server
  refuses a weak JWT secret and refuses SMTP credentials while offline.
- Social sign-in is a real slice (§9); the `google_id`/`github_id` columns it
  links by ship in the init migration.

## 9. Social sign-in: Google One Tap + GitHub

Two flows, one linking rule (provider id first, else verified email, else
provision), both orchestrated by `app/platform/oauthx.py` (the
`GoogleVerifier`, `GithubOAuth` and `ExternalIdentity` behind ports):

**Google** — `POST /api/v1/auth/google` with `{credential}`, the browser's One
Tap ID token. The verifier fetches Google's published JWKS through
`PyJWKClient` (cached), checks the RS256 signature, the `aud` =
`<APP>_GOOGLE_CLIENT_ID` and the required `sub`/`email`/`aud`/`exp` claims. An
unconfigured client id is `AUTH_OAUTH_FAILED` (503). On success the account is
linked by `google_id`, else by email — and if that email already carries a
`google_id`/`github_id`, the link is refused with `AUTH_EMAIL_TAKEN` (409) —
else provisioned (role `user`, auto-verified, no verification email) — and
the response is the standard `{access_token, refresh_token}` shape.

**GitHub** — the authorization-code flow. `GET /api/v1/auth/github/login`
answers **302** to GitHub's authorize URL (`client_id`, `redirect_uri` =
`<APP>_GITHUB_CALLBACK_URL`, scope `read:user user:email`); unconfigured or
offline → 503 `AUTH_OAUTH_FAILED`. The callback exchanges the code for the
identity, prefers `primary`+`verified` emails, applies the same linking rule
by `github_id`, and — never answering JSON — **redirects** (302) to
`<APP>_FRONTEND_SUCCESS#access_token=…&refresh_token=…` on success or
`?error=…&error_description=…` on any failure. A `state` minted as a link
JWT (purpose `github_link`, `sub` = user id, 10 min TTL) links instead of
logging in. To mint that link state, a bearer-protected
`GET /api/v1/auth/github/connect-url` returns `{url}` — the same authorize
URL with the link JWT as `state`.

Both flows perform outbound sockets. While `<APP>_OFFLINE=true` the Google
verifier and the GitHub callback exchange refuse with `AUTH_OFFLINE` (503)
before dialing, and the GitHub authorize URLs answer `AUTH_OAUTH_FAILED` (503)
when offline or unconfigured (INV-T7). The provider clients sit behind ports
so acceptance tests run them with an offline fake.

## 10. The worker janitor

`src/app/bin/worker.py` (not the API) runs a janitor loop beside the email
outbox: every `<APP>_JANITOR_POLL_SECONDS` (default 1 h) it purges `sessions`
rows that are inactive or expired and expired `email_verifications` /
`password_resets` codes. Idempotent DELETEs, safe per replica; audit rows are
never touched. See [Request Lifecycle §6](07-request-lifecycle.md).

## See also

- [Request Lifecycle](07-request-lifecycle.md) — how the dependency carries
  identity
- [Errors, Config & Observability](10-error-config-observability.md) — the
  envelope
- [Persistence & Migrations](08-persistence-and-migrations.md) — session and
  code tables
