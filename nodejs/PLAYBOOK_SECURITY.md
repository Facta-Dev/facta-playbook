# PLAYBOOK_SECURITY.md — security contract (Node backend)

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
| `users` PII (email, name, phone, address) | privacy, phishing                           |
| `audit_logs`                              | non-repudiation and incident reconstruction |
| Admin capability                          | destructive writes, data exposure           |
| SMTP and OAuth credentials                | outbound abuse, provider account loss       |

Actors: anonymous internet, authenticated user, compromised user session,
malicious admin (insider), operator with env access, dependency attacker.

### 2.2 `<app>`-specific threats

| #   | Threat                      | Current control                                      | Residual risk                                                                           |
| --- | --------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------- |
| T1  | Credential stuffing         | bcrypt cost 10, retry lock, generic errors           | no global rate limit, no CAPTCHA, no breach-password check                              |
| T2  | Account enumeration         | same error for unknown email and wrong password      | timing differs (unknown email skips bcrypt); `signup` returns `409 AUTH_EMAIL_TAKEN`    |
| T3  | Brute-force on codes        | 6 digits, 5 attempts, TTL, reset rate 3/5 min        | no IP-based limit; distributed guessing possible                                        |
| T4  | Token theft from logs       | none                                                 | **WS `?token=` reaches Fastify's default request log**                                  |
| T5  | IDOR                        | ownership predicates in SQL; admin rank guard        | any new query that omits the owner predicate breaks it                                  |
| T6  | Refresh-token theft from DB | rotation with CAS                                    | tokens stored **plaintext**; a DB read exposes live sessions                            |
| T7  | Account lockout DoS         | lock after 5 wrong passwords                         | **anyone can lock any known email** until an admin unblocks                             |
| T8  | XSS via stored names        | JSON API only; no HTML rendering server-side         | clients must escape; never build HTML from API data in email templates without escaping |
| T9  | CSRF on cookie mode         | SameSite=Strict cookie; CORS exact allowlist         | `AUTH_COOKIE=true` + a misconfigured allowlist reopens it                               |
| T10 | SQL injection               | Kysely parameterization; no string-built SQL         | a raw `sql` fragment with user input would bypass it                                    |
| T11 | SSRF                        | OAuth endpoints fixed; offline guard refuses sockets | `SMTP_HOST` is operator-controlled, not user-controlled                                 |
| T12 | Admin bootstrap abuse       | `ALLOWED_ADMIN_EMAILS` whitelist                     | route is public; promote only during bootstrap and remove it                            |
| T13 | Dependency compromise       | semver pins, `pnpm audit` in CI                      | **no lockfile ships (INV-T3)**; no SBOM/provenance scan                                 |
| T14 | DoS via large bodies        | `bodyLimit` 1 MiB                                    | no per-route rate limit or concurrency cap                                              |
| T15 | Log injection               | Pino JSON encodes values                             | user strings still reach logs via error details; keep 5xx generic                       |

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
edge (TLS, proxy limits) → Fastify hooks (CORS, headers, request id)
  → route guard (requireAuth/requireRole) → use case (rules)
  → adapter (parameterized SQL) → database (constraints, FKs, checks)
