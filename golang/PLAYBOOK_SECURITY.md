# PLAYBOOK_SECURITY.md — Security contract (Go backend)

Security rules for the `<app>` Go backend built from this kit. It hardens the
architecture in `PLAYBOOK_BACKEND.md`, implements the authorization model in
`PLAYBOOK_URLS.md`, is verified by the suites in `PLAYBOOK_TESTS.md` and
produces the evidence `PLAYBOOK_AUDIT.md` asks for.

The rules that carry the whole file:

1. **Deny by default, fail closed.** A route, role, network path or outbound
   destination exists only because it was granted explicitly.
2. **Authorization lives in the use case.** The URL prefix is an experience
   namespace; it never authorizes. The session token and `authx.Identity`
   are the only sources of identity.
3. **Ownership is the primary boundary.** `<app>` has no tenants. The
   worst-case incident is one user reading or mutating another user's data
   through a route that looks harmless.
4. **All input is hostile**: HTTP bodies, query strings, path parameters,
   outbox payloads, OAuth provider responses, and anything fetched from the
   open internet.
5. **Secrets never live in code, images, logs or job payloads.** Provider
   credentials are write-only and encrypted at rest.
6. **Assume breach.** Limit blast radius, log security events, and keep
   `<APP>_OFFLINE=true` a complete, working configuration.

> Scope note: a defensive engineering baseline. It lowers risk; it does not
> guarantee a system cannot be breached. Numeric values are starting points
> to tune with the threat model.

## 1. Standards and mapping

| Standard | Use |
|---|---|
| **OWASP Top 10:2025** | Web risk taxonomy |
| **OWASP API Security Top 10 (2023)** | BOLA, broken auth, BOPLA, BFLA, consumption, SSRF, misconfiguration, inventory |
| **OWASP ASVS 5.x** | Verification requirements. **Level 2** baseline, **Level 3** for authentication, sessions, cryptography and the admin surface |
| **OWASP Cheat Sheet Series** | Implementation detail per topic |
| **CWE Top 25** | Weakness classes the SAST rules target |
| **RFC 8725 (JWT BCP), RFC 9700 (OAuth 2.0 Security BCP)** | Token and OAuth handling |
| **NIST SSDF (SP 800-218), SLSA** | Secure development and build provenance |

| OWASP Top 10:2025 | Handled in |
|---|---|
| A01 Broken Access Control (incl. SSRF) | §5, §7 |
| A02 Security Misconfiguration | §9, §12 |
| A03 Software Supply Chain Failures | §11 |
| A04 Cryptographic Failures | §8 |
| A05 Injection | §6 |
| A06 Insecure Design | §2, §14 |
| A07 Authentication Failures | §4 |
| A08 Software or Data Integrity Failures | §10, §11 |
| A09 Security Logging and Alerting Failures | §13 |
| A10 Mishandling of Exceptional Conditions | §9.4 |

## 2. Threat model

### 2.1 Assets and actors

| Assets (by priority) | Actors |
|---|---|
| Session tokens and refresh rows, password hashes, OAuth client secrets, SMTP credentials, JWT signing secret and pepper, audit log, source code and CI secrets | External attacker, credential stuffer, malicious or compromised authenticated user, malicious admin, compromised dependency or CI action, insider |

### 2.2 `<app>`-specific threats

| Threat | Why it matters here |
|---|---|
| Cross-user read/write (IDOR/BOLA): a valid `USER` token reading another user's widgets, sessions or profile | No tenant gate exists to save you; only the use case predicate does |
| Privilege confusion: calling an `/api/v1/admin/...` route with a `USER` token, or relying on a client route guard | The prefix is a namespace; the backend guard is the control |
| OAuth account takeover: forged `state`, replayed code, unverified email from the provider accepted as identity | `authoauth` is a login path and a credential |
| Session replay after logout or suspension | Refresh tokens are opaque and rotating; the row is the control |
| Credential stuffing and account enumeration | Login and reset responses must be uniform |
| Stored injection from user content (SQL, log injection, template) | User-supplied names and payloads flow into lists and logs |
| Outbox/job abuse: forged jobs, replays, poisoned payloads, floods | The email outbox is an async write path reachable by user actions |
| Supply-chain compromise (Go modules, CI actions) | One ecosystem, but the binary holds every secret |
| Resource exhaustion (large bodies, expensive queries, email floods) | Cheap authenticated routes can trigger costly work |

