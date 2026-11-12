# PLAYBOOK_SECURITY.md — Backend cybersecurity playbook

Security contract for every Heimdall backend, whatever the language. The
**core rules are language-agnostic**; section 16 gives **profiles for
Node.js, Python, Go and Rust**, the four languages used. It hardens the
architecture in `PLAYBOOK_BACKEND.md`, enforces the authorization model of
`PLAYBOOK_URLS.md`, is verified by `PLAYBOOK_TESTS.md` and produces the
control evidence required by `PLAYBOOK_AUDIT.md`.

The rules that carry the whole file:

1. **Deny by default, fail closed.** No route, permission, network path, port,
   credential or outbound destination exists unless explicitly granted.
2. **Defense in depth.** Every layer assumes the layer in front of it failed:
   edge → gateway → controller → use case → domain → data → network.
3. **Authorization lives in the use case**, with identity, tenant, permission
   and ownership checked on every call. The URL prefix never authorizes.
4. **Tenant isolation is the primary security boundary.** Cross-tenant data
   exposure is the worst-case incident for this product.
5. **All input is hostile**: HTTP, queue messages, events, webhooks, provider
   responses and, above all, scraped lead data.
6. **Secrets never live in code, images, logs or client bundles**, and
   credentials are short-lived wherever possible.
7. **Assume breach.** Limit blast radius, detect fast, contain with prebuilt
   switches, recover from tested backups.
8. **Security is enforced by automation**, not by memory or good intentions:
   linters, scanners, architecture tests and CI gates.
9. **Same baseline in every language.** A control that exists in Node must
   exist in Python, Go and Rust; only the implementation differs.

> Scope note: this is a defensive engineering baseline. It lowers risk; it
> does not guarantee that a system cannot be breached. Numeric values (token
> lifetimes, limits, parameters) are starting points to tune with your threat
> model and measurements. Re-verify tool names and versions when adopting
> them.

## 1. Standards and how this playbook maps to them

| Standard | Use |
|---|---|
| **OWASP Top 10:2025** | Risk taxonomy for web applications (table below) |
| **OWASP API Security Top 10 (2023)** | API-specific risks: BOLA, broken auth, BOPLA, resource consumption, BFLA, sensitive business flows, SSRF, misconfiguration, inventory, unsafe consumption of APIs |
| **OWASP ASVS (current 5.x)** | Verification requirements. Target **Level 2 baseline**, **Level 3** for authentication, session, cryptography, tenant isolation and admin modules |
| **OWASP Cheat Sheet Series** | Implementation detail per topic |
| **CWE Top 25** | Weakness classes tracked by SAST rules |
| **RFC 8725 (JWT BCP), RFC 9700 (OAuth 2.0 Security BCP)** | Token and OAuth handling |
| **NIST SSDF (SP 800-218), SLSA** | Secure development and supply-chain levels |
| **CIS Benchmarks** | OS, container, Kubernetes and cloud baselines |

OWASP Top 10:2025 → where it is handled here:

| ID | Risk | Section |
|---|---|---|
| A01 | Broken Access Control (includes SSRF) | 5, 7 |
| A02 | Security Misconfiguration | 9, 12, 13 |
| A03 | Software Supply Chain Failures | 11 |
| A04 | Cryptographic Failures | 8 |
| A05 | Injection | 6 |
| A06 | Insecure Design | 2, 15 |
| A07 | Authentication Failures | 4 |
| A08 | Software or Data Integrity Failures | 10, 11 |
| A09 | Security Logging and Alerting Failures | 14 |
| A10 | Mishandling of Exceptional Conditions | 9.4 |

## 2. Threat model

### 2.1 Assets and actors

| Assets (by priority) | Threat actors |
|---|---|
| Tenant lead data (personal data), tenant provider API keys (Apify, telephony, LLM), credentials and tokens, session store, embeddings, billing data, source code and CI secrets, signing keys | External attacker, credential stuffer/bot, malicious or compromised tenant user, malicious tenant (cross-tenant), compromised vendor or dependency, insider, compromised employee device, hostile content source (scraped pages/leads) |

### 2.2 Heimdall-specific threats

| Threat | Why it matters here |
|---|---|
| Cross-tenant read/write (IDOR/BOLA, missing tenant filter, cache or vector leakage) | Multi-tenant SaaS; the highest-impact failure |
| Privilege confusion via `{userType}` (using `/admin` with an `/app` token) | Route prefix is an experience namespace only |
| Theft or misuse of tenant provider keys | Keys let an attacker spend money and read leads |
| SSRF through webhook URLs, scraping targets, redirect following | Features that fetch arbitrary URLs |
| Stored injection from scraped content (XSS, template, SQL, prompt injection) | Lead data originates from the open internet |
| Queue/batch abuse: forged jobs, replay, poisoned messages, queue flooding, lock key tampering | Redis/Dragonfly is a coordination plane |
| Account takeover and credential stuffing | Employee consoles expose lead data |
| Abuse of outreach features (spam, harassment, fraud) | Reputational and legal exposure |
| Supply-chain compromise (npm, PyPI, crates, Go modules, CI actions, extension store) | Four ecosystems = four attack surfaces |
| Denial of service and resource exhaustion (large payloads, expensive queries, extraction loops) | Costly operations are exposed behind APIs |
| Insider or support-staff over-access | Administrative tooling and production access |

### 2.3 Process

- Every new slice, externally reachable route, queue, webhook, provider
  integration or data category requires a **threat model** (STRIDE or
  equivalent) committed at `docs/threat-models/{slice}.md` and reviewed in the
  PR. Minimum content: data-flow diagram, trust boundaries, threats, controls,
  residual risk and tests that prove the controls.
- Threat models are revisited on major change and at least annually; findings
  feed the risk register (`PLAYBOOK_AUDIT.md`).
- Each slice has a **security champion** responsible for its threat model and
  security tests.

## 3. Layers of defense mapped to the hexagon

```text
Internet
  → Edge: DDoS protection, WAF, TLS termination, bot controls, geo/IP rules
    → Gateway/ingress: TLS 1.2+ (1.3 preferred), global rate limits, size limits, routing allow-list
      → Presentation adapter (adapters/in/http): authentication, schema validation,
        RequestContext construction, security headers, CORS, error envelope
        → Application (use case): AUTHORIZATION (permission + tenant + ownership), policies,
          idempotency, audit events
          → Domain: invariants (state machines, limits)
            → Data adapters: parameterized access, RLS, field encryption, least-privilege DB roles
Batch/event adapters (adapters/in/jobs, events): authenticate the broker, validate message schema,
  verify tenant context, then call the same use cases as a system actor with minimal permissions
Outbound adapters (clients/): egress allow-list, SSRF-safe HTTP client, timeouts, response validation
Network: default-deny segmentation, private data plane, mTLS between services
Platform: secrets manager, KMS, logging, monitoring, backups
```

