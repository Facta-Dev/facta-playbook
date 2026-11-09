# PLAYBOOK_SECURITY.md — security contract (Rust backend)

Security rules for `<app>`. They harden the architecture in
`PLAYBOOK_BACKEND.md`, implement the authorization model in `PLAYBOOK_URLS.md`,
are verified by `PLAYBOOK_TESTS.md`, and produce the evidence
`PLAYBOOK_AUDIT.md` asks for.

The rules that carry the whole file:

1. **Deny by default, fail closed.** A route, role, outbound destination or
   carrier exists because it was granted explicitly.
2. **Authorization lives in the handler and the use case.** The URL prefix is
   an experience namespace; it never authorizes. The verified JWT is the only
   source of identity.
3. **Ownership is the primary boundary.** There are no tenants. The
   worst-case incident is one user reading or mutating another user's data.
4. **All input is hostile**: JSON bodies, path and query parameters, OAuth
   provider responses, outbox payloads, headers.
5. **Secrets never live in code, images, logs or job payloads.**
6. **Assume breach.** Bound the blast radius, log security events, and keep
   `<APP>_OFFLINE=true` a complete, working configuration.

> Scope note: a defensive engineering baseline. It lowers risk; it does not
> guarantee the system cannot be breached. Numeric values are starting points.

---

## 1. Standards and mapping

| Standard | Use |
|---|---|
| **OWASP Top 10:2025** | web risk taxonomy |
| **OWASP API Security Top 10 (2023)** | BOLA, broken auth, BOPLA, BFLA, consumption, SSRF |
| **OWASP ASVS 5.x** | Level 2 baseline; Level 3 for auth, sessions, crypto, admin |
| **OWASP Cheat Sheet Series** | implementation detail |
| **CWE Top 25** | weakness classes the lints target |
| **RFC 8725 (JWT BCP), RFC 9700 (OAuth 2.0 Security BCP)** | token handling |
| **NIST SSDF (SP 800-218), SLSA** | secure development and build provenance |

| OWASP Top 10:2025 | Handled in |
|---|---|
| A01 Broken Access Control | §5, §7 |
| A02 Security Misconfiguration | §9, §12 |
| A03 Software Supply Chain Failures | §11 |
| A04 Cryptographic Failures | §8 |
| A05 Injection | §6 |
| A06 Insecure Design | §2, §14 |
| A07 Authentication Failures | §4 |
| A08 Software or Data Integrity Failures | §10, §11 |
| A09 Security Logging and Alerting Failures | §13 |
| A10 Mishandling of Exceptional Conditions | §9.4 |

---

## 2. Threat model

### 2.1 Assets and actors

| Assets (by priority) | Actors |
|---|---|
| JWT signing secret, password pepper, API-key HMAC secret, GitHub client secret, SMTP password; refresh tokens and session rows; password hashes; audit log; source and CI secrets | external attacker, credential stuffer, malicious authenticated user, malicious admin, compromised dependency or CI action, insider |

### 2.2 `<app>`-specific threats

| Threat | Why it matters here |
|---|---|
| Cross-user read/write (IDOR/BOLA) on widgets, sessions or profile | there is no tenant gate; only the handler/use-case predicate |
| Privilege confusion: calling `/api/v1/admin/...` with a Viewer token | the prefix is a namespace; the rank gate is the control |
| OAuth takeover: forged `state`, replayed code, unverified email trusted as identity | OAuth is a login path and a credential |
| Replayed refresh token after rotation | CAS stops the replay, but the family is not revoked (§17 G2) |
| Credential stuffing / account enumeration | login lockout exists; response uniformity must hold |
| Token leak through logs: `?token=` on the WebSocket route | the tracing span logs the full URI (§17 G7) |
| Cookie-mode CSRF in split deployments | `SameSite=Strict` is set, but no `Origin`/CORS check is wired (§17 G3) |
| Resource exhaustion: unbounded bodies, expensive queries, email floods | no global body cap or rate limiter (§17 G4/G6) |
| Supply-chain compromise (crates, CI actions) | the binary holds every secret |
| `panic = "abort"` in release: a handler panic kills the process | availability risk, not just a 500 (§17 G12) |