### 2.3 Process

- Every new slice, externally reachable route, outbound integration or data
  category requires a **threat model** committed at
  `docs/threat-models/{slice}.md` and reviewed in the PR: data-flow diagram,
  trust boundaries, threats, controls, residual risk, and the tests that prove
  the controls.
- Threat models are revisited on major change and at least annually; findings
  feed the risk register in `PLAYBOOK_AUDIT.md`.
- Each slice has a named security owner.

## 3. Layers of defense

```text
Internet
  → Edge (optional reverse proxy / nginx): TLS 1.2+ (1.3 preferred), size limits, routing allow-list
    → HTTP adapter (slice http.go / module.go):
        authx.RequireAuth, authx.RequireRole, schema validation,
        body cap, security headers, CORS allow-list, {detail, code} envelope
      → Use case: AUTHORIZATION (role + ownership), invariants, idempotency,
        audit event
        → Domain: state machines, limits, pure
          → Adapters: parameterized sqlc queries, least-privilege DB role
Async: cmd/worker claims email_jobs (FOR UPDATE SKIP LOCKED) — payload carries
  ids only, re-authorized as a system actor; in-process hub is fan-out only
Outbound: one guarded HTTP client for any user- or operator-supplied URL
  (allow-list, resolve-once-and-pin, budgets); provider adapters use fixed
  configured URLs; INV-T7 network adapters refuse under <APP>_OFFLINE=true
Platform: Postgres, JSON logs, backups
```

## 4. Authentication

### 4.1 Passwords

- Implemented in `internal/platform/authx/password.go`: **bcrypt cost 10 over
  HMAC-SHA256(password, pepper)**, base64-encoded. The pre-hash removes
  bcrypt's 72-byte truncation; the pepper keeps a stolen `users` table from
  being crackable without the secret. `<APP>_PASSWORD_PEPPER` must be set in
  every non-dev environment.
- No composition rules. Accept long passphrases, reject known-breached and
  trivial passwords, and never force rotation without a compromise signal.
- Login and password-reset responses are uniform in message and timing for
  "unknown user" and "wrong password". Use the same code path to avoid
  account enumeration.

### 4.2 Sessions and tokens (current implementation)

| Item | Rule as implemented | File |
|---|---|---|
| Access token | HS256 JWT, `<APP>_ACCESS_TOKEN_TTL` default **15m**, issuer-bound, `WithValidMethods(["HS256"])` pinned | `platform/authx` |
| Refresh token | Opaque, **rotating** on every refresh, one `sessions` row per token, `<APP>_REFRESH_TOKEN_TTL` default **720h** | `features/authsession` |
| Delivery | `Authorization: Bearer` always accepted; optional cookie mode (`<APP>_AUTH_COOKIE`) sets `HttpOnly; Secure; SameSite=Strict; Path=/` | `platform/authx/middleware.go` |
| Revocation | Logout and "revoke all" deactivate the row; refresh re-checks user state so blocked/deactivated users cannot ride an old session | `features/authsession` |
| OAuth | GitHub and Google through the `authoauth` slice; state validated; `connect-url` requires an authenticated session | `features/authoauth` |
| Verification/reset codes | `<APP>_VERIFY_CODE_TTL` 24h, `<APP>_RESET_CODE_TTL` 30m; single-use | `platform/config` |

Rules:

- **Never trust `alg` from the token.** The verifier pins HS256 and the HMAC
  key. If a second service ever needs to verify without holding the signing
  secret, move to asymmetric keys (EdDSA/ES256) with a `kid` rotation — do not
  share the symmetric secret.
- **Rotation is a security control.** A refresh that presents a token whose
  row was already rotated is a replay signal: revoke the session family and
  log it.