## 4. Authentication

### 4.1 Passwords and MFA

- Hash passwords with **Argon2id** (OWASP minimum: 19 MiB memory, 2
  iterations, 1 lane; prefer higher cost tuned to roughly 100–500 ms on your
  hardware). `bcrypt`/`scrypt` only for legacy compatibility, with migration
  on login.
- Salt unique per hash (library default), optional server-side pepper held in
  the secrets manager/KMS.
- Accept long passphrases (at least 64 chars), block known-breached and
  trivial passwords, no composition gymnastics, no periodic forced rotation
  without a compromise signal.
- **MFA**: WebAuthn/passkeys preferred, TOTP as fallback, SMS only as last
  resort. **Mandatory MFA** for `/admin` experience users, tenant owners,
  anyone who can read provider keys or export data, and all production access.
- Enterprise tenants: SSO through OIDC or SAML; enforce MFA at the IdP;
  provisioning via SCIM; just-in-time role mapping to permissions.

### 4.2 Credential-stuffing and abuse defenses

- Rate limit by IP, account and device fingerprint with progressive delays;
  global login anomaly detection; bot challenge on suspicious patterns.
- Uniform responses and timing for "unknown user" and "wrong password";
  generic account-recovery messages.
- Account recovery requires verified factor; recovery events notify the user;
  sensitive changes (email, password, MFA, API key) require re-authentication.
- Lockout is soft (delay + challenge) to avoid denial-of-service by lockout.

### 4.3 Sessions and tokens

| Item | Rule |
|---|---|
| Access token | Short-lived (5–15 minutes), audience- and issuer-bound, signed with **asymmetric keys** (RS256/ES256/EdDSA), `kid` header with rotation |
| Refresh token | Opaque, **rotating**, single use with **reuse detection** (reuse revokes the whole family), bound to client/device where possible, stored hashed server-side |
| JWT validation | Pin the allowed algorithm list, **reject `none`**, verify signature, `iss`, `aud`, `exp`, `nbf`, and token type; never trust `alg` from the header to pick the key; no sensitive data in claims; follow RFC 8725 |
| Claims | Carry `sub`, `tenant` (active), `session id`, role; **permissions are resolved server-side** from the session or a cache keyed by session, so revocation is immediate |
| Revocation | Session list and deny-list in the datastore/Dragonfly; logout, password change, role change, user suspension and detected compromise revoke immediately |
| Cookies (web) | `HttpOnly`, `Secure`, `SameSite=Lax` or `Strict`, `__Host-` prefix, minimal scope; CSRF protection for cookie-authenticated mutations (double-submit or synchronizer token, plus `Origin`/`Sec-Fetch-Site` checks) |
| Bearer tokens (mobile, extension, API) | OAuth 2.0/OIDC with **authorization code + PKCE** for public clients; no implicit flow, no password grant; follow RFC 9700 |
| Idle and absolute timeouts | Enforced server-side per policy |
| Tenant switch / role switch | New token, cache cleared, permissions re-resolved |

### 4.4 Service and machine authentication

- Service-to-service: **mTLS** or workload identity (for example SPIFFE/SPIRE,
  cloud workload identity) or short-lived signed service tokens. Never shared
  static passwords between services.
- Each service has its own identity and its own least-privilege access to
  datastores and queues.
- **Customer API keys** (if offered): random, high entropy, shown once, stored
  **hashed**, with a visible non-secret prefix, scopes, expiry, rotation,
  last-used timestamp and per-key rate limit; revocable instantly; never
  accepted in URLs (headers only).
- CI/CD authenticates to the cloud with OIDC federation, not stored keys.

## 5. Authorization

Implements `PLAYBOOK_URLS.md`; verified by `authz-matrix` and
`tenant-isolation` suites in `PLAYBOOK_TESTS.md`.

1. **Every use case authorizes**: permission (`module.resource.action`),
   **tenant match**, **resource ownership** and state-based rules. Controller
   checks are an early reject, not the authority.
2. **Deny by default**: a route or use case without a declared permission
   fails startup/CI.
3. **Tenant context comes from the token/session**, never from the body or an
   unvalidated header. `X-Tenant-Id` and path `{tenantId}` are validated
   against the caller's membership or the admin permission scope.
4. **Object-level (BOLA)**: every load by ID includes the tenant (and owner
   where applicable) in the query predicate. Return `404` rather than `403`
   when existence would leak.
5. **Property-level (BOPLA)**: explicit input DTOs (no mass assignment of
   `role`, `tenantId`, `ownerId`, `isAdmin`, prices) and explicit output DTOs
   (no returning internal fields, hashes, secrets).
6. **Function-level (BFLA)**: admin and partner functions require their own
   permissions even when the route name looks harmless.
7. **Centralized policy**: one policy module per slice (or a policy engine
   such as OPA, Cedar or Casbin if complexity grows). No scattered `if role ==
   admin`.
8. **Identifiers**: use unpredictable IDs (UUIDv4/v7, ULID) as defense in
   depth; never as the control.
9. **Second line of defense in the database**: row-level security keyed by
   `tenant_id`, with the application connecting as a role that **cannot
   bypass it** (not the table owner unless `FORCE ROW LEVEL SECURITY` is set,
   not a superuser, not a `BYPASSRLS` role).
10. **Impersonation and support access**: explicit, time-boxed, approved,
    fully audited, shown to the customer if policy requires; never silent.
11. **Sensitive business flows** (launch campaign, bulk export, send messages,
    spend credits) have quotas, step-up authentication where appropriate, and
    anomaly alerts.
12. **Batch jobs authorize too**: run as a system actor with explicit minimum
    permissions, always scoped to the message's verified `tenantId`.

Pseudocode of the required shape (language-agnostic):

```text
useCase.execute(ctx, input):
    require ctx.authenticated
    require policy.allows(ctx, "funnels.leads.update")           # permission
    lead = repo.findById(ctx.tenantId, input.leadId)              # tenant in predicate
    if lead is null: return NotFound                              # no existence leak
    require policy.canModify(ctx, lead)                           # ownership / state
    lead.apply(validated(input))                                  # explicit DTO, domain invariants
    repo.save(lead); outbox.add(LeadStageChanged(...)); audit.record(ctx, "lead.update", lead.id)
```

## 6. Input handling and injection (A05)

### 6.1 Rules for every boundary