### 2.3 Process

- Every new slice, externally reachable route, outbound integration or data
  category needs a threat model at `docs/threat-models/{slice}.md`, reviewed in
  the PR: data-flow, trust boundaries, threats, controls, residual risk, and
  the tests that prove the controls.
- Revisit on major change and annually; findings feed the risk register.
- Each slice has a named security owner.

---

## 3. Layers of defense

```text
Internet
  → Edge (optional reverse proxy / load balancer): TLS 1.3 preferred,
    body and rate limits, security headers, routing allow-list
    → axum middleware: request_id → TraceLayer → auth middleware
        → handler: extractor validation, can(role, action) / rank gate,
          ApiError envelope
          → slice service: business rules, ownership, idempotency
            → ports adapter: runtime sqlx, parameterized, least-privilege role
Async: loom outbox worker claims email_jobs (FOR UPDATE SKIP LOCKED),
  payload carries ids only, rendered inside the worker; the hub is fan-out
  for connections authenticated at upgrade
Outbound: OAuth clients (fixed endpoints), SMTP (configured host), provider
  mailer (configured endpoint) — all refuse under <APP>_OFFLINE=true
Platform: Postgres, tracing JSON logs, backups
```

---

## 4. Authentication

### 4.1 Passwords

Implemented in `<app>-warden/src/password.rs`:

- **argon2id** with `Params::DEFAULT` (m = 19 MiB, t = 2, p = 1 — the OWASP
  default), over `base64(HMAC-SHA256(password, pepper))`.
- The pepper is applied twice: as the HMAC key for the pre-hash and as the
  Argon2 secret key (`Argon2::new_with_secret`). An empty pepper feeds the raw
  password and uses plain argon2id — that is dev mode, not production.
- The PHC string carries algorithm and params, so `verify_password` adapts to
  what `hash_password` wrote.
- `<APP>_PASSWORD_PEPPER` must be set (≥ 32 bytes) in every non-dev
  environment; `Config::validate` rejects a short or placeholder value.
- Verification is constant-time inside argon2; `constant_time_eq` is used for
  token and code comparisons.
- No composition rules. Accept long passphrases, reject trivial passwords,
  never force rotation without a compromise signal.
- Login and reset responses are uniform for "unknown user" and "wrong
  password"; the login path runs the hash comparison even when no user is
  found only if the implementation does — verify that in review, because a
  missing dummy verify is a timing oracle. Current code returns early on
  unknown email (`auth/src/session.rs`), so **uniform timing is a known gap**.

### 4.2 Access tokens (JWT)

Implemented in `<app>-warden/src/lib.rs`:

| Check | Rule |
|---|---|
| Algorithm | `alg` must be `HS256`; anything else rejected before the key lookup |
| Key selection | `kid` is **required**; an unknown `kid` is rejected |
| Issuer / audience | `iss = <app>`, `aud = <app>-api`, both pinned |
| Lifetime | `exp ≥ iat`, `exp − iat ≤ MAX_ACCESS_TTL_SECS` (900 s), enforced on **verify** as well as mint |
| Clock | strict; future `iat` rejected; no leeway (`exp`/`nbf` leeway 0) |
| Subject | `sub` is a UUID |
| Roles | `roles: Vec<Role>` snapshot, at most one access-TTL stale |
| `jti` | required, ≤ 64 chars, consulted against the revocation store |

- The token carries a roles snapshot so the middleware needs no DB read. That
  is a deliberate trade: a role change takes effect within one access TTL.
- Mint rejects TTLs over the ceiling; verify re-asserts it, because the mint
  check is bypassable by anyone holding the signing key.
