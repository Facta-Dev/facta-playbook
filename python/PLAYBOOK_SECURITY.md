# PLAYBOOK_SECURITY.md — security contract (Python backend)

The security rules for `<app>`, written against the shipped code. It states
what the code actually does, where it falls short, and what a change must do
before it lands.

Premise for actors, roles and invariants: `MEMORY.md`. Route postures:
`PLAYBOOK_URLS.md`. Test evidence: `PLAYBOOK_TESTS.md`.

---

## 1. Standards and mapping

The contract follows, without claiming certification:

- **OWASP ASVS 4.0** — level 2 as the target for this API.
- **OWASP Top 10 (2021)** — the threat model in §2 is organized to hit every
  category.
- **CWE/SANS Top 25** — injection, auth bypass, IDOR, crypto misuse, race
  conditions.
- **SOC 2 / ISO 27001** — control mapping lives in `PLAYBOOK_AUDIT.md`;
  evidence in `PLAYBOOK_TESTS.md` §11.

When a rule here conflicts with a framework's wording, the stricter of the
two applies. "We are too small for that" is not a control.

---

## 2. Threat model

### 2.1 Assets and actors

| Asset                                     | Why it matters                              |
| ----------------------------------------- | ------------------------------------------- |
| Passwords and password hashes             | credential reuse, account takeover          |
| Access JWTs and refresh tokens            | full impersonation                          |
| Session rows                              | active sessions, IP/UA history              |
| Email verification / reset codes          | account takeover if guessed                 |
| `users` PII (email, name, language, photo) | privacy, phishing                          |
| `audit_logs`                              | non-repudiation and incident reconstruction |
| Admin capability                          | destructive writes, data exposure           |
| SMTP and OAuth credentials                | outbound abuse, provider account loss       |

Actors: anonymous internet, authenticated user, compromised user session,
malicious admin (insider), operator with env access, dependency attacker.

### 2.2 `<app>`-specific threats

| #   | Threat                       | Current control                                        | Residual risk                                                                          |
| --- | ---------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| T1  | Credential stuffing          | argon2id, 64 MiB / t=3 / p=4 per attempt               | no global rate limit, no CAPTCHA, no breach-password check, **no lockout**             |
| T2  | Account enumeration          | same code for unknown email and wrong password; unknown email still runs a dummy argon2 verify | `signup` and `verify/resend` answer `409 AUTH_EMAIL_TAKEN` for a verified email — that difference discloses the account |
| T3  | Brute-force on codes         | 6 digits, 5 attempts, TTL, reset limited to 3/5 min    | no IP-based limit; the verification resend path re-issues a code with no per-email cap  |
| T4  | Token theft from logs        | nginx logs `$uri`, never `$request`; uvicorn access log off in the shipped entrypoint | **the WS token still rides in `?token=`** — any access log that prints the request line captures it |
| T5  | IDOR                         | ownership predicates in SQL (`owner_user_id = :owner_id`); admin rank guard | any new query that omits the owner predicate breaks it                                |
| T6  | Refresh-token theft from DB  | rotation with a compare-and-swap on the old value      | tokens stored **plaintext**; a DB read exposes live sessions                            |
| T7  | Brute force on login         | argon2id cost + dummy verify equalizes unknown-email timing | **no retry counter, no lockout, no delay** — `users.retry_count` is inserted as 0 and never incremented |
| T8  | XSS via stored names         | JSON API only; email bodies are plain text             | clients must escape; never interpolate user text into a template without escaping       |
| T9  | CSRF                         | bearer only, no cookie is ever set                    | moving the token into a cookie reopens CSRF — that change needs SameSite + a token      |
| T10 | SQL injection                | SQLAlchemy Core `text()` with bound `:name` parameters | an f-string that interpolates a user value bypasses it                                 |
| T11 | SSRF                         | OAuth endpoints fixed; offline guard refuses sockets   | `<APP>_SMTP_HOST` is operator-controlled, not user-controlled                             |
| T12 | Admin bootstrap abuse        | `<APP>_ALLOWED_ADMIN_EMAILS` whitelist                       | route is public; promote only during bootstrap and remove it                            |
| T13 | Dependency compromise        | semver floors, `pip-audit` (`make vuln`) in CI         | **no lockfile ships (INV-T3)**; no SBOM/provenance scan                                 |
| T14 | DoS via large bodies         | edge applies nginx's default 1 MiB body cap            | **the app sets no body limit**; no per-route rate limit or concurrency cap              |
| T15 | Log injection                | JSON formatter escapes values                          | user strings still reach logs via error details; keep 5xx generic                       |

### 2.3 Process