- **Validate at the edge with a schema** (allow-list of fields, types, ranges,
  lengths, formats). Reject unknown fields by default. Validate again where a
  value enters the domain (value objects).
- **Canonicalize before validating** (Unicode normalization, trimming, path
  and URL normalization), then validate the canonical form.
- Strict `Content-Type`; only the formats you serve. Maximum body size, header
  size, field count, array length, nesting depth and string length.
- **Parameterize everything**: SQL, NoSQL, search, LDAP, XPath, GraphQL,
  shell, templates. Never build queries or commands by string concatenation,
  even with "escaped" input.
- **No shell**: execute subprocesses with an argument array, absolute binary
  path, minimal environment, timeouts and resource limits; prefer libraries
  over spawning processes.
- **Path handling**: never join user input to a filesystem path directly;
  resolve to an absolute path and verify it stays inside the allowed root;
  prefer generated names and object storage keys.
- **Deserialization**: only data formats (JSON) with schema validation. No
  native object deserialization of untrusted data (pickle, unsafe YAML,
  language-native serializers). XML: disable external entities and DTDs.
- **Regular expressions**: avoid catastrophic patterns; use linear-time
  engines (RE2-style) or timeouts for user-influenced patterns.
- **Output encoding by context** (HTML, attribute, JS, URL, SQL, log).
  Prefer JSON APIs that never render HTML server-side.
- **Mass assignment**: map DTOs to domain objects explicitly.

### 6.2 Scraped and third-party content is untrusted

- Lead fields extracted via Apify or any source are **data, never code**: no
  templating, no evaluation, no HTML rendering without sanitization, no use as
  filenames, query fragments or URLs without validation.
- Sanitize on output for each sink (UI, email, CSV export — neutralize formula
  prefixes `=`, `+`, `-`, `@` for spreadsheet exports).
- Enforce length and charset limits at ingestion; store the raw payload
  separately from normalized fields.
- If an LLM processes lead content: treat prompts as an injection surface;
  separate instructions from data, restrict tool access, validate model output
  against a schema, never let model output decide authorization, and log
  prompts/responses per policy (see OWASP guidance for LLM applications).

### 6.3 File uploads

Allow-list extensions and sniffed MIME types, enforce size limits, generate
random storage names, store outside any executable or public path, scan for
malware, strip active content, serve with `Content-Disposition` and a
non-executing content type from a separate origin, and use pre-signed,
short-lived, scoped URLs.

## 7. SSRF and outbound traffic

Applies to webhooks, scraping targets, "fetch this URL" features, image and
avatar fetchers, provider callbacks and redirects.

1. **Prefer allow-lists** of destination hosts (Apify API, payment provider,
   telephony, LLM, object storage). For tenant-supplied URLs (webhooks), use a
   strict validator plus the network controls below.
2. **Validate after DNS resolution**: resolve the hostname, reject private,
   loopback, link-local, multicast, carrier-grade NAT and metadata ranges
   (IPv4 and IPv6, including IPv4-mapped IPv6 and `169.254.169.254`), then
   **connect to the validated IP** (pin it) to defeat DNS rebinding.
3. Only `https` (and `http` if explicitly justified); no `file`, `gopher`,
   `ftp`, `data` or custom schemes; reject credentials in URLs and unusual
   ports.
4. **Redirects**: disabled by default; if needed, cap the count and
   re-validate every hop.
5. Timeouts (connect, TLS, total), response size cap, content-type check,
   limited concurrency per tenant.
6. **Network-level enforcement**: workloads that fetch tenant URLs run in a
   segment with an **egress proxy/firewall** that blocks internal ranges and
   the cloud metadata service; require IMDSv2 or disable instance metadata for
   workloads that do not need it.
7. Never reflect raw upstream errors to the caller.
8. Tests: the `security` suites include SSRF cases (private ranges, rebinding,
   redirect-to-internal, IPv6 forms, alternative IP encodings).

## 8. Cryptography and data protection (A04)

- **TLS everywhere**: 1.2 minimum, 1.3 preferred, modern cipher suites,
  HSTS with preload for public domains, certificate automation and expiry
  monitoring. Internal traffic encrypted too (mTLS preferred).
- **Never implement your own crypto.** Use vetted libraries and high-level
  APIs. Prefer AEAD (AES-256-GCM or ChaCha20-Poly1305/XChaCha20-Poly1305);
  never reuse a nonce with the same key; use a CSPRNG for every random value
  that matters (tokens, IDs used for secrecy, salts, nonces).
- **Constant-time comparison** for secrets, MACs, tokens and hashes.
- **Encryption at rest**: disk/volume and managed-database encryption for
  everything, plus **application-level envelope encryption** for Restricted
  fields (tenant provider API keys, refresh tokens at rest if stored, highly
  sensitive personal data). Data-encryption keys wrapped by KMS keys; consider
  per-tenant data keys for high-assurance tenants and cryptographic erasure.
- **Key management**: keys in KMS/HSM, separate keys per environment and
  purpose, rotation schedule, key-use audit logs, no key material in
  application memory longer than needed, break-glass procedure.
- **Tenant provider keys** are write-only through the API and UI, decrypted
  only inside the outbound adapter that calls the provider, never logged,
  never returned, never placed in job payloads (pass an ID, resolve at use).
- **Data minimization and retention**: collect only what the feature needs;
  enforce retention with scheduled jobs that cascade to the vector index,
  caches and exports; backups follow the same retention.
- **Masking** in logs, analytics, error reports and non-production data.
- **Backups**: encrypted, access-restricted, immutable copies, restore tested
  (`PLAYBOOK_TESTS.md` DR suite).
- Embeddings and vector payloads are personal data when derived from personal
  data: same tenant scoping, encryption and deletion rules.

## 9. API and application hardening

### 9.1 Rate limiting and resource consumption

- Layered limits: edge (per IP), gateway (per route), application (per user,
  **per tenant**, per API key, per sensitive business flow).
- Token-bucket counters in Dragonfly/Redis (key convention in
  `PLAYBOOK_BACKEND.md`), with fail-safe behavior defined when the store is
  down (fail closed for sensitive flows, degraded local limits for others).
- Hard maximums: page size, request body, upload size, batch size, query
  complexity, concurrent jobs per tenant, run duration, export row count.
- Timeouts on every handler, query (`statement_timeout`) and outbound call;
  cancellation propagates.
- Expensive operations return `202` and run as batch jobs with per-tenant
  quotas (cost caps for provider usage).

### 9.2 CORS, CSRF and headers

- CORS: explicit origin allow-list (never `*` together with credentials), only
  needed methods/headers, short preflight cache.