- **Known gap G8:** the composition root builds the codec with a single key
  (`kid = "default"`). Rotation is not wired; `JwtCodec` supports a key map,
  so rotation is a config change plus a deploy, but it is untested.
- **Known gap G11:** no MFA. When added, require it for Admin and Owner before
  exposing provider keys or audit surfaces.

### 4.3 Refresh tokens and sessions

| Item | Implementation |
|---|---|
| Format | 48 CSPRNG bytes → 64-char base64url (`new_token`) |
| Storage | `sessions.refresh_token VARCHAR(128) UNIQUE`, **plaintext** (G1) |
| Rotation | on every refresh, in place, via compare-and-swap on the old token |
| Replay | CAS loses → `401 invalid refresh token`; identity and rotated token survive (G2) |
| Account gates | refresh re-checks user state; blocked/deactivated users cannot ride an old session (the refused refresh deactivates the session) |
| Logout | deactivates the row for that user; idempotent |
| Revoke-all | deactivates every session for the user; password reset can sweep all sessions (`log_out_all_on_reset`) |
| Access revocation | `jti == sessions.id`; revoking the session revokes the access token by `jti` |

Rules:

- **Rotation is a security control.** A refresh that presents an already
  rotated token is a replay signal. The target behavior is to revoke the whole
  session family and log it; the shipped code only rejects the replay. Do not
  describe the current behavior as reuse detection.
- Never accept a refresh token from a URL. The WebSocket route takes the
  short-lived **access** token only.
- Never store a second plaintext token type. When the refresh hash migration
  lands, it replaces this one.

### 4.4 Verification and reset codes

- 6 digits, uniformly sampled by rejection sampling (`new_code`), never
  `rand % 10` on a biased byte.
- Single use, enforced by a CAS update on the token row.
- `<APP>_VERIFY_CODE_TTL` default 24h; `<APP>_RESET_CODE_TTL` default 30m.
- Resend and reset are rate-limited per user/email (`<APP>_RESET_RATE_LIMIT`
  default 3 per `<APP>_RESET_RATE_WINDOW`, default 15m).
- Codes are compared in constant time.

### 4.5 OAuth (Google, GitHub)

- **Google** is One Tap/ID-token verification against `GOOGLE_CLIENT_ID`; the
  ID token's signature, issuer and audience are validated before the email is
  trusted. A client secret is not needed and is not read.
- **GitHub** is the authorization-code flow with `state` round-trip and code
  exchange; `GITHUB_CALLBACK` is pinned, `GITHUB_ENABLED` defaults to false,
  and both routes answer 403 while disabled.
- Linking (`/auth/github/connect-url`) requires an authenticated session;
  login does not.
- `upgrade-to-admin` is a **bootstrap path**: it only promotes an email on
  `<APP>_ALLOWED_ADMIN_EMAILS`, is unauthenticated by design, and must be
  disabled (empty list) once a real admin exists.
- Under `<APP>_OFFLINE=true` the clients refuse before any socket is opened.

### 4.6 Cookies and CSRF

- `set_auth_cookie` emits `Path=/; HttpOnly; SameSite=Strict`, plus `Secure`
  when the environment is not development. The name is
  `<APP>_AUTH_COOKIE_NAME` (default `<app>_access_token`).
- Cookie mode is enabled by `<APP>_AUTH_COOKIE=true`; bearer is always
  accepted.
- Bearer requests are not CSRF-able. Cookie-authenticated mutations rely on
  `SameSite=Strict` **and** an exact-origin check — and the second half is not
  wired (G3). Until it is, do not enable cookie mode in a split-origin
  deployment.
- New cookie-authenticated mutations require a double-submit token. Do not
  loosen `SameSite`.

---

## 5. Authorization

Implements `PLAYBOOK_URLS.md`; verified by the role-matrix and IDOR suites.

1. **The backend is the authority.** Auth middleware verifies the JWT and
   attaches `Principal { id, roles }`. Every mutation additionally calls
   `require_action(&principal, Action::…)` or the admin rank gate.