Every change that touches auth, crypto, config, SQL, or an inbound boundary
gets:

1. a named threat from the table above (or a new row);
2. a test that fails if the control regresses;
3. a note in the PR describing the residual risk.

"No new risk" is not an analysis; name what changed.

---

## 3. Layers of defense

```text
edge (TLS, proxy limits) → middleware (CORS, headers, request id)
  → route guard (require_auth / require_role) → use case (rules)
  → adapter (parameterized SQL) → database (constraints, FKs, checks)
```

Each layer assumes the one above may fail. The database constraints are not
decoration: the unique email, the unique `refresh_token`, the FK cascades
(`sessions`, `email_verifications` and `widgets` → `users`) and the `CHECK`
constraints on `email_jobs.status` and `widgets.kind` are part of the
security model. Note two gaps: `users.role` is plain `TEXT` with no
`CHECK`, so a bad role is prevented in the domain layer only; and
`password_resets` is keyed by email with no foreign key, so a reset row is
not tied to the lifetime of the account it was issued for (the janitor
purges it once it expires).

---

## 4. Authentication

### 4.1 Passwords

- Stored as **argon2id** through argon2-cffi's `PasswordHasher` with the
  library defaults: `time_cost=3`, `memory_cost=65536` (64 MiB),
  `parallelism=4`, 16-byte salt, 32-byte hash. The hasher is a module-level
  singleton in `app.platform.authx`; do not create one per request.
- The optional `<APP>_PASSWORD_PEPPER` is **appended to the plaintext**
  (`plain + pepper`) before hashing. It is not a keyed pre-hash, and argon2
  has no 72-byte truncation to defeat, so no pre-hash is needed. Changing the
  pepper invalidates every stored hash — a migration, not a config change.
  Set it before the first signup.
- Verification is `Passwords.verify`, which turns `VerifyMismatchError`,
  `VerificationError` and `InvalidHashError` into `False`. A user-supplied
  comparison never uses `==`.
- Password policy: minimum length 8, maximum length 128 (pydantic
  `StringConstraints`), enforced at signup, reset and change. No composition
  rules. If a policy is added, cap length and reject known-breached
  passwords deliberately.
- `Passwords.needs_rehash` exists but **is never called**: changing the
  argon2 parameters will not migrate the stored hashes.
- Never log a password, even at debug level. Never put one in an error
  detail.

### 4.2 Sessions and tokens

- **Access token**: JWT HS256, `iss = jwt_issuer`, TTL
  `<APP>_ACCESS_TTL_MINUTES` (15). The verifier pins `algorithms=["HS256"]`,
  checks issuer + expiry, and requires the claims `sub`, `exp`, `iss`,
  `role`. No `alg: none`, no RS/HS confusion.
- **Refresh token**: 64 random bytes as hex, opaque, 128 characters. Stored
  in `sessions.refresh_token` (unique). Rotation is in place with a
  compare-and-swap on the old value: the update matches on the old token
  and the row is returned only if it was still active and unexpired, so a
  replay of the old token fails and the session dies.
- **Rotation extends the absolute expiry**: each rotation writes
  `expires_at = now + refresh_ttl`. A client that keeps refreshing keeps
  its session alive indefinitely. The absolute lifetime is not pinned at
  creation.
- **Transport is bearer only.** `Authorization: Bearer <jwt>` and nothing
  else. There is no `<APP>_AUTH_COOKIE`, no session cookie, no refresh
  cookie, no `HttpOnly` issuance and no `SameSite`/`Secure` decision to
  make — the surface does not exist. `HTTPBearer(auto_error=False)` means a
  missing header is answered by our own guard with `401 UNAUTHENTICATED`
  ("missing bearer token"), not by the framework.
- **Logout** revokes the refresh session; the access token remains valid
  until its short TTL expires. That is the accepted stateless tradeoff — do
  not claim instant revocation.
- **Password reset** revokes every session of the account but does **not**
  mark the email verified; a reset account stays unverified and cannot log
  in until the verification code is consumed.
- Refresh tokens are stored **plaintext**. A DB read grants live sessions.
  The upgrade is a keyed hash (HMAC with a server secret) on store and a
  lookup by hash; do not describe the current state as hashed.

### 4.3 Email codes

- 6 digits, zero-padded, from `secrets.randbelow(1_000_000)` — uniform over
  000000–999999.
- Verification TTL 24h (`<APP>_VERIFY_CODE_TTL_HOURS`); reset TTL 30m
  (`<APP>_RESET_TTL_MINUTES`).
- 5 wrong attempts cap a code (`<APP>_VERIFY_MAX_ATTEMPTS`, shared by the
  verification and the reset flow) and answer `429 AUTH_CODE_ATTEMPTS`.