- Response headers on every response: `Strict-Transport-Security`,
  `X-Content-Type-Options: nosniff`, `Referrer-Policy`, `Permissions-Policy`,
  `Cache-Control: no-store` on authenticated/sensitive responses,
  `Content-Security-Policy` with `frame-ancestors` for anything that serves
  HTML; remove `Server`/framework version banners.
- API endpoints that use cookies require CSRF defenses; bearer-token APIs are
  not CSRF-prone but still validate `Origin` on browser clients.

### 9.3 Webhooks and callbacks (inbound)

- Verify a **signature** (HMAC-SHA256 or provider-specified scheme) over the
  raw body with a per-endpoint secret, compare in constant time, require a
  **timestamp** within a short window (for example 5 minutes), and store
  event IDs to reject **replays**.
- Respond fast, process asynchronously and idempotently; treat the payload as
  untrusted input (validate schema, verify IDs belong to the tenant).
- Restrict by provider IP ranges only as an additional layer, never as the
  only control.
- Outbound webhooks to tenants are signed the same way and sent through the
  SSRF-safe client.

### 9.4 Errors and exceptional conditions (A10)

- **Fail closed**: on any error in authentication, authorization, validation
  or crypto, deny the request.
- A global handler converts all unexpected errors to the standard envelope
  with a `correlationId`; no stack traces, SQL, file paths, versions or
  internal hostnames leave the process.
- No empty catch blocks, no swallowed errors, no "default to allow".
- Consistent behavior under failure of dependencies: circuit breakers,
  bounded retries with jitter, bulkheads, defined degraded modes; **a failing
  security dependency (token introspection, policy cache, rate limiter) must
  not silently open access**.
- Resource cleanup on all paths (connections, locks, temp files, leases);
  transactions rolled back on error.
- Crashes: supervisor restarts the process, alerts on crash loops, no secrets
  in core dumps (disable dumps in production or encrypt).

### 9.5 Inventory and exposure

- Maintain an inventory of **all** routes, versions and environments (from the
  OpenAPI spec and route table); deprecated versions have sunset dates and
  are removed.
- No debug, profiling, metrics, health-detail or admin endpoints on public
  listeners; internal ports bound to private interfaces.
- Non-production environments are not internet-open by default and never
  contain production data.

## 10. Batch, queue and event security

- **Authenticate and authorize broker access** per service: Redis/Dragonfly
  ACL users per slice limited to their key prefix (`heimdall:{env}:{slice}:*`)
  and to the commands they need, where the engine supports key-pattern ACLs;
  TLS on connections; never exposed to the internet; dangerous commands
  (`FLUSHALL`, `FLUSHDB`, `CONFIG`, `DEBUG`, `KEYS`, scripting if unused)
  disabled or ACL-denied.
- **Messages are untrusted input**: validate the schema and version, verify
  that the `tenantId` is real and consistent with the referenced entity, and
  re-authorize as a system actor. Do not assume that "internal" means safe.
- **Integrity**: for cross-service or cross-trust-zone events, sign messages
  (HMAC or asymmetric signature) or rely on authenticated broker identities;
  protect the outbox table with the same least-privilege rules as business
  tables.
- **No secrets or personal data in payloads**: pass IDs and resolve inside the
  authorized handler; payloads in DLQs are therefore less sensitive but still
  access-controlled and retained per policy.
- **Abuse controls**: per-tenant enqueue quotas and concurrency caps,
  maximum payload size, deduplication, backpressure; a flooding tenant cannot
  starve others.
- **Lock and lease keys** use unpredictable tokens and fencing so a forged or
  stale holder cannot overwrite state (`PLAYBOOK_BACKEND.md` section 5).
- **Replay**: idempotency keys and unique constraints make replays harmless;
  security-relevant jobs also check freshness.
- DLQ and replay tools are admin-only, audited and cannot change the
  `tenantId`.

## 11. Supply chain (A03, A08)

### 11.1 Dependencies (all languages)

- **Lockfiles are mandatory and committed**; CI installs strictly from them
  (`npm ci`, `pip install --require-hashes`, `go` with `-mod=readonly`,
  `cargo --locked`). A change to a lockfile is reviewed like code.
- **Pin versions** (exact or lock-resolved); upgrade through automated PRs
  that run the full pipeline.
- **Cooldown for new releases**: do not adopt a version published in the last
  7–14 days unless it is an urgent security fix reviewed by hand; malicious
  versions are usually detected within days.