2. **Deny by default.** A protected route without a middleware is a bug; a
   sensitive route whose handler does not check the action is also a bug.
3. **Ownership in the predicate.** A load-by-id for user data carries
   `user_id = $caller` in the SQL, not an `if` after the fetch. The caller id
   comes from `Principal`, never from the body or a query parameter.
4. **404 over 403 when existence would leak.** Another user's row is "not
   found" to the caller.
5. **BOPLA.** Explicit request DTOs (`CreateWidgetRequest`,
   `UpdateWidgetRequest` with `deny_unknown_fields`); no mass assignment of
   `role`, `is_active`, `user_id`, provider ids. Explicit response DTOs; never
   serialize a password hash, refresh token or internal flag.
6. **BFLA.** Admin functions require the rank gate even when the route looks
   harmless. The gate is strict `>`: an actor cannot modify a peer or higher
   rank. Viewer/Editor/Admin/Owner are distinct; do not conflate them.
7. **Policy is server-side only.** A client hiding a button is cosmetic.
8. **Workers authorize too.** An outbox job carries ids; the worker resolves
   and acts as a scoped system actor, never trusting an unvalidated payload.
9. **UUIDs are defense in depth, not authorization.**
10. **No row-level security.** One application role against one schema;
    isolation is application-enforced. Do not describe the system as
    RLS-isolated.

Required shape:

```rust
pub async fn update_widget(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWidgetRequest>,
) -> Result<Json<Widget>, ApiError> {
    require_action(&principal, Action::Update)?;      // role
    let widget = state.widgets.update(WidgetId(id), req).await?; // owner in the port
    Ok(Json(widget))
}
```

---

## 6. Input handling

- **Deserialize with explicit DTOs.** `#[serde(deny_unknown_fields)]` on every
  request struct; unknown fields are a `400`, not silently ignored.
- **Validate at the boundary and again in the domain.** The handler checks
  wire shape (length, enum, UUID); the service checks business invariants
  (blank name, allowed transition).
- **Canonicalize before validating** (trim, lowercase email) and store the
  canonical form.
- **Parameterize everything.** Runtime `sqlx::query*` with `$1` bindings;
  never format a string into SQL, identifiers included. Dynamic sort/filter
  columns come from a fixed allow-list.
- **No shell.** If a slice must execute a process, use an argument vector, an
  absolute binary path, a minimal environment, a timeout and resource limits.
  Prefer a library.
- **Regex**: `regex` crate (linear time); cap input length before matching.
- **Output sinks**: JSON only; no HTML rendering in the backend. CSV exports
  neutralize formula prefixes (`=`, `+`, `-`, `@`).
- **Path handling**: generated storage names; resolve and verify containment;
  never join user input into a path.

### 6.1 Fetched and model output is untrusted

Provider responses, OAuth profiles and fetched documents are **data, never
code**: no templating, no evaluation, no use as filenames, SQL fragments or
URLs without validation. If a model processes untrusted content, separate
instructions from data, restrict tool access, validate against a schema, and
never let model output decide authorization.

---

## 7. SSRF and outbound traffic

Outbound sockets exist in exactly three places: OAuth clients (fixed Google
and GitHub endpoints), the SMTP sender (configured host), and the provider
mailer (configured `MAILER_ENDPOINT`). All refuse under `OFFLINE=true`.

The moment a slice fetches a **user- or operator-supplied URL**, it must use
one guarded client providing:

- scheme and host allow-list, credentials-in-URL rejection, redirects
  re-validated per hop;
- DNS resolved once and the connection **pinned** to the validated address
  (defeats rebinding); loopback/private/link-local/multicast/metadata ranges
  denied by default;
- request budgets, bounded response reads, capped retries;
- blocked fetches counted and surfaced.

`reqwest::get(user_url)` is a finding. Provider adapters use configured base
URLs, never a URL from a request. Map upstream errors to `core::Error`; never
reflect the raw provider body.