- Reset requests are limited to 3 per 5 minutes per email
  (`<APP>_RESET_RATE_LIMIT`); the limit is **silent** — the endpoint always
  answers `202` so it discloses nothing.
- Comparison is constant-time (`hmac.compare_digest`); consumption is
  atomic — the code row is deleted in the same transaction that flips
  `is_verified` or sets the new hash.
- Codes are stored plaintext in their tables (short-lived by design). The
  janitor purges expired rows.

### 4.4 Login gates and lockout

Order matters and is deliberate: password first, then `is_blocked`,
`is_active`, `is_verified`. A wrong password never reveals account state.

An unknown email still runs a dummy argon2 verify against a fixed
`_DUMMY_HASH`, so the unknown-email branch costs the same as the
wrong-password branch and timing does not disclose which accounts exist.

**There is no lockout and no retry counter.** The `users.retry_count`
column exists in the migration with a default of 0, but no query ever
increments it and no setting caps attempts. Brute-force protection against
password guessing is therefore **absent** in this template; the only brake
is the argon2 cost and whatever the edge enforces. The proper fix is a
per-account counter with a time-based lockout (`locked_until`) and
exponential delay, which requires a schema change and is not shipped. Until
it is, the edge rate limit is the control.

### 4.5 OAuth

- Google: One Tap ID token verified against Google's JWKS over HTTPS
  (`PyJWKClient` with cached keys, fetched on a worker thread), RS256,
  audience = client id, required claims `sub`, `email`, `aud`, `exp`. Empty
  client id = unconfigured → `503 AUTH_OAUTH_FAILED`.
- GitHub: auth-code exchange against fixed GitHub endpoints; client secret
  from env; the callback URL is configured, not taken from the request.
  The exchange runs over `httpx` with a 10s read timeout and a 5s connect
  timeout.
- Both are disabled under `<APP>_OFFLINE=true` at the request boundary,
  before any socket opens (INV-T7).
- Linking rule: provider id → email → provision an auto-verified user. An
  account already linked to a different provider subject is refused with
  `409 AUTH_EMAIL_TAKEN`.