- **Disable install-time code execution by default**: npm `--ignore-scripts`
  (or pnpm's allow-list of build scripts), review `build.rs`/proc-macros in
  Rust, avoid `setup.py` execution in Python by using wheels from trusted
  indexes.
- **Single trusted registry path**: an internal proxy/mirror (Artifactory,
  Nexus, Verdaccio, devpi, Athens/GOPROXY, crates mirror) with allow-listed
  upstreams; private package names are reserved on public registries to
  prevent **dependency confusion**; scoped/namespaced packages.
- **Minimize dependencies**; justify each new one (maintainers, activity,
  license, size, install scripts, transitive count); prefer the standard
  library for small tasks.
- **Typosquatting and impersonation checks** on every new dependency; verify
  publisher and provenance (npm provenance, Sigstore attestations, PyPI
  trusted publishing, crates.io ownership).
- **Continuous SCA** on PRs and nightly with the vulnerability SLAs from
  `PLAYBOOK_AUDIT.md`; track end-of-life runtimes and frameworks.
- **License policy** enforced by tooling.

### 11.2 Build and release integrity

- CI hardened: pin third-party actions/plugins **by commit SHA**, minimal
  token permissions, ephemeral runners, no secrets to untrusted forks/PRs,
  protected branches, required reviews, signed commits/tags for releases.
- **Reproducible, hermetic builds** where feasible; build in a clean
  environment; no network at build time beyond the proxy.
- Produce an **SBOM** (CycloneDX/SPDX), **build provenance** (SLSA) and
  **sign artifacts and images** (for example Sigstore/cosign); deployment
  admission verifies signatures and digests; deploy by **digest**, not mutable
  tags.
- Base images: minimal (distroless or slim), pinned by digest, rebuilt
  regularly; run as non-root.
- Secrets are never baked into images or build args.

### 11.3 Developer environment

Managed devices with disk encryption and EDR, hardware-key MFA on the code
host and registries, least-privilege publishing tokens, and no long-lived
registry credentials on laptops.

## 12. Infrastructure, network and platform (A02)

### 12.1 Network

- **Default-deny segmentation**: public edge, application subnet, data
  subnet. Databases, Dragonfly/Redis, vector stores and queues have **no
  public endpoints**.
- Service-to-service only through allowed paths (security groups / network
  policies); east-west mTLS; egress restricted to the proxy and allow-listed
  destinations.
- Administrative access without open inbound ports (identity-aware access,
  session recording); no shared bastion passwords; break-glass is monitored.
- WAF and DDoS protection at the edge; managed DNS with DNSSEC where
  appropriate; CAA records.

### 12.2 Containers and Kubernetes

Run as non-root with a read-only root filesystem, drop all capabilities,
`allowPrivilegeEscalation: false`, seccomp `RuntimeDefault`, no host
namespaces or privileged containers, resource requests/limits, restricted Pod
Security level, network policies default-deny, secrets mounted from the
secrets manager (not baked into manifests), admission policies (signature,
registry, no `latest`), and regular node/OS patching.

### 12.3 Cloud and IaC

Least-privilege IAM per workload, no long-lived access keys, MFA on human
accounts, separate accounts/projects per environment, private storage by
default, KMS key policies, audit logging on, drift detection, **everything in
IaC** scanned in CI, and guardrails/SCPs that prevent disabling logging or
making data stores public.

### 12.4 Datastores

- **PostgreSQL**: separate roles for migration (DDL), application
  (DML on the slice schema only) and read-only reporting; no superuser for
  the app; `sslmode=verify-full`; row-level security; `statement_timeout`,
  `idle_in_transaction_session_timeout`; extensions allow-listed; audit
  logging for privileged actions.
- **Dragonfly/Redis**: `requirepass`/ACL, TLS, private network only, ACLs per
  slice, memory limits and eviction policy chosen so locks and queues are not
  evicted, persistence configured per criticality, no data of one tenant
  retrievable by key patterns shared across tenants.
- **Vector database**: authenticated, tenant filter enforced in the adapter,
  private network, encrypted, same retention/deletion as relational data.
- **Object storage**: private, per-tenant prefixes, pre-signed URLs with short
  expiry, versioning and object lock for evidence buckets.

### 12.5 Configuration hygiene

Secure defaults in code; configuration validated at startup (fail fast on
unsafe values such as debug mode, wildcard CORS, disabled TLS verification);
no default credentials; feature flags cannot disable security controls;
configuration drift detected.

## 13. Secrets management

- Single source: a secrets manager (Vault, cloud secret manager); apps fetch
  at startup or on demand with workload identity.
- **No secrets** in the repository, `.env` files committed, container images,
  CI logs, client bundles, the Chrome extension, tickets or chat.
- Prefer **short-lived dynamic credentials** (database creds, cloud tokens);
  otherwise rotate on schedule and on any suspicion; support **zero-downtime
  rotation** (two active versions).
- Environment variables are acceptable for non-sensitive config; for secrets
  prefer files mounted in memory-backed volumes or API retrieval, because
  environment variables leak into process listings, crash reports and
  child processes.
- Separate secrets per environment and per service; least-privilege read
  policies; every read audited.
- **Secret scanning** at pre-commit, in CI and across history; a verified
  leaked secret is rotated immediately, not just removed.
- Memory hygiene: avoid logging or serializing secret-bearing objects; clear
  buffers where the language allows (section 16).

## 14. Logging, monitoring, detection and response (A09)

### 14.1 What to log

Structured security events with `timestamp` (system clock), `correlationId`,
`tenantId`, `userId` or system actor, `ip`/`userAgent` (per privacy policy),
`action`, `resource`, `outcome`, `reason`:

- Authentication: success, failure, MFA events, lockouts, token refresh reuse.
- Authorization: every denial with the required permission.
- Privileged actions: role/permission changes, user suspension, impersonation,
  API key creation/rotation/revocation, exports, configuration changes,
  retention/deletion runs, DLQ replays.
- Tenant-boundary anomalies, SSRF blocks, WAF blocks, rate-limit hits,
  webhook signature failures, validation floods.
- Never log secrets, tokens, full card data, raw credentials or unnecessary
  personal data; neutralize newline/control characters to prevent log
  injection.

### 14.2 Platform

- Centralized, append-only/tamper-evident log pipeline separate from the
  application's own credentials; retention per policy; time-synchronized
  hosts (NTP).
- **Detection rules / alerts**: brute force and credential stuffing, sudden
  403/404 patterns across many tenant or object IDs (enumeration), new
  admin or privileged role, key creation outside change windows, mass export,
  access from new geographies for privileged users, DLQ or outbox growth,
  unusual egress, spikes in provider spend, repeated webhook signature
  failures, tenant-isolation anomalies.
- Honeytokens/canary credentials and records to detect unauthorized access.
- On-call with runbooks; alerts have owners and severity.

### 14.3 Incident readiness

Runbooks (in `compliance/procedures`, referenced by `PLAYBOOK_AUDIT.md`) for:
compromised user, leaked API/provider key, cross-tenant exposure, dependency
or CI compromise, ransomware/data destruction, DDoS, insider misuse, extension
store compromise.

Prebuilt **containment switches** (tested): global token revocation and key
rotation, per-tenant kill switch, per-feature kill switch (for example disable
campaign launch), route-level blocks at the edge, queue pause, read-only mode,
credential rotation automation, network isolation of a workload. Preserve
evidence before destroying anything; follow legal notification clocks.

### 14.4 Vulnerability disclosure

Public policy and `security.txt`, a monitored security mailbox, triage and
response SLAs, safe-harbor language, and CVE handling when appropriate.

## 15. Secure development lifecycle

| Stage | Required |
|---|---|
| Design | Threat model, security requirements (ASVS level), architecture review for new slices and integrations |
| Code | Secure-coding standard (this file), language profile rules, pre-commit secret scan and linters, mandatory review with the security checklist below |
| Build | SAST, SCA, secrets, IaC and container scans as PR gates; architecture tests (`PLAYBOOK_TESTS.md`) |
| Test | Authz matrix, tenant isolation, SSRF, injection, fuzzing of parsers and message handlers, abuse-case tests, DAST on staging |
| Release | Signed artifacts, SBOM, provenance, change record, rollback plan |
| Operate | Monitoring, vulnerability SLAs, patching, key rotation, access reviews, backups, game days |
| Retire | Credential revocation, data deletion, DNS/route removal, dependency removal |

Additional practices: security training at onboarding and annually, role-based
modules for engineers, **fuzzing** for parsers, importers, and event/webhook
handlers (language-native fuzzers), periodic **independent penetration test**
(annual and on major change), optional bug bounty after the baseline is
stable, and tracking of runtime/framework support dates (upgrade before end of
life).

### 15.1 Security review checklist for every PR touching a boundary

- [ ] New/changed route has a declared permission and an `authz-matrix` row.
- [ ] Tenant and ownership are in every query predicate.
- [ ] Input validated by schema with unknown fields rejected; limits set.
- [ ] No string-built queries, shell calls, unsafe deserialization or path
      joins with user input.
- [ ] Outbound calls use the SSRF-safe client with timeouts and size caps.
- [ ] Errors are caught centrally, fail closed, and leak nothing.
- [ ] No secrets, tokens or personal data in logs, payloads or responses.
- [ ] New dependency justified, pinned, scanned and past the cooldown.
- [ ] Security events emitted for privileged or sensitive actions.
- [ ] Tests added (positive and abuse cases); threat model updated if needed.

## 16. Language profiles

The core rules apply identically; these profiles list the traps and the
native tooling. Tool names are examples; choose equivalents your team
maintains.

### 16.1 Cross-language equivalence table

| Control | Node.js | Python | Go | Rust |
|---|---|---|---|---|
| Schema validation (reject unknown) | zod / Ajv strict / TypeBox | Pydantic v2 (`extra="forbid"`) | struct decoding + `DisallowUnknownFields` + validator lib | `serde` with `deny_unknown_fields` + validator crates |
| Parameterized queries | Prisma, Drizzle, Knex bindings | SQLAlchemy bind params, Django ORM | `database/sql` placeholders, sqlc | SQLx (compile-time checked), Diesel |
| Password hashing (Argon2id) | `argon2` | `argon2-cffi` | `golang.org/x/crypto/argon2` | `argon2` (RustCrypto) |
| CSPRNG | `crypto.randomBytes`, `crypto.randomUUID` | `secrets` | `crypto/rand` | `rand::rngs::OsRng` / `getrandom` |
| Constant-time compare | `crypto.timingSafeEqual` | `hmac.compare_digest` | `crypto/subtle.ConstantTimeCompare` | `subtle` crate |
| HTTP server timeouts | `server.headersTimeout`, `requestTimeout`, `keepAliveTimeout` | ASGI/WSGI server and proxy timeouts | `http.Server{ReadHeaderTimeout, ReadTimeout, WriteTimeout, IdleTimeout}` | `tokio::time::timeout`, tower `TimeoutLayer` |
| Body-size limit | framework `limit` option | server/framework limit | `http.MaxBytesReader` | tower-http `RequestBodyLimit` / axum `DefaultBodyLimit` |
| Outbound client with timeout + SSRF guard | `undici` with custom `connect`/lookup guard | `httpx`/`requests` with explicit timeouts + resolver guard | `http.Client{Timeout}` + `Dialer.Control` guard | `reqwest` with timeouts + custom resolver/connector |
| Rate limiting | `rate-limiter-flexible` (Redis store) | `slowapi`, `limits` (Redis) | `x/time/rate`, Redis-backed limiters | `tower-governor`, Redis-backed limiters |
| SAST | Semgrep, CodeQL, `eslint-plugin-security` | Semgrep, CodeQL, Bandit, Ruff `S` rules | Semgrep, CodeQL, gosec, staticcheck | Semgrep, CodeQL, Clippy |
| SCA | `npm audit`, OSV-Scanner, Socket-style analyzers | `pip-audit`, OSV-Scanner, Safety | `govulncheck`, OSV-Scanner | `cargo-audit`, `cargo-deny`, `cargo-vet` |
| Secret wiping | Best effort (GC; use short-lived values, avoid logging) | Best effort (immutable `str`; use `bytearray` where possible) | Best effort (overwrite `[]byte`) | `zeroize` + `secrecy` crates |
| Race / memory checks in CI | — | — | `go test -race` | Miri on `unsafe`, sanitizers, `cargo-geiger` |

### 16.2 Node.js

- Use an **active LTS** release and track its end-of-life; update promptly for
  security releases.
- **TypeScript `strict`**; no `any` at boundaries; validate with a runtime
  schema because types vanish at runtime.
- **Prototype pollution**: strip/reject `__proto__`, `constructor`,
  `prototype` keys; use schema validation that drops unknown keys; use
  `Map`/`Object.create(null)` for dictionaries keyed by user input; consider
  `--disable-proto=delete`; avoid unsafe deep-merge helpers.
- **No `eval`, `new Function`, or `vm` as a sandbox** (`vm` is not a security
  boundary).
- **Subprocesses**: `execFile`/`spawn` with an argument array and
  `shell: false`; never `exec` with interpolated strings.
- **Event-loop safety**: DoS by blocking the loop. Offload CPU-heavy work to
  worker threads or batch jobs; cap JSON size and depth; avoid synchronous
  crypto/fs on request paths; beware catastrophic regexes (use RE2 bindings
  for user-controlled patterns).
- **Unhandled rejections and exceptions**: handle centrally; on
  `uncaughtException`/`unhandledRejection` log, fail the request, and let the
  supervisor restart the process rather than continuing in an unknown state.
- **Framework hardening**: `helmet`-style headers, trusted-proxy
  configuration so `req.ip` and protocol are correct only behind your proxy,
  strict CORS config, cookie flags, disable `x-powered-by`.
- **Path traversal**: `path.resolve` + prefix check against the allowed root.
- **Randomness**: never `Math.random()` for anything security-related.
- **Supply chain**: `npm ci`, `--ignore-scripts`, `npm audit signatures`,
  provenance checks, lockfile linting, scoped private packages, pnpm's
  default blocking of dependency build scripts with an explicit allow-list,
  and Node's permission model where it is stable for your LTS (restrict
  filesystem, child-process and network access).