---

## 8. Cryptography and secrets

- **TLS everywhere** in any multi-host deployment: 1.2 minimum, 1.3 preferred,
  HSTS at the edge, certificate expiry monitoring.
- **No custom crypto.** Use the vetted crates already pinned (`argon2`,
  `jsonwebtoken`, `hmac`, `sha2`, `subtle`, `getrandom`).
- **CSPRNG** (`getrandom`) for every token, salt, nonce and code. Never
  `rand`-with-a-seeded-RNG for a secret.
- **Constant-time comparison** (`subtle`) for tokens and codes.
- **Secrets** (`JWT_SECRET`, `PASSWORD_PEPPER`, `APIKEY_HMAC_SECRET`,
  `GITHUB_CLIENT_SECRET`, `SMTP_PASS`) live in the environment or a secret
  manager. Startup fails on missing, short (< 32 bytes) or placeholder values;
  keep the blocklist.
- **`Config` debug is redacted** for every secret; a `{:?}` of the config is
  safe to log.
- **Known gap G14:** secrets are not zeroized on drop (`secrecy`/`zeroize` are
  not dependencies). Processes with strict memory hygiene should isolate the
  codec behind a process boundary.
- **Rotation**: document the procedure per secret. JWT rotation needs the key
  map wired (G8); rotating `JWT_SECRET` today invalidates all sessions — that
  is acceptable only when announced. OAuth secrets rotate from the provider
  console without a code change.
- **Retention**: the janitor deletes expired sessions and stale codes;
  retention jobs cascade; no personal data in analytics.

---

## 9. API and application hardening

### 9.1 Limits and resource consumption

- The auth flows have their own gates: login lockout after
  `<APP>_MAX_LOGIN_RETRIES` (default 3) failures, and the reset rate limit.
- **Known gap G4:** there is no `DefaultBodyLimit` layer. axum's default body
  limit applies to `Bytes`/`Json` extractors (2 MiB), but do not rely on an
  implicit default for a contract — set it explicitly per router.
- **Known gap G6:** no general API rate limiter. New expensive endpoints carry
  their own budget and the gap goes in the threat model.
- Long work returns a resource and is executed by a worker; no handler blocks
  on a slow task.
- **G12:** release builds use `panic = "abort"`. A panic in a handler aborts
  the process, not just the request. Never `unwrap()`/`expect()` on
  request-derived data; if a panic is reachable, that is a P1.

### 9.2 CORS, CSRF and headers

- `cors_origins` is parsed from `<APP>_CORS_ORIGINS`, but **no CORS layer is
  mounted** (G3). Wire `tower_http::cors::CorsLayer` with an exact-origin
  allow-list before exposing a browser surface across origins.
- **Known gap G5:** no security-headers layer (`X-Content-Type-Options:
  nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`,
  `Cache-Control: no-store` on identity responses). Add a
  `SetResponseHeaderLayer` stack at the edge or on the router.
- In dev, a SPA proxying `/api` avoids preflight; the allow-list matters for
  split deployments.

### 9.3 Webhooks and callbacks

- OAuth callbacks verify `state` before parsing the body; a signed webhook
  verifies the raw-body HMAC with a constant-time compare, rejects replayed
  timestamps and is idempotent.
- An unverified webhook is an unauthenticated write: deny by default.

### 9.4 Errors and exceptional conditions

- `core::Error` → `ApiError` → `{"error": {code, title, detail, request_id,
  extra}}`. 5xx detail is replaced with `"An internal error occurred."`; the
  real error goes to the log with the request id.
- Never leak SQL, provider bodies, internal paths or backtraces.
- `404` does not disclose whether an id exists and was denied.
- A handler must not panic (G12). Use `Result` end to end; reserve
  `expect`/`unwrap` for values the type system or a constructor already
  guarantees.

### 9.5 Inventory and exposure