- **Never accept a long-lived token in a URL.** The WebSocket route
  (`GET /api/v1/ws`) reads `?token=` because browsers cannot set headers on a
  WebSocket upgrade. That token must be the short-lived access token over
  `wss://`, and the route must never be logged with its query string. Any new
  stream endpoint follows the same rule.
- **Known gap — do not copy it.** `sessions.refresh_token` is stored and
  looked up in plaintext (`features/authsession/queries.sql`). The target is
  to store `sha256(refresh_token)` and look up by hash, so a database dump
  cannot be replayed. Track it; do not add a second plaintext token store.
- **MFA does not exist yet.** When it is added, it is mandatory for `ADMIN`
  and `MODERATOR` before any provider-key or audit-log surface is exposed.

### 4.3 CSRF

Bearer-header requests are not CSRF-able. Cookie mode is: every mutating
cookie-authenticated route relies on `SameSite=Strict` plus the exact-origin
CORS allow-list, and must reject a request whose `Origin` is present and not
allow-listed. New cookie-authenticated mutations require an explicit
double-submit token; do not relax `SameSite`.

## 5. Authorization

Implements `PLAYBOOK_URLS.md`; verified by the role-matrix and
ownership-isolation suites in `PLAYBOOK_TESTS.md`.

1. **Backend is the authority.** `authx.RequireAuth` parses the token and puts
   `authx.Identity{UserID, Email, Role, JTI}` on the context;
   `authx.RequireRole(rolex.Admin)` enforces `rolex.AtLeast`. Admin routes use
   the guard and fail closed.
2. **Deny by default.** A route registered without a guard is a bug; the
   route table in each slice's `module.go` is the review surface.
3. **Ownership in the query predicate.** Every load-by-id for user data
   includes `user_id = $caller` (or an equivalent join) in the SQL, not an
   `if` after the fetch. `user_id` comes from `authx.IdentityFrom(ctx)`, never
   from the body or a query parameter.
4. **404 over 403 when existence would leak.** A row owned by someone else is
   "not found" to the caller.
5. **BOPLA.** Explicit request DTOs (no mass assignment of `role`,
   `is_active`, `user_id`, provider keys) and explicit response DTOs (no
   password hashes, refresh tokens, internal flags).
6. **BFLA.** Admin functions require the admin guard even when the route name
   looks harmless. `MODERATOR` and `ADMIN` are separate ranks; do not conflate
   them in a handler.
7. **Policy is server-side only.** A client may hide a button for a role; that
   never replaces the guard or the use-case check.
8. **Batch and worker paths authorize too.** An outbox job carries an explicit
   user id; the worker resolves it and acts as a scoped system actor. It never
   trusts an id from an unvalidated payload field.
9. **Identifiers are defense in depth, not the control.** UUIDs everywhere;
   never treat unpredictability as authorization.
10. **No row-level security today.** The application connects with one role
    against a single `public` schema; isolation is application-enforced. Do
    not describe the system as RLS-isolated. If a second writer service
    appears, revisit.

Required shape:

```go
func (uc UseCase) Update(ctx context.Context, in UpdateInput) (Row, error) {
    id, ok := authx.IdentityFrom(ctx)
    if !ok {
        return Row{}, apperr.E(apperr.CodeUnauthorized, apperr.AuthUnauthorized, "authentication required")
    }
    row, err := uc.store.ByIDOwned(ctx, in.ID, id.UserID) // owner in the predicate
    if err != nil {
        return Row{}, err // apperr.ErrNotFound when not owned — no existence leak
    }
    return uc.store.UpdateOwned(ctx, row.ID, id.UserID, in.Patch) // explicit DTO
}
```

## 6. Input handling

- **Schema at the boundary.** `httpx.Decode` caps the body at **1 MiB** and
  runs validator tags; `httpx.DecodeStrict` additionally rejects unknown JSON
  fields. New mutating endpoints use `DecodeStrict`; the generated
  `oapi-codegen` models keep `additionalProperties: false` honest.
- **Validate again in the domain** where a value becomes an invariant
  (state transitions, length limits), using value objects.
- **Canonicalize before validating** (trim, Unicode normalize, URL normalize),
  then validate the canonical form.