- **ORM raw-query escape hatches** (`$queryRawUnsafe` and equivalents) are
  forbidden by lint rule.

### 16.3 Python

- Supported CPython version; virtual environments; dependencies locked with
  hashes (`uv`/`pip-tools` + `--require-hashes`); one configured index to
  avoid dependency confusion.
- **Never**: `pickle`/`marshal`/`shelve` on untrusted data, `yaml.load`
  (use `safe_load`), `eval`/`exec` on input, `subprocess(..., shell=True)`
  with interpolation, `assert` for security checks (stripped under `-O`).
- **XML**: `defusedxml`. **Archives**: guard against path traversal
  ("zip slip"); use `tarfile` extraction filters (`filter="data"`) and
  size/ratio limits.
- **Templates**: Jinja2 autoescape on; user-authored templates only in a
  sandboxed environment with no access to objects that expose secrets.
- **HTTP clients**: `requests` has no default timeout, so always set one;
  keep TLS verification on; bound response size; use the SSRF resolver guard.
- **Frameworks**: Django (`DEBUG=False`, `ALLOWED_HOSTS`, CSRF, secure
  cookies, `SECURE_*` settings, no `mark_safe` on untrusted data), FastAPI
  (Pydantic models for every body, dependency-injected `RequestContext`,
  explicit response models to prevent over-exposure), SQLAlchemy `text()`
  only with bound parameters.