- `/healthz` (liveness) and `/readyz` (DB check) are outside auth and expose
  no configuration.
- The router table is the inventory: `router::build` lists every route and its
  middleware in one place. A route mounted elsewhere is a review finding.
- **Known gap G10:** `/metrics` and `/docs` are unguarded. Restrict both to an
  internal listener or the admin rank check, and feature-gate Swagger UI out of
  production builds.

---

## 10. Batch, outbox and event security

- **Jobs are untrusted input.** Validate the payload, resolve the owner from a
  trusted column, re-authorize as a scoped system actor. "Internal" is not a
  trust boundary.
- **Idempotency is structural**: terminal-safe transitions, unique
  constraints, `FOR UPDATE SKIP LOCKED`, a lease so a killed worker's job is
  recovered.
- **No secrets or personal data in the payload**: ids only; the email body is
  rendered inside the worker, not stored.
- **Abuse controls**: per-user enqueue quotas, a maximum payload size, a
  bounded batch, backpressure. One user cannot starve the belt.
- **The hub** fans out already-authorized data to a connection authenticated
  at upgrade; it never widens visibility. Slow subscribers are dropped
  (`try_send`), not allowed to stall the publisher.
- DLQ inspection and replay are admin-only and audited.

---

## 11. Supply chain

- **`Cargo.lock` is committed** once the project is instantiated (the
  template cannot ship one — placeholders break Cargo). A lockfile change is
  reviewed like code; the build uses `--locked` once the lockfile exists.
- **`deny.toml`**: yanked crates denied, wildcard dependencies denied, license
  allow-list, crates.io only.
- **`cargo audit`** for advisories, **`cargo machete`** for unused
  dependencies — both in `just audit` and the CI `audit` job.
- **Cooldown**: do not adopt a crate version published in the last 7–14 days
  unless it is a hand-reviewed security fix.
- **Minimize dependencies**; justify each new one (maintainers, activity,
  license, transitive count). Prefer std.
- **CI hardening**: pin third-party actions by commit SHA, minimum token
  permissions (`contents: read`), no secrets to fork PRs, protected branches,
  required reviews.
- **Release integrity**: build from a clean checkout, publish checksums, and
  produce an SBOM (G16).

---

## 12. Deployment

- The `Dockerfile` runs the binaries on distroless `nonroot` (UID 65532), no
  shell, no package manager.
- docker-compose is the dev deployment; the literal `JWT_SECRET` in
  `docker-compose.yml` is a **dev-only** placeholder — production injects
  secrets from a manager and never reuses it.
- Postgres is on the private network; no published database port in
  production; `.env` is never committed.
- **Known gap G15:** compose uses one Postgres role for DDL and DML. Target:
  separate migration and application roles, the app never superuser,
  `sslmode=verify-full` when the database is remote.
- Backups are encrypted, restore-tested and covered by the retention policy.
- A feature flag cannot disable a security control.

---

## 13. Logging, monitoring and response

### 13.1 What to log

`tracing` JSON with `request_id`, actor (`user_id` or system), action,
resource, outcome, reason:

- authentication success and failure, refresh replay, logout;
- every authorization denial with the required role and route;
- privileged actions: role changes, suspension, config changes, exports;
- validation floods, webhook signature failures;
- outbound provider failures and budget exhaustion.

Never log tokens, cookies, passwords, provider keys, full request bodies or
unnecessary personal data. **G7:** the `TraceLayer` span records `uri`, which
includes `/api/v1/ws?token=…`. Redact the query for that route before shipping;
a token in logs is a credential leak.

### 13.2 Detection

Alerts for brute force and credential stuffing, bursts of 403/404 across ids,
new admin grants, key rotation outside a change window, mass export, outbox
growth, unusual egress, provider spend spikes, repeated webhook failures.

`/readyz` reflects the database; `/healthz` is liveness only.

### 13.3 Incident readiness