- **Parameterize everything.** sqlc generates prepared statements; never
  concatenate SQL, identifiers included. Dynamic sort/filter columns come from
  a fixed allow-list.
- **Path handling**: generated storage names; resolve to an absolute path and
  verify it stays inside the allowed root; never join user input to a path.
- **No shell.** Execute with an argument array, absolute binary path, minimal
  environment, timeout and resource limits. Prefer libraries.
- **Regex**: prefer RE2 semantics (Go's `regexp`); reject user-supplied
  patterns where possible; cap input length before matching.
- **Output sinks**: JSON responses only, no HTML rendering in the backend.
  CSV exports neutralize formula prefixes (`=`, `+`, `-`, `@`).
- **Mass assignment**: map request DTOs to domain objects field by field.

### 6.1 Fetched and model output is untrusted

- Provider responses, OAuth profile payloads and any fetched document are
  **data, never code**. No templating, no evaluation, no use as filenames, SQL
  fragments or URLs without validation.
- Enforce length and charset limits at ingestion; keep the raw artifact
  separate from normalized fields.
- When a model processes untrusted content: separate instructions from data,
  restrict tool access, validate output against a schema, never let model
  output decide authorization, and log prompts/responses per policy.

## 7. SSRF and outbound traffic

The skeleton performs outbound sockets in exactly two places: `oauthx`
(fixed Google/GitHub endpoints) and the SMTP mailer (fixed host). Both are
provider adapters with configured base URLs and both refuse under
`<APP>_OFFLINE=true`.

The moment a slice fetches a **user- or operator-supplied URL**, it must go
through one guarded client that provides:

- host and scheme allow-list, exact-port handling, credentials-in-URL
  rejection, redirects re-validated per hop;
- DNS resolved once and the connection **pinned to the validated address**
  (defeats rebinding); private/loopback/link-local/multicast/metadata ranges
  are denied, and private access is opt-in only at the policy layer, never a
  default;
- request budgets, bounded response reads and capped `Retry-After` retries;
- blocked fetches counted and surfaced.

Rules:

- A new target-facing outbound integration must use that guard, or justify a
  different control in review. `http.Get` on an operator- or user-supplied URL
  is a finding.
- Provider adapters (OAuth, SMTP) use fixed, configured base URLs — never a
  URL taken from a request.
- `<APP>_OFFLINE=true` must fail every outbound path cleanly and keep the
  product usable; that mode is also the test that no provider is load-bearing
  for core function. `config.Load` already rejects SMTP credentials combined
  with offline mode.
- Never reflect raw upstream errors to the caller; map them to `apperr`.

## 8. Cryptography and secrets

- **TLS everywhere** in any deployment with more than one host: 1.2 minimum,
  1.3 preferred, HSTS at the edge, certificate expiry monitoring.
- **Never implement your own crypto.** Use the standard library or vetted
  packages. CSPRNG (`crypto/rand`) for every token, session id, nonce and
  verification code; constant-time comparison (`crypto/subtle`) for secrets.
- **At-rest encryption** for any new restricted configuration. Values are
  write-only through the API: never returned by a GET, never logged, never
  placed in a job payload — pass the key id and resolve inside the adapter
  that uses it.
- **Secrets** (`<APP>_JWT_SECRET`, `<APP>_PASSWORD_PEPPER`,
  `<APP>_GITHUB_CLIENT_SECRET`, `<APP>_SMTP_PASS`) live in the environment or a
  secrets manager. Startup fails when a required secret is missing, too short
  or a known placeholder — keep that check (`config.Load`).
- **Rotation**: document the procedure per secret; a rotation that
  invalidates all sessions is acceptable if it is announced. OAuth client
  secrets rotate from the console without a code change.
- **Data minimization and retention**: store what the feature needs; the
  janitor deletes expired sessions and stale codes; retention jobs cascade
  across tables and backups; no personal data in analytics.

## 9. API and application hardening

### 9.1 Limits and resource consumption

- Body cap 1 MiB (`httpx`), `ReadHeaderTimeout`, and per-handler timeouts on
  expensive operations.
- Long or expensive work returns a run/resource and is executed by a worker;
  no request handler blocks on a long-running task.
- The auth slice has its own gates (`<APP>_MAX_LOGIN_RETRIES`,
  `<APP>_RESET_RATE_LIMIT` / `<APP>_RESET_RATE_WINDOW`). There is **no general
  API rate limiter yet**; until one exists, new expensive endpoints must carry
  their own budget and the gap must be in the threat model.

### 9.2 CORS, CSRF and headers

Implemented in `cmd/api/main.go` / `internal/platform/httpx`:

- `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `Referrer-Policy: no-referrer`, `Cache-Control: no-store`.
- CORS is an **exact-origin allow-list** from `<APP>_CORS_ORIGINS`; `*` only in
  development. Preflight answers without touching the mux.
- In dev, a SPA proxies `/api` through Vite, so there is no CORS preflight at
  all; the allow-list matters for split deployments.

### 9.3 Webhooks and callbacks

- Inbound webhooks (OAuth callbacks, provider callbacks) verify signature or
  state before parsing the body, use a raw-body HMAC where the provider
  specifies it, reject replayed timestamps, and are idempotent.
- An unverified webhook is an unauthenticated write: deny by default.

### 9.4 Errors and exceptional conditions

- Errors map through `internal/shared/apperr` to `{ "detail", "code" }`
  (`httpx.WriteErr`). Never leak stack traces, SQL, provider bodies or
  internal paths.
- Panics are recovered at the top of the chain, logged with the request id,
  and answered `500` with a generic detail.
- `not found` does not disclose whether an id exists and was denied (see §5,
  rule 4).

### 9.5 Inventory and exposure

- `GET /healthz` and `GET /readyz` are mounted outside auth and expose no
  configuration. Anything else requires the session.
- The route table is the inventory: each slice's `module.go` lists every
  route and its guard in one place. A route added elsewhere is a review
  finding.

## 10. Batch, outbox and event security

- **Jobs are untrusted input.** Validate the schema, resolve the owner from a
  trusted field, re-authorize as a scoped system actor. Do not assume
  "internal" means safe.
- **Idempotency** is structural: terminal-safe state transitions and unique
  constraints make replays harmless. Security-relevant jobs also check
  freshness.
- **No secrets or personal data in payloads**: pass ids and resolve inside the
  authorized handler. Email bodies are rendered inside the worker, not stored.
- **Abuse controls**: per-user enqueue quotas and concurrency caps, maximum
  payload size, backpressure; one user cannot starve the worker belt.
- **Leases and locks** use unpredictable tokens with fencing so a stale
  holder cannot overwrite state (`PLAYBOOK_BACKEND.md` §5). `FOR UPDATE SKIP
  LOCKED` is the claim primitive, not a mutex.
- The in-process hub fans out already-authorized data to a connection that was
  authenticated at upgrade; it never widens visibility.
- DLQ and replay tooling is admin-only and audited.

## 11. Supply chain

- **Lockfiles are mandatory**: `go.sum` with `-mod=readonly`. `go.sum` is not
  committed in the template (INV-T3); once the project is instantiated it is
  committed and a lockfile change is reviewed like code.
- **Cooldown for new releases**: do not adopt a version published in the last
  7–14 days unless it is an urgent, hand-reviewed security fix.
- **Minimize dependencies**; justify each new one (maintainers, activity,
  license, transitive count). Prefer the standard library.
- **Continuous SCA**: `make vuln` (govulncheck) runs at the release step; SLAs
  come from `PLAYBOOK_AUDIT.md`.
- **CI hardening**: pin third-party actions by commit SHA, minimal token
  permissions, no secrets to untrusted forks, protected branches, required
  reviews.
- **Release integrity**: build from a clean checkout; publish checksums.

## 12. Deployment (self-hosted)

- docker-compose is the default deployment: no port published beyond the
  API/edge, Postgres on the private network, no default credentials, `.env`
  never committed.
- **Postgres roles**: separate migration (DDL) and application (DML) roles;
  the app is never superuser; `sslmode` verified when the database is remote.
- **Filesystem**: the binary runs as a non-root user; artifact directories are
  read-only at runtime where possible.
- **Backups**: encrypted, restore-tested, and covered by the retention policy
  in `PLAYBOOK_AUDIT.md`.
- Feature flags cannot disable a security control.

## 13. Logging, monitoring and response

### 13.1 What to log

Structured `slog` JSON with `request_id`, actor (`user_id` or system), action,
resource, outcome, reason:

- authentication success and failure, refresh replay detection, logout;
- every authorization denial, with the required role and the route;
- privileged actions: role changes, user suspension, config changes, exports;
- validation floods, webhook signature failures;
- outbound provider failures and budget exhaustion.

Never log tokens, cookies, passwords, provider keys, full request bodies or
unnecessary personal data. Neutralize newlines/control characters.

### 13.2 Detection

- Alerts for: brute-force and credential stuffing, a burst of 403/404 across
  many ids (enumeration), new admin grants, key rotation outside a change
  window, mass export, outbox growth, unusual egress, provider spend spikes,
  repeated webhook signature failures.
- `/readyz` reflects dependencies; `/healthz` is liveness only.

### 13.3 Incident readiness

Runbooks for: compromised user session, leaked OAuth secret, cross-user
exposure, dependency/CI compromise, database loss. Prebuilt, tested switches:
global session revocation, OAuth secret rotation, per-feature disable
(`<APP>_OFFLINE=true`), route-level block at the edge, queue pause, read-only
mode. Preserve evidence before destroying anything; follow legal notification
clocks.

### 13.4 Disclosure

`security.txt`, a monitored security mailbox, triage SLAs and safe-harbor
language.

## 14. Secure development lifecycle

| Stage | Required |
|---|---|
| Design | Threat model, ASVS level, architecture review for new slices and integrations |
| Code | This playbook, pre-commit secret scan, linters, review with the checklist below |
| Build | SAST, SCA, secrets scanning as PR gates; architecture tests (depguard) |
| Test | Role matrix, ownership isolation, SSRF cases, injection, fuzzing of parsers and message handlers, DAST on staging |
| Release | Change record, checksums, rollback plan |
| Operate | Monitoring, vulnerability SLAs, patching, key rotation, access reviews, backups |
| Retire | Credential revocation, data deletion, DNS/route removal, dependency removal |

### 14.1 Security review checklist for every PR touching a boundary

- [ ] New route has a declared guard and a role-matrix row.
- [ ] Owner id comes from `authx.Identity` and is in the query predicate.
- [ ] Request DTO is explicit; `DecodeStrict` on mutating endpoints.
- [ ] No string-built SQL, shell call, unsafe deserialization or path join.
- [ ] Outbound URL goes through the guarded client; no raw `http.Get`.
- [ ] Errors map to `apperr` and leak nothing.
- [ ] No secret, token or personal data in logs, payloads or responses.
- [ ] New dependency justified, locked, scanned and past the cooldown.
- [ ] Security event emitted for the privileged action.
- [ ] Tests cover the abuse case, not only the happy path.
- [ ] Threat model updated when the trust boundary moved.

## 15. Go profile

| Control | Go |
|---|---|
| Schema validation | `httpx.DecodeStrict` + validator tags; sqlc-generated types |
| Queries | sqlc (prepared), pgx |
| Password hashing | `authx.HashPassword` (bcrypt + pepper) |
| CSPRNG | `crypto/rand` |
| Constant-time compare | `crypto/subtle` |
| Server timeouts | `http.Server{ReadHeaderTimeout, ReadTimeout, WriteTimeout, IdleTimeout}` |
| Body size | `httpx` cap (1 MiB) |
| Outbound + SSRF | one guarded client for user-supplied URLs; provider adapters use fixed URLs |
| SAST | `gosec`, `staticcheck`, depguard |
| SCA | `govulncheck` (`make vuln`) |
| Race checks | `go test -race` |

- `context.Context` first on every I/O path; the request context carries the
  identity. Never spawn a goroutine without an owner and a shutdown path.
- Errors: wrap with `%w`, match with `errors.Is`/`errors.As`, map to `apperr`
  only at the HTTP boundary.
- `domain.go` imports stdlib and `internal/shared` only (depguard enforces
  it); no I/O in the domain.
- Never `os/exec` with interpolated strings; use an argument array.
- Deserialize JSON with the shared helpers, not a bare `json.Unmarshal` on a
  request body.

## 16. Concrete example: a user asks for another user's session

Feature: `GET /api/v1/sessions/{id}` (user twin) and
`GET /api/v1/admin/sessions/{id}` (admin twin), slice `authsession`.

```text
internal/features/authsession/http.go         # admin route vs user route; identity from the context
internal/features/authsession/usecase.go      # SessionDetailOwned passes the owner down
internal/features/authsession/postgres.go     # GetSession(ctx, id, owner) -> sqlc owner param
specs/features/auth-session.feature           # the behavioral contract
internal/features/authsession/http_test.go    # TestGateAdmin / TestGateUser
internal/features/authsession/usecase_test.go # TestSessionDetailNotFound
internal/features/authsession/postgres_test.go # TestUIDFailClosed
```

```go
// http.go — the admin twin carries the admin guard; the user twin does not.
mux.Handle("GET /api/v1/admin/sessions/{id}", adminHF(h.sessionBundle))
mux.Handle("GET /api/v1/sessions/{id}", authHF(h.userSessionBundle))

// userSessionBundle — identity comes from the context, never the request.
ident, ok := authx.IdentityFrom(r.Context())
if !ok {
    httpx.WriteErr(w, ErrUnauthorized)
    return
}
b, err := h.uc.SessionDetailOwned(r.Context(), r.PathValue("id"), ident.UserID)
```

```go
// usecase.go — the owner is an argument, not a post-fetch check.
func (u *UseCase) SessionDetailOwned(ctx context.Context, id, owner string) (SessionBundle, error) {
    session, ok, err := u.store.GetSession(ctx, id, &owner)
    // ... ErrSessionNotFound when !ok
}
```

```go
// postgres.go — the owner rides the generated query parameter.
r, err := p.q.SessionByID(ctx, db.SessionByIDParams{ID: id, Owner: uid(owner)})
```

The invariants the tests pin:

- A foreign or unknown id answers `404`; `TestUserRoute404` and
  `TestSessionDetailNotFound` pin the not-found path, so the user route never
  reveals that another user's row exists.
- The admin bundle and the user bundle share one wire shape; only the guard
  and the `nil` owner differ (`TestGateAdmin`, `TestGateUser`).
- The owner predicate is in the SQL, not an `if` after the fetch; a missing or
  malformed owner cannot silently widen the query (`TestUIDFailClosed`).
- The admin list is deliberately not ownership-filtered; the user list is.

The anti-pattern this replaces: fetch by id, then `if row.OwnerID !=
ident.UserID { return 403 }`. That leaks existence and leaves list and worker
paths unprotected.

## 17. Checklist

- [ ] Every route is registered with an explicit guard; the route table is
      reviewed as the inventory.
- [ ] Authorization is decided in the use case from `authx.Identity`, never
      from the URL, a header, the body or a query parameter.
- [ ] Every user-data load-by-id carries the owner in the SQL predicate.
- [ ] A foreign row answers `404`, never `403`; the error envelope leaks no
      existence.
- [ ] Admin and user twins share the wire shape and differ only in guard and
      owner scope.
- [ ] Request and response DTOs are explicit; privileged fields cannot be
      mass-assigned and secrets cannot be serialized.
- [ ] Workers and outbox jobs re-authorize and never trust an id from an
      unvalidated payload.
- [ ] All input is validated at the boundary (`DecodeStrict`, size caps,
      content type) and every query is parameterized.
- [ ] Outbound fetches go through the guarded client; no raw `http.Get` on a
      user- or operator-supplied URL.
- [ ] Secrets live in the secret manager, are write-only through the API and
      never appear in logs or job payloads.
- [ ] Security events (denied access, privileged mutation, auth failure) are
      emitted and retained.
- [ ] `<APP>_OFFLINE=true` remains a complete configuration.
- [ ] Dependencies are locked and scanned; the SLAs in the
      vulnerability-management policy are met.
- [ ] The threat model and the role matrix are updated when the trust
      boundary moves.