- **Async/GIL**: do not block the event loop with CPU or sync I/O; use
  executors/batch jobs; cap concurrency.
- **Tools**: Bandit and Ruff `S` rules, mypy/pyright strict, `pip-audit`.
- Randomness via `secrets`, never `random`; compare secrets with
  `hmac.compare_digest`.
- Logging: structured; never `logging` raw request bodies; avoid logging
  exceptions that embed secrets (connection strings).

### 16.4 Go

- **Set all `http.Server` timeouts** (`ReadHeaderTimeout` to stop slowloris,
  `ReadTimeout`, `WriteTimeout`, `IdleTimeout`), limit headers, wrap bodies
  with `http.MaxBytesReader`, propagate `context` cancellation everywhere.
- **`html/template`** for HTML (never `text/template` with user data);
  `database/sql` placeholders or `sqlc`; `os/exec` with discrete arguments,
  never via `sh -c`.
- **`encoding/json`** with `DisallowUnknownFields` for strict inputs;
  validate after decoding.
- **Filesystem**: `filepath.Clean` is not enough; use rooted access (`os.Root`
  in recent Go versions) for user-influenced paths.
- **Outbound HTTP**: custom `http.Client` with `Timeout`, and a
  `net.Dialer.Control` hook that rejects internal addresses at connect time
  (this covers DNS rebinding and redirects).
- **Crypto**: `crypto/rand` (never `math/rand`) for secrets;
  `subtle.ConstantTimeCompare`; `x/crypto` for Argon2id; prefer the standard
  TLS config with minimum TLS 1.2.
- **Concurrency safety**: run `go test -race` in CI; avoid goroutine leaks
  (context + bounded worker pools); `recover` at request/job boundaries so a
  panic does not take down the process or leak details.
- **Integer conversions** and slice bounds: explicit checks on untrusted
  sizes (gosec rules catch common overflows).
- Avoid `unsafe` and cgo in application code; forbid via lint unless reviewed.
- **Do not expose** `pprof`, `expvar` or debug handlers on public listeners.
- **Supply chain**: `go.sum` committed, `-mod=readonly`, private
  `GOPROXY`/`GONOSUMDB`/`GOPRIVATE` configuration, `govulncheck` in CI,
  static builds (`CGO_ENABLED=0`, `-trimpath`) on distroless images.

### 16.5 Rust

- Memory safety is the baseline, not a complete defense: logic bugs, panics,
  integer issues, injection, SSRF, auth and supply-chain risks all remain.
- **`unsafe` policy**: `#![forbid(unsafe_code)]` in application crates;
  anywhere `unsafe` is unavoidable (FFI, performance) it is isolated in a
  small reviewed module with `// SAFETY:` justification and tests under Miri;
  track dependencies' `unsafe` with `cargo-geiger`.
- **Panics are availability bugs**: deny `unwrap`, `expect`, `panic!`,
  indexing and arithmetic panics in non-test code with Clippy lints; handle
  task failures at spawn boundaries so one bad request or job cannot crash the
  service; set a supervisor and alert on restarts.
- **Integer overflow**: release builds wrap silently by default; enable
  `overflow-checks = true` for services or use `checked_`/`saturating_`
  arithmetic on untrusted sizes.
- **Async safety**: timeouts on all I/O (`tokio::time::timeout`), bounded
  channels and queues, request body limits, concurrency limits, and care with
  cancellation (a dropped future may leave invariants half-done: design for
  cancellation safety, use transactions/guards).
- **Data handling**: `serde` with `deny_unknown_fields`; SQLx compile-time
  checked or parameterized queries; `secrecy` + `zeroize` for secrets;
  `subtle` for constant-time comparisons; `rustls` preferred; RustCrypto or
  `ring` for primitives.
- **Supply chain**: `Cargo.lock` committed, builds with `--locked`;
  `cargo-deny` (advisories, licenses, banned crates, allowed registries/git
  sources), `cargo-audit`, `cargo-vet` for reviewed dependency trust; review
  `build.rs` and procedural macros (they run arbitrary code at build time);
  `cargo auditable` to embed dependency data in binaries.
- Release builds: `panic` strategy chosen deliberately, symbols stripped in
  images, static or distroless runtime.

## 17. Security requirements by slice (Heimdall)

| Slice | Top threats | Required controls beyond the baseline |
|---|---|---|
| `identity` | Credential stuffing, token theft, session fixation, privilege escalation | Argon2id, mandatory MFA for admin, refresh rotation + reuse detection, per-account/IP throttling, ASVS L3, audit of all auth events |
| `tenants` | Cross-tenant access, API key theft, malicious membership changes | RLS + tenant predicate, envelope-encrypted write-only provider keys, key-use audit, step-up auth for key changes, approval flow for ownership transfer |
| `projects` | BOLA, mass assignment | Ownership checks, explicit DTOs, quotas |
| `extraction` | SSRF, malicious scraped content, provider key misuse, queue forgery, cost abuse | SSRF-safe client, content sanitization pipeline, per-tenant concurrency and spend caps, signed/authenticated jobs, lease + fencing, provider key resolved only in adapter |
| `funnels` | IDOR on leads, stored XSS, CSV injection, vector cross-tenant leakage | Tenant+owner predicates, output encoding, export neutralization, tenant-scoped vector queries (impossible to omit via the port), deletion cascades |
| `campaigns` | Abuse (spam/harassment/fraud), webhook forgery, unauthorized launch | `campaigns.launch` permission + step-up, rate/volume caps, opt-out/suppression enforcement, signed provider webhooks with replay protection, anomaly alerts |
| `usage` | Tampering with metering | Append-only metering events, server-side computation only |
| `billing` | Webhook forgery, price tampering, invoice IDOR | Signature verification, server-side pricing, tenant-scoped invoices, payment data never touches Heimdall (hosted fields/redirect) |
| `platform` (shared) | Misconfiguration, secret leakage, log exposure | Config validation at startup, secrets manager only, log redaction, hardened base images |

## 18. Starting baseline values

Tune with threat modeling and measurements.