- **State handling is split.** The *link* flow mints a signed state token
  (`Tokens.mint_state`, HS256, 600s TTL) carrying the caller's subject. The
  callback asks `is_link_state(state)` — the state must verify *and* carry
  the link purpose — and only then links, which is why a link cannot be
  driven by a forged state. The *login* flow has no server-issued state at
  all: `state` on `GET /api/v1/auth/github/login` is a client-supplied
  string (≤512 chars) echoed into the authorize URL and back, and the
  callback accepts *any* state value as a login state. Classic OAuth
  login-CSRF (forcing a victim into the attacker's account) is therefore
  possible. A new provider must mint and verify its own state before it
  ships.
- The callback returns the token pair in the URL **fragment**
  (`#access_token=…`), not the query string, so it is not sent to a server
  on the next navigation.

### 4.6 CSRF

There is no CSRF token in this template, and none is needed while the
transport is bearer-only: the browser does not attach an `Authorization`
header on its own, so a cross-site form post arrives with no credential.

That is acceptable only while:

- no cookie is ever set by this API;
- no state-changing `GET` exists (the OAuth callback is the one redirect
  that mutates, and it is covered by the provider's code exchange, not by a
  session cookie);
- the CORS allowlist contains no wildcard in production.

If the token is ever moved into a cookie — a common client-side choice —
CSRF comes back and the change must ship `SameSite=Strict` (plus `Secure`
and `HttpOnly`) and a double-submit token, or a documented equivalent,
before it goes live.

---

## 5. Authorization

- Roles: `GUEST < USER < MODERATOR < ADMIN` (`app.shared.rolex`, a `StrEnum`
  with an explicit rank map). `require_role(floor)` checks rank ≥ floor.
  RBAC reads the role column, never a `user_type`.
- Route guard alone is insufficient. Every mutation re-checks:
  - **ownership** in SQL (`WHERE owner_user_id = :owner_id`) for user data;
  - **rank** via `app.shared.authz` for admin actions. `can_manage_subordinate`
    requires the actor to be at least moderator and to rank **strictly**
    above the target: an ADMIN cannot manage another ADMIN. `can_manage_self`
    (same id) exists; `can_manage` (admin manages everyone, moderator
    manages below moderator) exists but **no shipped call site uses either
    one** — only `can_manage_subordinate` is called.
- A stale token whose role no longer matches the database row still
  carries the old rank: the actor's role is read from the JWT, not re-read
  from `users`. A demoted admin keeps its power until the access token
  expires (≤15 minutes by default).
- A foreign or unowned row returns `404` (`WIDGET_NOT_FOUND` for widgets,
  `NOT_FOUND` elsewhere). Do not confirm existence.
- An authenticated but disallowed action returns `403 FORBIDDEN`.
- The admin surface writes `audit_logs` in the same use-case path as the
  change (actor, action, entity, entity id, JSON payload, IP, user agent).
  An admin change without an audit row is a defect.
- `POST /api/v1/auth/upgrade-to-admin` is public and gated only by the
  `<APP>_ALLOWED_ADMIN_EMAILS` list. It writes an audit row with an empty actor
  id, because the caller is not authenticated.
  Treat it as a bootstrap tool: remove it after the first admin exists.
- No impersonation, no "act as" header, no tenant header. If one is ever
  added, it is an audited, role-gated feature, not a header passthrough.

---

## 6. Input handling

- **Shape** is validated at the edge with pydantic v2 models
  (`StringConstraints`), producing `422 VALIDATION_FAILED` through the
  shared envelope. The bounds are explicit: email 3–254 with a pattern,
  password 8–128, code exactly 6, names ≤100, language ≤10, widget name
  ≤120, widget kind ≤32, Google credential ≤8192. An unbounded string is a
  DoS vector.
- **Body limit**: the application sets none — Starlette reads the body into
  memory as presented. The edge applies nginx's default 1 MiB cap, which
  the template does not raise. Keep it that way, or add an application-level
  limit.
- **Path params** are treated as opaque strings; the row is resolved by the
  query, not by a regex that leaks format.
- **Query params** go through `pagination(page, page_size)`
  (`app.platform.envelope`): page ≥ 1, size clamped to 1–100. The routes
  also declare `Query(ge=1)` / `Query(ge=1, le=100)`. Do not read a raw
  limit out of the query.
- **Unknown fields** are ignored by pydantic's default model parsing. The
  update paths pick fields explicitly (`UpdateMeRequest`, the `Patch`
  dataclass) — never splat a request body into an update.
- **Output encoding** is the client's job for HTML; the API returns JSON.
  Never interpolate user data into an HTML error page.
- **Email templates** are module constants rendered with
  `str.format(**variables)`; the variables are server-computed (the code
  and the TTL). An unknown template key raises; an unknown locale falls
  back to `en`. The templates ship `en` and `es` locales — that is
  user-facing content, not repository prose. A template that ever
  interpolates a display name must escape it.

### 6.1 Fetched and model output is untrusted

No model output and no third-party API response is trusted. A provider
response is parsed, typed and validated before it reaches the domain — the
Google claims and the GitHub profile become an `ExternalIdentity` dataclass
with a known shape. It never becomes SQL, a shell argument, a file path, or
a redirect target.

---

## 7. SSRF and outbound traffic

- Outbound destinations are fixed constants: Google JWKS, GitHub token/user
  endpoints, the configured SMTP host.
- No user input selects a URL, host, port or redirect target.
- `<APP>_OFFLINE=true` refuses OAuth before dialing and forbids SMTP
  credentials at boot; the console sender never opens a socket (INV-T7).
- Every outbound call carries a timeout: 10s read / 5s connect for the
  GitHub exchange, 10s for SMTP, and the JWKS fetch runs on a worker thread
  so it cannot block the event loop.
- If a webhook or image-fetch feature is added, it needs an allowlist, DNS
  rebinding protection, no redirect following, and a private-IP block.
  "We validate the scheme" is not enough.

---

## 8. Cryptography and secrets

- **JWT secret**: `<APP>_JWT_SECRET`, at least 32 characters
  (`Field(min_length=32)`). The boot-time `_fail_closed` validator in
  `app.platform.config` rejects the weak literals (`secret`, `changeme`,
  `change-me`, `password`, `development`) and any secret with fewer than 8
  distinct characters — the process refuses to start rather than run with a
  guessable key. Rotating it invalidates every access token and every
  minted OAuth state; refresh sessions are opaque and survive it, so revoke
  them explicitly if that is the intent. Rotation needs a dual-key
  verification window if zero-downtime matters — not shipped.
- **Pepper**: `<APP>_PASSWORD_PEPPER`, appended to the plaintext. Rotating
  it invalidates every password hash; treat it as a migration, not a config
  change.
- **Randomness**: the `secrets` module only (`token_hex(64)` for refresh
  tokens, `randbelow` for codes), plus `uuid4` for identifiers. Never the
  `random` module for ids, tokens or codes.
- **Comparison**: `hmac.compare_digest` for secrets (codes); argon2 for
  passwords. Never `==` on a secret.
- **Secrets live in env**, not in the repo. `.env` is gitignored;
  `.env.example` contains placeholders only. A secret in a commit is
  compromised: rotate it, do not just delete the line.
- **Database credentials** are operator-provided; the app never creates
  roles or databases. The DSN must use the `postgresql+asyncpg://` driver
  or the boot fails.
- **No custom crypto.** Use `secrets`, `hmac`, `hashlib`, PyJWT,
  argon2-cffi. A hand-rolled cipher or hash is rejected in review.

---

## 9. API and application hardening

### 9.1 Limits and resource consumption

- Body: no application limit (the edge caps at nginx's default 1 MiB); list
  page 100; outbox batch configurable (`<APP>_OUTBOX_BATCH`, default 10).
- No per-IP or per-account rate limit ships. Put one at the edge (nginx) or
  add an application limiter with a shared store; the reset flow's
  per-email limit is not a substitute.
- Password hashing is memory-hard: 64 MiB per hash with 4 lanes. A burst of
  login attempts is a memory and CPU DoS. Rate-limit before hashing, not
  after.

### 9.2 CORS, headers, cookies

- CORS is an exact-origin allowlist (`<APP>_CORS_ORIGINS`, a JSON array). The
  middleware is installed **only when the list is non-empty** — an empty
  list means no CORS headers at all. `allow_credentials` is not enabled, so
  the browser never sends credentials cross-origin. Allowed methods are the
  fixed REST set; allowed headers are `authorization`, `content-type`,
  `x-request-id`.
- Response headers: `x-content-type-options: nosniff`, `x-frame-options:
  DENY`, `referrer-policy: no-referrer`. No CSP is set (JSON API); if an
  HTML surface is added, set one at the edge. **`cache-control: no-store` is
  not set** — authenticated JSON responses carry no cache directive (the SSE
  stream sets `cache-control: no-cache` on its own).
- Cookies: none. The API never sets one, so there is no cookie to
  misconfigure and no `HttpOnly`/`SameSite`/`Secure` decision to get
  wrong.
- `x-request-id` is echoed. It is taken verbatim from the client when
  present, with no length or character check, and it lands both in the
  response header and in the log record. It must never be trusted for
  authorization.

### 9.3 Webhooks and callbacks

None shipped. The GitHub callback is a provider redirect, not a webhook; it
is guarded by the code exchange (and, for the link flow, by the signed
state). If a webhook is added: verify the signature with a constant-time
compare, bind the timestamp window, reject replays by event id, and process
idempotently.

### 9.4 Errors and exceptional conditions

- One envelope `{detail, code}`; 5xx details are generic
  ("internal server error") and the traceback goes to the log, never to the
  client.
- The stable codes are the `Code` enum in `app.shared.apperr`:
  `VALIDATION_FAILED`, `UNAUTHENTICATED`, `FORBIDDEN`, `NOT_FOUND`,
  `CONFLICT`, `RATE_LIMITED`, `INTERNAL`, `AUTH_EMAIL_TAKEN`,
  `AUTH_INVALID_CREDENTIALS`, `AUTH_ACCOUNT_BLOCKED`,
  `AUTH_ACCOUNT_INACTIVE`, `AUTH_EMAIL_UNVERIFIED`, `AUTH_INVALID_TOKEN`,
  `AUTH_CODE_INVALID`, `AUTH_CODE_EXPIRED`, `AUTH_CODE_ATTEMPTS`,
  `AUTH_OFFLINE`, `AUTH_OAUTH_FAILED`, `WIDGET_NOT_FOUND`. Match on the
  code, never on the message text. `CONFLICT` and `RATE_LIMITED` are
  defined but no shipped path raises them — the reset limiter is silent by
  design — so do not write a client that depends on them.
- Never include SQL, driver messages, stack traces, env values, tokens or
  emails in a response.
- A 404 for an unowned row is intentional; do not "fix" it to 403.
- Log the stable `code` and the request id for 5xx; keep the user-facing
  detail useless to an attacker.

### 9.5 Inventory and exposure

Shipped surface is listed in `PLAYBOOK_URLS.md` §2.1 — 35 HTTP operations
over 30 paths, plus one WebSocket endpoint. It is the inventory; reconcile
it with the route table each release. `specs/openapi.yaml` is generated
from the app and `make gen-check` fails when it goes stale.

FastAPI serves **`/docs` and `/openapi.json` by default** in this template
(`docs_url="/docs"`, `redoc_url=None`), so the interactive documentation
and the full schema are public in production unless they are turned off.
There are no `/metrics` endpoints and no debug endpoints. A new endpoint is
a security review item, not a convenience.

---

## 10. Outbox and event security

- The outbox prevents inline delivery, but it is not a trust boundary: the
  payload is JSON written by the app. Treat it as untrusted on read
  (validate the shape, cap size) in case a row is tampered with.
- The enqueue is **atomic with the business write**: the slice adapter
  stages the `email_jobs` row on the caller's session and commits the user
  row, the code row and the job row in one transaction. A crash cannot
  separate a mail from the state change that justifies it.
- Retry backoff is bounded (`2^attempt` minutes, capped) and the row
  dead-letters to `failed` at `max_retries` (column default 3); a poisoned
  row cannot retry forever.
- A worker claims with `FOR UPDATE SKIP LOCKED` and reclaims rows stuck in
  `sending` for more than 5 minutes; multiple replicas are safe.
- The lease means a crashed worker's mail can be delivered twice.
  At-least-once is the contract; templates must be safe to repeat.
- Realtime events carry only data the same identity could read over REST.
  A room is not an authorization mechanism; it is a routing key. The hub is
  in-process and bounded (queue of 32, drop-oldest on overflow), so a second
  replica needs a real broker — until then, realtime is single-process.
- The WS `?token=` is an access token. It must not be logged, cached, or
  stored client-side longer than the session.

---

## 11. Supply chain

- Dependency ranges are pinned to modern floors; `pip-audit` (`make vuln`)
  runs in CI after `make ci`, followed by actionlint.
- **The template ships no `uv.lock` (INV-T3).** `uv.lock` is gitignored and
  `make tidy` runs `uv sync`, so installs resolve newest-in-range: builds
  are not reproducible, a compromised release can be picked up, and the
  `pip-audit` output is a snapshot rather than a pin. After instantiation,
  commit the lockfile, install with `uv sync --frozen` in CI, and remove the
  ignore entry.
- No SBOM, no provenance/signature verification, no container scan ships.
  Add Syft + Trivy (or equivalent) as advisory jobs, then gating ones.
- Review dependency additions for install scripts and typosquats. A new
  package is a code review, not a one-line manifest edit.
- The container installs the project with `pip install .` as root during the
  build and runs as the unprivileged `app` user (uid 10001) at runtime. Do
  not install with elevated privileges in CI.

---

## 12. Deployment (self-hosted)

- TLS terminates in front of the edge. The shipped `infra/nginx` server
  block listens on plain port 80 and only publishes that port; the API is
  reachable through it, never directly. Put TLS in front of it (or add it
  to the server block) before production — the app itself speaks HTTP
  inside the network and must never be exposed directly.
- The image is single-stage `python:3.12-slim`, installs the project, runs
  as the non-root `app` user and health-checks `/healthz` with the standard
  library (the slim image carries no curl).
- Postgres is not exposed publicly in the production composition;
  credentials come from the host environment, never baked into the image.
- `docker-compose.prod.yml` overrides dev defaults; verify it does not carry
  dev passwords or a wildcard CORS list.
- Migrations run as a one-shot job (the `migrations` service behind the
  `tools` profile), never from the API process. A failed migration stops
  the rollout.
- Backups: encrypted, tested by restore, with a retention policy. A backup
  that has never been restored is a hypothesis.

---

## 13. Logging, monitoring and response

### 13.1 What to log

Log: request id, method, path (no query secrets), status, latency, stable
error code, actor id on authenticated actions, admin audit entries, worker
lifecycle, janitor counts.

What the app actually emits today is narrower: the worker lifecycle, outbox
delivery failures, the janitor counts and the unhandled-error trace. There
is **no per-request log line** — no method, path, status or latency — and
the `AppError` handler answers the envelope without logging, so a 401, 403
or 404 leaves no application record. The audit trail lives in the
`audit_logs` table, not in the log stream. Treat the list above as the
target, and the edge access log as the only per-request source until the
app logs one.

The formatter (`JsonFormatter` in `app.platform.logging`) emits one JSON
object per line with `ts`, `level`, `logger`, `msg`, `request_id` and the
exception text. The request id is bound to the task through a `ContextVar`
and echoed as `x-request-id`.

**Caveat in the shipped entrypoint:** `main()` (the `python -m app.bin.api`
path used by the container) does not call `configure_logging`, and it starts
the server with `log_config=None`, so no handler is installed on the root
logger and the access log stays off. Records then reach stderr through the
standard library's last-resort handler as plain text, without the JSON
shape and without the request id. `create_app()` and the worker entrypoint
do configure it. Fix this before relying on structured logs.

Never log: passwords, tokens, cookies, authorization headers, full request
bodies on auth routes, SMTP credentials, the WS query string, email bodies
(they carry the one-time code).

### 13.2 Detection

Alert on: 5xx rate, `AUTH_CODE_ATTEMPTS` spikes (code guessing), repeated
`AUTH_INVALID_CREDENTIALS` from one IP, `AUTH_ACCOUNT_BLOCKED` and
`AUTH_ACCOUNT_INACTIVE` spikes, outbox `failed` growth, readiness failures,
migration job failure. There is no `AUTH_RATE_LIMITED` code in this
template — the reset limiter is silent — so watch the reset-email volume
and the edge counters instead.

The app logs no per-request line and no 4xx, so every auth alert above is
an **edge** alert today: the counts come from the proxy access log. The
nginx `clean` format carries no request id, so an edge line cannot be
joined with an application record — add `$upstream_http_x_request_id` to
the log format when the two must be correlated.

### 13.3 Incident readiness

- One on-call owner; one channel; one severity scale.
- Break-glass: rotate `<APP>_JWT_SECRET` (invalidates access tokens and pending
  OAuth states; refresh sessions survive, so revoke them), rotate the
  pepper only with a password-reset plan, revoke sessions in SQL
  (`UPDATE sessions SET is_active = false`), rotate SMTP/OAuth secrets,
  empty `<APP>_ALLOWED_ADMIN_EMAILS`.
- Preserve logs before remediation; note the timeline.
- Post-incident: root cause, control gap, test that would have caught it.

### 13.4 Disclosure

A `security.txt` or a published contact is the minimum. Acknowledgment
within 3 business days; do not promise a bounty you do not run.

---

## 14. Secure development lifecycle

### 14.1 Review checklist for every PR touching a boundary

- [ ] Input is pydantic-validated with explicit bounds.
- [ ] SQL is a `text()` statement with bound `:name` parameters; no user
      string inside an f-string.
- [ ] Authorization checks ownership/rank at the handler, not only the route.
- [ ] A foreign row returns 404.
- [ ] No secret, token or password reaches a log or an error detail.
- [ ] New outbound call has a fixed destination and a timeout.
- [ ] New config key is prefixed `<APP>_`, validated and documented; no
      default secret.
- [ ] A regression test covers the control.
- [ ] Dependency addition reviewed; no install script.

### 14.2 Python-specific profile

| Risk                     | Rule                                                                       |
| ------------------------ | -------------------------------------------------------------------------- |
| Dynamic attribute access | never `getattr`/`setattr` on a user-supplied name; use explicit fields      |
| Mutable default argument | never `def f(x=[])`; a shared default leaks state across calls              |
| Late-bound closure       | bind loop variables explicitly (default argument or `functools.partial`)    |
| `eval`/`exec`/`compile`  | forbidden                                                                    |
| Unsafe deserialization    | never `pickle`/`marshal` untrusted bytes; it is remote code execution       |
| Command injection        | never `subprocess` with `shell=True`; pass an argv list                      |
| `assert` as control flow | stripped under `python -O`; never enforce a security rule with `assert`     |
| Swallowed exception      | `except Exception: pass` is banned; log and re-raise, or map to a `Code`    |
| Blocking the event loop  | no sync file/DB/HTTP call on a request path; `asyncio.to_thread` instead    |
| Unbounded fan-out        | no `asyncio.gather` over user-sized input                                   |
| Coercion                 | pydantic is not strict by default; a string becomes an int — use strict types |
| Naive datetime           | use the injected clock (`app.shared.clockx`) and UTC-aware values            |
| ReDoS                    | no user-supplied regex; bound any pattern over user text                     |
| Format injection         | `str.format` on a user-supplied template string is an injection sink         |
| Path traversal           | no `open()` on a request-derived path; resolve and prefix-check              |
| Timing                   | constant-time compares for secrets; do not branch on token bytes             |
| `random` module          | forbidden for security values; use `secrets`                                |
| Unhandled task exception | a task without an owner loses its exception; keep the reference and cancel it |

---

## 15. Concrete example: a user asks for another user's session

Request: `GET /api/v1/admin/auth-session/users/{user_id}/sessions` with a
USER token for someone else's id.

1. Route posture: the handler takes `Admin = Depends(require_role(
   Role.ADMIN))`, so the request is rejected with `403 FORBIDDEN` before
   the body is read. The route is not reachable with a USER token.
2. If the route posture were missing, the admin session view has **no
   second ownership or rank check of its own** — the handler goes straight
   to the query scoped by `user_id`. The only other layer is the shape of
   the response: `SessionView` carries id, IP, user agent, created and
   expiry time, never the refresh token. That is why the guard is pinned by
   an acceptance scenario ("A non-admin caller gets 403 on the admin
   session routes") rather than by a unit test of the handler.
3. If the guard were also missing, the query is scoped to the target id but
   the handler would return another user's session metadata — which is why
   the guard is a test, not a comment.
4. The response never includes the refresh token; session listings expose
   id, ip, user agent, activity and expiry only.
5. The rejection leaves no application record: the `AppError` handler
   answers the envelope without logging, so a 401/403/404 is visible only in
   the edge access log. Repeated probes of `/api/v1/admin/**` from one
   address are the signal to alert on.

The test: the acceptance feature asserts `403` for the USER token. The
sibling admin path (`PATCH /api/v1/admin/users/{user_id}`) additionally
re-checks rank in the use case (`can_manage_subordinate`), and a unit test
asserts that an admin cannot edit a peer. Both fail if a guard regresses.

---

## 16. Known deviations — fix before production

| #   | Deviation                                                     | Impact                                            | Fix                                                     |
| --- | ------------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------- |
| S1  | The WS token travels in the query string (`/api/v1/ws?token=…`) | access token in any access log or proxy log       | accept the token in a header or subprotocol; at minimum strip the query string from the logged request line |
| S2  | Refresh tokens stored plaintext                               | DB read = live sessions                           | store an HMAC of the token; look up by hash            |
| S3  | No login lockout and no retry counter                         | unlimited password guessing, account DoS by email  | per-account counter + `locked_until` + exponential delay; notify the owner |
| S4  | No global rate limit                                          | credential stuffing, memory-exhaustion DoS        | edge limit + an application limiter with a shared store |
| S5  | No lockfile in the template                                   | non-reproducible installs; supply-chain drift     | commit `uv.lock`; `uv sync --frozen` in CI             |
| S6  | Public `upgrade-to-admin`                                     | privilege escalation if the list is misconfigured | remove after bootstrap                                  |
| S7  | No CSRF token (safe only while the transport stays bearer-only) | moving the token into a cookie reopens CSRF      | if a cookie is ever introduced, add `SameSite=Strict` + a double-submit token in the same change |
| S8  | `/docs` and `/openapi.json` are served by default             | endpoint inventory and schema disclosure          | disable both in production                              |
| S9  | No `cache-control: no-store` on responses                     | an intermediary may store a private response      | add the header to authenticated routes                  |
| S10 | The shipped API entrypoint installs no log formatter          | plain-text stderr logs, no request id, no access log | call `configure_logging` in `main()` and keep `log_config=None` |
| S11 | No application-level body size limit                          | memory DoS on a large body                       | keep the edge cap explicit, or add a middleware limit   |
| S12 | `x-request-id` is echoed verbatim and unbounded               | attacker-controlled value in logs and headers     | bound the length, sanitize the characters               |
| S13 | `users.role` has no `CHECK` constraint                        | an invalid role can be written outside the domain | add the constraint in a migration                       |
| S14 | Refresh rotation extends the absolute expiry                  | a rotating client keeps a session alive forever   | pin the absolute expiry at creation, add a rotation window |
| S15 | The admin session routes are rank-free and write no audit row | an admin reads another admin's session metadata  | add `can_manage_subordinate` and an `audit_logs` row      |
| S16 | The GitHub **login** flow has no server-issued `state`       | OAuth login CSRF: a victim can be bound to the attacker's account | mint and verify a signed state on the login flow too |
| S17 | The console sender logs the whole email body at INFO         | the one-time verification/reset code lands in the log | log the recipient and the template key only, never the body |

---

## 17. Checklist

- [ ] Passwords hashed with argon2id; pepper set before the first signup.
- [ ] JWT pinned to HS256, issuer and expiry checked, required claims
      present.
- [ ] Refresh rotation is a compare-and-swap; replay of the old token
      fails.
- [ ] Bearer only: no cookie is set anywhere in the app.
- [ ] Codes constant-time compared, attempt-capped, TTL-bound.
- [ ] Login gates in order, and a brute-force control exists (lockout at
      the app or a rate limit at the edge).
- [ ] Every route has an explicit posture; admin routes rank-gated.
- [ ] Ownership is enforced in SQL; foreign rows return 404.
- [ ] Admin mutations write `audit_logs`.
- [ ] Input pydantic-validated with bounds; the body stays size-capped.
- [ ] No secret in a log, error, or the repo.
- [ ] Outbound destinations fixed; offline guard honored.
- [ ] Errors use the envelope and leak nothing.
- [ ] New dependency reviewed; lockfile policy decided.
- [ ] Threat row named in the PR; regression test added.