Runbooks for: compromised session, leaked OAuth secret, cross-user exposure,
dependency/CI compromise, database loss. Prebuilt switches: revoke-all
sessions, rotate provider secrets, `<APP>_OFFLINE=true`, route block at the
edge, pause the outbox, read-only mode. Preserve evidence before destroying
anything; follow legal notification clocks.

### 13.4 Disclosure

`security.txt`, a monitored security mailbox, triage SLAs and safe-harbor
language.

---

## 14. Secure development lifecycle

| Stage | Required |
|---|---|
| Design | threat model, ASVS level, architecture review for new slices and integrations |
| Code | this playbook, linters, review with the checklist below |
| Build | `clippy -D warnings`, `cargo deny`, `cargo audit`, `cargo machete`, `gen-check` |
| Test | role matrix, IDOR, SSRF cases, injection, parser fuzzing, DAST on staging |
| Release | change record, checksums, rollback plan, evidence bundle |
| Operate | monitoring, vulnerability SLAs, patching, key rotation, access reviews, backups |
| Retire | revoke credentials, delete data, remove DNS/routes, remove dependencies |

### 14.1 Review checklist for every PR touching a boundary

- [ ] New route has a declared middleware and a role-matrix row.
- [ ] The caller id comes from `Principal` and is in the SQL predicate.
- [ ] Request DTO is explicit and `deny_unknown_fields`.
- [ ] No string-built SQL, shell call, unsafe deserialization or path join.
- [ ] Outbound URL goes through a guarded client or a fixed configured host.
- [ ] Errors map to `ApiError` and leak nothing; no reachable panic.
- [ ] No secret, token or personal data in logs, payloads or responses.
- [ ] New dependency justified, locked, scanned and past cooldown.
- [ ] A security event is emitted for the privileged action.
- [ ] Tests cover the abuse case, not only the happy path.
- [ ] Threat model updated when the trust boundary moved.

---

## 15. Rust profile

| Control | Rust implementation |
|---|---|
| Schema validation | serde DTOs + `deny_unknown_fields`; domain re-validation |
| Queries | runtime `sqlx` with `$n` bindings |
| Password hashing | argon2id + HMAC pepper (`warden::password`) |
| CSPRNG | `getrandom` |
| Constant-time compare | `subtle::ConstantTimeEq` |
| Access token | `jsonwebtoken` 9, HS256, `kid`, `aud`, `iss`, TTL ceiling |
| Memory safety | `unsafe_code = "forbid"` workspace-wide |
| SAST | `clippy` (all, pedantic, nursery) `-D warnings` |
| SCA | `cargo audit`, `cargo deny`, `cargo machete` |
| Panic policy | no unwrap on request data; release `panic = "abort"` |
| Supply chain | committed `Cargo.lock`, `deny.toml`, pinned CI actions |
| Runtime image | distroless nonroot, stripped binary |

Conventions:

- `thiserror` in libraries, `anyhow` only at a binary's top level; never leak
  a foreign error past an adapter.
- `#[expect(lint, reason = "…")]` over `#[allow]`; a bare suppression is a
  review finding.
- No `println!` in a library; `tracing` only.

---

## 16. Concrete example: a user asks for another user's session

Feature: `GET /api/v1/auth/sessions/current` and the admin twin
`GET /api/v1/admin/auth-session/users/{id}/sessions`.

```text
crates/<app>-auth/src/session.rs              # use cases, owner passed down
crates/<app>-api/src/routes/session.rs        # handlers + guards
crates/<app>-api/src/auth.rs                  # require_admin rank gate
crates/<app>-ports/src/adapters/session_pg.rs # owner in the WHERE clause
crates/<app>-api/src/router.rs                # mounted behind require_auth_cookie
```

Invariants the tests pin:

- the user route resolves the caller from `Principal`; a foreign or unknown
  session id answers `404`, never `403` with existence confirmation;
- the admin route carries the rank gate and is a separate registration, not a
  handler switching on the prefix;