| Item | Starting value |
|---|---|
| TLS | 1.2 minimum, 1.3 preferred, HSTS 1 year + `includeSubDomains` |
| Access token lifetime | 10 minutes |
| Refresh token | Rotating, 7–30 days idle/absolute per risk, reuse = revoke family |
| Admin session | Shorter idle timeout (for example 15 minutes) and MFA step-up for sensitive actions |
| Argon2id | ≥ 19 MiB, t = 2, p = 1 (tune upward) |
| Login throttle | Progressive delay after 5 failures per account/IP; challenge after 10 |
| Request body | 1 MiB default; larger only on specific upload routes |
| Page size | Default 50, maximum 200 |
| Outbound timeouts | Connect 3 s, total 10–30 s, response cap per endpoint |
| API rate limits | Per user 60/min, per tenant 600/min, sensitive flows lower; tune per route |
| Webhook timestamp tolerance | 5 minutes |
| DB statement timeout | 5–30 s per workload class |
| Vulnerability SLAs | Set in policy (for example critical days, high weeks) |
| Secret rotation | Dynamic/short-lived preferred; static secrets at least annually and on any exposure |

## 19. Verification map to `PLAYBOOK_TESTS.md`

| Control area | Test suite |
|---|---|
| Authorization, BOLA, BFLA, `{userType}` prefix not authorizing | `authz-matrix`, `<slice>/backend/security` |
| Tenant isolation (API, jobs, events, cache, vector) | `tenant-isolation`, `queue`, `vector` integration |
| Input validation and injection | `<slice>/backend/security`, fuzzing |
| SSRF and outbound controls | `<slice>/backend/security`, `external` contracts |
| Auth/session/token behavior | `identity/backend/security`, `http` integration |
| Webhook signature and replay | `contract/external`, `security` |
| Queue security and idempotency | `queue`, `concurrency` |
| Secrets and log hygiene | `_platform/security/secrets`, `_platform/observability` |
| Dependencies, containers, IaC | `_platform/security` (SCA, container, IaC, SBOM) |
| Resilience and fail-closed behavior | `_platform/resilience` |
| Privacy and retention | `_platform/privacy` |
| Audit trail of security events | `_platform/audit-log` |

Control IDs for audit evidence follow `PLAYBOOK_AUDIT.md` (domains `ACC`,
`CRY`, `SDL`, `VUL`, `LOG`, `INC`, `BCP`, `VND`).

## 20. Concrete example for Heimdall: Apify run-completion webhook

Feature: Apify calls `POST /api/v1/app/projects/{projectId}/runs/{runId}/complete`
(or a platform-level webhook route) when an extraction run finishes.

1. **Edge**: WAF + rate limit per source; TLS 1.2+; body ≤ 1 MiB.
2. **Authentication**: verify the HMAC signature over the raw body with the
   per-project webhook secret (stored encrypted, constant-time compare);
   reject if the timestamp is outside 5 minutes; reject duplicate `eventId`
   (replay) using a database unique constraint, Dragonfly only as a fast path.
3. **Validation**: schema with unknown fields rejected; `runId` must belong to
   `projectId`, and `projectId` to the tenant resolved from the **stored** run,
   never from the payload.
4. **Authorization**: the handler runs as a **system actor** with only
   `extraction.runs.update` for that tenant.
5. **Processing**: `202` immediately; enqueue a job carrying only IDs; the
   worker fetches results from Apify using the tenant key resolved inside the
   outbound adapter, through the SSRF-safe client with timeouts and size caps.
6. **Content handling**: scraped fields are length-limited, sanitized,
   stored as data, written idempotently (unique key on
   `tenantId + externalResultId`); embeddings indexed per tenant.
7. **Failure behavior**: invalid signature → `401` with the standard envelope
   and a security event; parsing errors fail closed; poison payloads go to the
   DLQ after max attempts with admin-only access.
8. **Observability**: events logged with `correlationId` and `tenantId`;
   alert on repeated signature failures; metrics on lag and DLQ size.
9. **Tests**: forged signature, stale timestamp, replay, mismatched
   `projectId`, cross-tenant `runId`, oversize body, malformed JSON, scraped
   payload with HTML/formula/prompt-injection strings, duplicate delivery
   (`PLAYBOOK_TESTS.md` suites).
10. **Same behavior in every language**: the steps are identical for the
    Node.js, Python, Go and Rust implementations; only the library calls
    differ (section 16.1).

## 21. Checklist

- [ ] Threat model exists for every slice, route group, queue and integration
      and is reviewed at least annually.
- [ ] Every route/use case declares a permission; deny by default; authz is in
      the use case; tenant and ownership are in every query.
- [ ] `{userType}` is never used for authorization; the `/admin`-with-`/app`-token
      case is tested.
- [ ] RLS enabled with an application role that cannot bypass it.
- [ ] Argon2id, MFA for privileged users, rotating refresh tokens with reuse
      detection, asymmetric short-lived access tokens validated per RFC 8725.
- [ ] Schema validation with unknown fields rejected; limits on size, depth
      and counts; parameterized queries; no shell; no unsafe deserialization.
- [ ] Scraped and third-party content is sanitized per sink; exports
      neutralize formulas.
- [ ] SSRF-safe outbound client with post-resolution IP checks, no or
      re-validated redirects, timeouts, size caps and network egress controls.
- [ ] TLS everywhere; envelope encryption for Restricted data; KMS-managed
      keys with rotation; provider keys write-only and decrypted only in the
      adapter.
- [ ] Rate limits and quotas per IP, user, tenant, key and sensitive flow;
      timeouts on every handler, query and outbound call.
- [ ] Webhooks verified by signature + timestamp + replay protection.
- [ ] Errors fail closed; no stack traces or internals leak; a failing
      security dependency never opens access.
- [ ] Broker (Dragonfly/Redis) with TLS, ACLs per slice prefix, no public
      exposure, dangerous commands restricted; messages treated as untrusted.
- [ ] Lockfiles, pinned versions, release cooldown, install scripts disabled
      by default, private registry proxy, SCA/SAST/secrets/IaC/container scans
      as gates; actions pinned by SHA; signed artifacts, SBOM, provenance.
- [ ] Containers non-root, read-only, minimal; network default-deny;
      datastores private; IaC scanned; least-privilege IAM; no long-lived keys.
- [ ] Secrets only in the secrets manager; scanning in pre-commit, CI and
      history; rotation tested.
- [ ] Security events logged without secrets or PII; tamper-evident
      centralized logs; detection rules and on-call runbooks exist.
- [ ] Containment switches (token revocation, tenant/feature kill switch,
      queue pause, key rotation) are implemented and rehearsed.
- [ ] Language profile rules for Node.js, Python, Go and Rust are enforced by
      linters, SAST and CI.
- [ ] Penetration test at least annually and after major change; findings
      tracked to closure; vulnerability disclosure policy published.
- [ ] Security tests mapped in `PLAYBOOK_TESTS.md` pass and produce evidence
      for the controls in `PLAYBOOK_AUDIT.md`.