```

Each layer assumes the one above may fail. The database constraints are not
decoration: the `CHECK` on `role`/`user_type`, the unique email, the FK
cascades and the partial indexes are part of the security model.

---

## 4. Authentication

### 4.1 Passwords

- Stored as bcrypt (cost 10) over an HMAC-SHA256 pre-hash keyed by
  `<APP>_PASSWORD_PEPPER` when set. The pre-hash defeats bcrypt's 72-byte
  truncation and keeps the pepper out of a stolen `users` table.
- Without a pepper, the raw password feeds bcrypt and silently truncates past
  72 bytes — documented ceiling. Set a pepper in production.
- Verification is `bcrypt.compare`; user-supplied comparison never uses `===`.
- Password policy: minimum length 8 at signup (`zod`) and at
  reset/change (the use case). No composition rules, no maximum beyond the
  bcrypt ceiling; if a policy is added, cap length and reject known-breached
  passwords deliberately.
- Never log a password, even at `debug`. Never put one in an error detail.

### 4.2 Sessions and tokens

- **Access token**: JWT HS256, `iss = "<app>"`, TTL
  `<APP>_ACCESS_TOKEN_TTL` (15m). The verifier pins `algorithms: ["HS256"]`
  and checks issuer + expiry. No `alg: none`, no RS/HS confusion.
- **Refresh token**: 48 random bytes, base64url, opaque. Stored in
  `sessions.refresh_token` (VARCHAR 128, unique). Rotation is in-place with a
  compare-and-swap on the old token value: a replay of the old token fails
  and the session stays inactive. Absolute expiry is fixed at creation
  (`REFRESH_TOKEN_TTL`, 720h); rotation does not extend it.
- **Cookie mode** (`AUTH_COOKIE=true`): `<app>_access_token`,
  `HttpOnly`, `SameSite=Strict`, `Secure` outside `env=dev`, path `/`.
- **Logout** revokes the refresh session; the access token remains valid
  until its short TTL expires. That is the accepted stateless tradeoff — do
  not claim instant revocation.
- **Reset** auto-verifies the account and revokes sessions when
  `LOGOUT_ALL_ON_RESET=true` (default).
- Refresh tokens are stored **plaintext**. A DB read grants live sessions.
  The upgrade is a keyed hash (HMAC with a server secret) on store and a
  lookup by hash; do not describe the current state as hashed.

### 4.3 Email codes

- 6 digits generated by rejection sampling (`newCode`), uniform over 0–9.
- Verification: 24h TTL; reset: 30m TTL; resets limited to
  `RESET_RATE_LIMIT` per `RESET_RATE_WINDOW` (3 per 5 min).
- 5 wrong attempts cap a code (`maxCodeAttempts`). Consumption is atomic
  (`markResetUsed` / `useVerification` return false when raced).
- Comparison is constant-time (`tokenEq`, length gate first).
- Codes are stored plaintext in their tables (short-lived by design). The
  janitor purges expired rows.

### 4.4 Login gates and lockout

Order matters and is deliberate: password first, then `is_blocked`,
`is_active`, `is_verified`. A wrong password never reveals account state.

`afterFailedLogin` increments `retry_count` in SQL and sets
`is_blocked = retry_count + 1 >= MAX_LOGIN_RETRIES` (default 5). A
successful login resets the counter.

**Residual risk (T7):** the lock is permanent until an admin unblocks the
account, so an attacker who knows an email can deny service to it. Mitigate
with a notification to the account owner and an admin unlock runbook; the
proper fix is a time-based lockout with exponential delay, which requires a
schema change (`locked_until`) and is not shipped.

### 4.5 OAuth

- Google: One Tap ID token verified against Google's JWKS over HTTPS,
  audience = client id. Empty client id = unconfigured → `401`.
- GitHub: auth-code exchange against fixed GitHub endpoints; client secret
  from env; callback URL is configured, not taken from the request.
- Both are disabled under `<APP>_OFFLINE=true` at the request boundary,
  before any socket opens (INV-T7).
- Linking rule: provider id → email → provision an auto-verified user. State
  is carried by the provider flow; a new provider must add CSRF `state`
  handling before it ships.

### 4.6 CSRF

Cookie mode relies on `SameSite=Strict` plus the exact-origin CORS allowlist.
There is no CSRF token. That is acceptable only while:

- cookies are `SameSite=Strict`;
- no state-changing `GET` exists;
- the CORS allowlist contains no wildcard outside development.

A cross-site form POST to a JSON endpoint can still arrive; Fastify parses
JSON only, and the endpoint requires `Origin`-allowed CORS for browser reads.
Do not loosen `SameSite` without adding a token.

---

## 5. Authorization

- Roles: `GUEST < USER < MODERATOR < ADMIN` (`src/shared/rolex.ts`).
  `requireRole(min)` checks rank ≥ min. RBAC reads the role column, never
  `user_type`.
- Route guard alone is insufficient. Every mutation re-checks:
  - **ownership** in SQL (`where owner_user_id = $owner`) for user data;
  - **rank** via `authz` for admin actions: `canManageSelf` (same id, token
    rank ≥ row rank) and `canManageSubordinate` (token rank **strictly** >
    row rank). An ADMIN cannot manage another ADMIN.
- A stale token whose role no longer matches the DB row must not outrank the
  row: `canManageSelf` compares against the DB role.
- A foreign or unowned row returns `404`, not `403`. Do not confirm existence.
- An authenticated but disallowed action returns `403 AUTH_FORBIDDEN`.
- The admin surface writes `audit_logs` in the same use-case path as the
  change. An admin change without an audit row is a defect.
- `POST /api/v1/auth/upgrade-to-admin` is public and gated only by the
  `ALLOWED_ADMIN_EMAILS` list. Treat it as a bootstrap tool: remove it after
  the first admin exists.
- No impersonation, no "act as" header, no tenant header. If one is ever
  added, it is an audited, role-gated feature, not a header passthrough.

---

## 6. Input handling

- **Shape** is validated at the edge with zod (`httpx.decode`), producing
  `422 VALIDATION_ERROR`. The schema caps string lengths, array sizes and
  numeric ranges; an unbounded string is a DoS vector.
- **Body limit** is 1 MiB (`bodyLimit` on the Fastify instance). Keep it.
- **Path params** are treated as opaque strings; a UUID-shaped id is checked
  by the query, not by a regex that leaks format.
- **Query params** go through `queryPage` (limit 1–100, offset ≥0). Do not
  read `req.query.limit` directly.
- **Unknown fields** are ignored by zod's default object parsing. If mass
  assignment matters, use `.strict()` or pick fields explicitly — never
  spread a request body into an update.
- **Output encoding** is the client's job for HTML; the API returns JSON with
  `content-type: application/json`. Never interpolate user data into an HTML
  error page.
- **Email templates** interpolate `{{vars}}`; escape HTML in templates when a
  var can contain user text. A name is user text.

### 6.1 Fetched and model output is untrusted

No LLM/model output and no third-party API response is trusted. A provider
response is parsed, typed and validated before it reaches the domain. It
never becomes SQL, a shell argument, a file path, or a redirect target.

---

## 7. SSRF and outbound traffic

- Outbound destinations are fixed constants: Google JWKS, GitHub token/user
  endpoints, the configured SMTP host.
- No user input selects a URL, host, port or redirect target.
- `<APP>_OFFLINE=true` refuses OAuth before dialing and forbids SMTP
  credentials at boot; the console sender never opens a socket (INV-T7).
- If a webhook or image-fetch feature is added, it needs an allowlist, DNS
  rebinding protection, no redirect following, and a private-IP block.
  "We validate the scheme" is not enough.

---

## 8. Cryptography and secrets

- **JWT secret**: ≥32 random bytes, `<APP>_JWT_SECRET`; the loader rejects
  placeholders and short values. Rotating it invalidates every access token
  and every refresh session (refresh rows are opaque and unaffected; access
  tokens are not). Rotation needs a dual-key verification window if
  zero-downtime matters — not shipped.
- **Pepper**: ≥32 bytes when set. Rotating it invalidates every password
  hash; treat it as a migration, not a config change.
- **Randomness**: `node:crypto` only (`randomUUID`, `randomBytes`). Never
  `Math.random()` for ids, tokens or codes.
- **Comparison**: `crypto.timingSafeEqual` for secrets (codes); `bcrypt` for
  passwords. Never `===` on a secret.
- **Secrets live in env**, not in the repo. `.env` is gitignored;
  `.env.example` contains placeholders only. A secret in a commit is
  compromised: rotate it, do not just delete the line.
- **Database credentials** are operator-provided; the app never creates
  roles or databases.
- **No custom crypto.** Use `node:crypto`, `jose`, `bcryptjs`. A hand-rolled
  cipher/hash is rejected in review.

---

## 9. API and application hardening

### 9.1 Limits and resource consumption

- Body 1 MiB; list page 100; outbox batch configurable.
- No per-IP or per-account rate limit ships. Put one at the edge (nginx) or
  add `@fastify/rate-limit` with a store; the reset flow's per-email limit is
  not a substitute.
- Password hashing is CPU-bound; a burst of login attempts is a CPU DoS.
  Rate-limit before bcrypt, not after.

### 9.2 CORS, headers, cookies

- CORS is an exact-origin allowlist (`CORS_ORIGINS`); `*` is accepted only in
  development. Credentials are not enabled with `*`; in cookie mode, list
  origins explicitly.
- Response headers: `x-content-type-options: nosniff`, `x-frame-options:
DENY`, `referrer-policy: no-referrer`, `cache-control: no-store`.
  No CSP is set (JSON API); if an HTML surface is added, set one at the edge.
- Cookies: `HttpOnly`, `SameSite=Strict`, `Secure` outside dev.
- `x-request-id` is echoed; it must not be trusted for authorization.

### 9.3 Webhooks and callbacks

None shipped. If added: verify the signature with a constant-time compare,
bind the timestamp window, reject replays by event id, and process
idempotently.

### 9.4 Errors and exceptional conditions

- One envelope `{detail, code}`; 5xx details are generic.
- Never include SQL, driver messages, stack traces, env values, tokens or
  emails in a response.
- A 404 for an unowned row is intentional; do not "fix" it to 403.
- Log the stable `code` and request id for 5xx; keep the user-facing detail
  useless to an attacker.

### 9.5 Inventory and exposure

Shipped surface is listed in `PLAYBOOK_URLS.md` §2.1. It is the inventory;
reconcile it with the route table each release. No `/metrics`, no `/docs`, no
debug endpoints. A new endpoint is a security review item, not a convenience.

---

## 10. Outbox and event security

- The outbox prevents inline delivery, but it is not a trust boundary: the
  payload is JSON written by the app. Treat it as untrusted on read
  (validate the shape, cap size) in case a row is tampered with.
- Retry backoff is bounded (`2^n` minutes + jitter) and dead-letters; a
  poisoned row cannot retry forever.
- A worker claims with `FOR UPDATE SKIP LOCKED`; multiple replicas are safe.
- The lease (5 minutes) means a crashed worker's mail can be delivered twice.
  At-least-once is the contract; templates must be safe to repeat.
- Realtime events carry only data the same identity could read over REST.
  A room is not an authorization mechanism; it is a routing key.
- The WS `?token=` is an access token. It must not be logged, cached, or
  stored client-side longer than the session.

---

## 11. Supply chain

- Dependency ranges are pinned to modern majors; `pnpm audit
--audit-level=high` runs in CI.
- **The template commits no `pnpm-lock.yaml` (INV-T3).** Installs resolve
  newest-in-range, so builds are not reproducible and a compromised release
  can be picked up. After instantiation, commit a lockfile and use
  `pnpm install --frozen-lockfile` in CI; remove the ignore entry.
- No SBOM, no provenance/signature verification, no container scan ships.
  Add Syft + Trivy (or equivalent) as advisory jobs, then gating ones.
- Review dependency additions for install scripts and typosquats. A new
  package is a code review, not a one-line `package.json` edit.
- Do not run `pnpm` with elevated privileges; the Docker build runs as
  `node`, not root.

---

## 12. Deployment (self-hosted)

- TLS terminates at the edge (`infra/nginx`); the app speaks HTTP inside the
  network. Never expose the API port directly.
- The container runs as the non-root `node` user and has a healthcheck.
- Postgres is not exposed publicly; credentials come from the platform's
  secret store, not from a committed compose file.
- `docker-compose.prod.yml` overrides dev defaults; verify it does not carry
  dev passwords or `*` CORS.
- Migrations run as a one-shot job, not from the API process. A failed
  migration stops the rollout.
- Backups: encrypted, tested by restore, with a retention policy. A backup
  that has never been restored is a hypothesis.

---

## 13. Logging, monitoring and response

### 13.1 What to log

Log: request id, method, path (no query secrets), status, latency, stable
error code, actor id on authenticated actions, admin audit entries, worker
lifecycle, janitor counts.

Never log: passwords, tokens, cookies, authorization headers, full request
bodies on auth routes, SMTP credentials, the WS query string.

### 13.2 Detection

Alert on: 5xx rate, `AUTH_ACCOUNT_BLOCKED` spikes (lockout abuse),
`AUTH_RATE_LIMITED` spikes, repeated `AUTH_INVALID_CREDENTIALS` from one IP,
outbox `failed` growth, readiness failures, migration job failure.

### 13.3 Incident readiness

- One on-call owner; one channel; one severity scale.
- Break-glass: rotate `JWT_SECRET` (invalidates access tokens), rotate the
  pepper only with a password-reset plan, revoke sessions in SQL, rotate
  SMTP/OAuth secrets.
- Preserve logs before remediation; note the timeline.
- Post-incident: root cause, control gap, test that would have caught it.

### 13.4 Disclosure

A `security.txt` or a published contact is the minimum. Acknowledgment
within 3 business days; do not promise a bounty you do not run.

---

## 14. Secure development lifecycle

### 14.1 Review checklist for every PR touching a boundary

- [ ] Input is zod-validated with explicit bounds.
- [ ] SQL is Kysely-parameterized; no user string in a `sql` fragment.
- [ ] Authorization checks ownership/rank at the handler, not only the route.
- [ ] A foreign row returns 404.
- [ ] No secret, token or password reaches a log or an error detail.
- [ ] New outbound call has a fixed destination and a timeout/signal.
- [ ] New config key is prefixed, validated and documented; no default
      secret.
- [ ] A regression test covers the control.
- [ ] Dependency addition reviewed; no install script.

### 14.2 Node-specific profile

| Risk                | Rule                                                                              |
| ------------------- | --------------------------------------------------------------------------------- |
| Prototype pollution | never deep-merge request bodies; `Object.create(null)` or explicit picks for maps |
| ReDoS               | no user-supplied regex; bound any regex over user text                            |
| Path traversal      | no `fs` on request-derived paths; if added, resolve + prefix check                |
| Command injection   | no `child_process` on request paths; never `shell: true`                          |
| `eval`/`Function`   | forbidden                                                                         |
| JSON depth          | body limit + zod; avoid unbounded recursive parse                                 |
| Timing              | constant-time compares for secrets; do not branch on token bytes                  |
| `Math.random`       | forbidden for security values                                                     |
| Unhandled rejection | a rejected promise must not crash the process silently; log + exit non-zero       |
| SSRF via fetch      | fixed URLs only; `redirect: "error"` for provider calls                           |

---

## 15. Concrete example: a user asks for another user's session

Request: `GET /api/v1/admin/auth-session/users/:id/sessions` with a USER
token for someone else's id.

1. Route posture: `requireRole(Admin)` rejects with `403 AUTH_FORBIDDEN`
   before the handler runs. The route is not reachable with a USER token.
2. If the route posture were missing, the handler's `authz` check
   (`canManageSubordinate`) would reject: rank(USER) is not strictly greater
   than rank(target), and the actor is not the target.
3. If the guard were also missing, the query is scoped to the target id but
   the handler would return another user's session metadata — which is why
   the guard is tested at both layers.
4. The response never includes the refresh token; session listings expose
   id, ip, user agent, activity and expiry only.
5. The attempt is logged with the actor id and the stable code; repeated
   attempts from one actor are an alert.

The test: acceptance asserts `403` for the USER token and `200` for an ADMIN
on a USER; a unit test asserts `canManageSubordinate` is strict. Both fail if
the guard regresses.

---

## 16. Known deviations — fix before production

| #   | Deviation                                                 | Impact                                            | Fix                                                     |
| --- | --------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------- |
| S1  | Fastify default request log includes `/api/v1/ws?token=…` | access token in logs                              | strip the query in a `serializers.req`                  |
| S2  | Refresh tokens stored plaintext                           | DB read = live sessions                           | store HMAC of the token; look up by hash                |
| S3  | Permanent lockout after N failed logins                   | account DoS by email                              | `locked_until` + exponential delay; notify the owner    |
| S4  | No global rate limit                                      | credential stuffing, CPU DoS                      | edge limit + `@fastify/rate-limit` with a shared store  |
| S5  | No lockfile in the template                               | non-reproducible installs; supply-chain drift     | commit `pnpm-lock.yaml`; `--frozen-lockfile` in CI      |
| S6  | Public `upgrade-to-admin`                                 | privilege escalation if the list is misconfigured | remove after bootstrap                                  |
| S7  | Timing difference on unknown email (no bcrypt)            | user enumeration                                  | run a dummy bcrypt compare when the user is absent      |
| S8  | No CSRF token in cookie mode                              | relies on SameSite alone                          | add a double-submit token if `SameSite` is ever relaxed |
| S9  | No SBOM/image/secret scan                                 | unmanaged supply chain                            | add Syft/Trivy/gitleaks as jobs                         |
| S10 | Access tokens not revocable before TTL                    | logout is not instant                             | shorten TTL or add a session-version claim              |

---

## 17. Checklist

- [ ] Passwords hashed with bcrypt + pepper; pepper set in production.
- [ ] JWT pinned to HS256, issuer and expiry checked.
- [ ] Refresh rotation is CAS; replay of the old token fails.
- [ ] Cookies `HttpOnly`, `SameSite=Strict`, `Secure` outside dev.
- [ ] Codes constant-time compared, attempt-capped, TTL-bound.
- [ ] Every route has an explicit posture; admin routes rank-gated.
- [ ] Ownership is enforced in SQL; foreign rows return 404.
- [ ] Admin mutations write `audit_logs`.
- [ ] Input zod-validated with bounds; body limit intact.
- [ ] No secret in a log, error, or the repo.
- [ ] Outbound destinations fixed; offline guard honored.
- [ ] Errors use the envelope and leak nothing.
- [ ] New dependency reviewed; lockfile policy decided.
- [ ] Threat row named in the PR; regression test added.