- the owner predicate is in the SQL, so a missing owner cannot widen the
  query;
- the admin list is deliberately not ownership-filtered; the user list is;
- revoking a session is scoped (current vs. all) and counted.

The anti-pattern this replaces: fetch by id, then
`if row.user_id != principal.id { 403 }`. That leaks existence and leaves list
and worker paths unprotected.

---

## 17. Known gaps (state them; do not cite them as controls)

| # | Gap | Impact | Fix |
|---|---|---|---|
| G1 | refresh tokens stored plaintext | a DB dump is replayable | store `sha256(token)`, look up by hash, migrate the column |
| G2 | refresh replay rejected but family not revoked | attacker who rotated first keeps the session | on CAS loss, revoke all sessions for the user and emit a security event |
| G3 | `cors_origins` configured but no `CorsLayer` mounted | cookie mode unsafe across origins | wire `tower_http::cors` with an exact-origin allow-list |
| G4 | no explicit body-size limit | memory/CPU exhaustion on large bodies | `DefaultBodyLimit` per router (e.g. 1 MiB) |
| G5 | no security headers | clickjacking/MIME sniffing at the app layer | response-header layer or edge config |
| G6 | no global rate limiter | cheap authenticated routes can drive costly work | add a limiter; per-flow budgets meanwhile |
| G7 | WS `?token=` appears in the tracing span's `uri` | token leak to logs | redact the query for `/api/v1/ws` |
| G8 | single JWT key, `kid = "default"` | no rotation without mass logout | wire the key map + rotation procedure |
| G9 | revocation store in memory | lost on restart, not shared across replicas | persistent `SessionStore` (config warns in production) |
| G10 | `/metrics`, `/docs` unguarded | surface and traffic-shape leak | internal listener or admin gate; feature-gate Swagger |
| G11 | no MFA | admin accounts rely on one factor | add TOTP/WebAuthn, mandatory for Admin/Owner |
| G12 | `panic = "abort"` in release | a panic kills the process | no reachable panics; fuzz request parsers |
| G13 | dev compose secrets are literals | never valid in production | inject from a manager; rotate |
| G14 | secrets not zeroized on drop | memory hygiene | `secrecy`/`zeroize` if required |
| G15 | one Postgres role for DDL and DML | blast radius | separate roles; least privilege |
| G16 | no SBOM or image scan | supply-chain evidence gap | add both to CI |
| G17 | uniform login timing not proven | account enumeration oracle | dummy hash verify on unknown email + a constant-time test |

Each gap has an owner, a ticket and an expiry in the risk register
(`PLAYBOOK_AUDIT.md`).

---

## 18. Checklist

- [ ] Every route is mounted with an explicit middleware; the router table is
      reviewed as the inventory.
- [ ] Every mutation calls `can(role, action)` or the rank gate; the URL prefix
      never decides.
- [ ] The caller id comes from `Principal` and rides the SQL predicate.
- [ ] A foreign row answers `404`; the envelope leaks no existence.
- [ ] Admin and user twins share the wire shape and differ only in guard and
      owner scope.
- [ ] Request DTOs are explicit; privileged fields cannot be mass-assigned and
      secrets cannot be serialized.
- [ ] Workers and outbox jobs carry ids, re-authorize, and never trust an
      unvalidated payload.
- [ ] All input is validated at the boundary; every query is parameterized.
- [ ] Outbound fetches use a fixed host or the guarded client.
- [ ] Secrets come from a manager, are redacted in `Debug`, and never reach
      logs or job payloads.
- [ ] No reachable `unwrap`/`expect` on request data; no panic killed by
      `abort`.
- [ ] Security events (denial, privileged mutation, auth failure) are emitted
      and retained.
- [ ] `<APP>_OFFLINE=true` remains a complete configuration.
- [ ] Dependencies are locked and scanned; SLAs from the vulnerability policy
      are met.
- [ ] The threat model, role matrix and gap register are updated when the
      trust boundary moves.
