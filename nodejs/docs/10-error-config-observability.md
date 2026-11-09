# Errors, Config & Observability

One error envelope, one typed-config path, one logging primitive — and a
deliberate cut: no OpenTelemetry in the template.

## 1. The error envelope

Every non-2xx body is `{"detail": string, "code": STABLE_CODE}` — stable codes
are constants in `#shared/apperr`, domain errors carry them, and `httpx` +
the root's error handler map them to HTTP statuses in exactly one place:

```ts
// src/shared/apperr.ts
export const AuthEmailNotVerified = "AUTH_EMAIL_NOT_VERIFIED"; // → 403
export const AuthInvalidCredentials = "AUTH_INVALID_CREDENTIALS"; // → 401
export const AuthRateLimit = "AUTH_RATE_LIMITED"; // → 429
// bin/api.ts — app.setErrorHandler writes the envelope once, for every route
// sendJSON(reply, ae.http, envelope(ae)) // → 403 {"detail":"...","code":"AUTH_EMAIL_NOT_VERIFIED"}
```

Rules: adapters never invent codes; use cases throw `AppError` (or map thrown
values through `from()`); the envelope is written only by the `httpx`
helpers — grep for `"detail"` and you find one definition
(`platform/httpx.ts`), one writer (`setErrorHandler` + `setNotFoundHandler`
at the root; unmatched paths answer the same 404 envelope). Client code keys
on the _code_, not the message.

## 2. Error handling posture

- `AppError` is the single error shape crossing boundaries (`http`, `code`,
  `detail`); everything else is coerced through `apperr.from()` at the edge.
- Translate, don't string-match: adapters map _mechanism_ errors to domain
  errors (pg's `23505` unique_violation → `AUTH_EMAIL_TAKEN` in
  `postgres.ts`); use cases match on `code`, never on messages.
- Fastify's error handler is the recovery point: a `5xx` answers the generic
  `INTERNAL_ERROR` envelope (no internals leak) and the stack is logged once,
  there. A `413` body becomes a 400 `Validation` — "invalid JSON body".
- No error strings in conditionals; no `throw new Error("user message")`
  past a slice boundary — that is what the stable codes are for.

## 3. Config: zod Load into one object

`platform/config` owns a single typed `Config` loaded once at each root
(`bin/api.ts`, `bin/worker.ts` validate the same way), zod-parsed from the
`<APP>_`-prefixed environment:

```ts
const raw = {
  databaseUrl: env("DATABASE_URL") ?? "",
  jwtSecret: env("JWT_SECRET") ?? "",
  pepper: env("PASSWORD_PEPPER") ?? "",
  offline: d("OFFLINE", "false"),
  // ...
};
// safeParse fails closed: unknown shapes, bad durations, weak secrets all
// throw at boot with the offending field's path.
```

`.env.example` documents every key with placeholder values only (JWT_SECRET
placeholder-guard pattern — the server _refuses_ known placeholder values and
secrets shorter than 32 bytes; `<APP>_OFFLINE=true` forbids SMTP credentials
entirely). Secrets never ship in files under version control. Validation
happens at boot, so a mistyped key fails in 200ms, not at first use.

## 4. Observability: fastify's JSON logger + health gates

The fastify logger (pino, JSON) is the only logging dependency; the
`requestID` hook mints or propagates an `x-request-id` and echoes it on every
response, and each request's log lines carry their request id (worker loops
use the entrypoint's `console.*` — bounded, structured). Log at the boundary
where context exists; throw errors rather than logging and re-throwing.

Ship-time gates:

- `GET /healthz` — process up, no external calls (it also backs the container
  probe: `api.js probe` GETs itself, no shell/curl in the runtime image).
- `GET /readyz` — `SELECT 1` via the pg pool with a 2s timeout; failure →
  503 `{degraded}` envelope. This is the deploy-signal surface: since
  migrations never run at startup, a schema drift shows up here, loudly.

## 5. The otel cut + pnpm audit

OpenTelemetry is **deliberately cut** to keep the template's dependency
surface minimal (and to avoid the "tracing everywhere, sampled nowhere"
trap). The seams that make otel a clean _addition_ when you need it are
already there: request-scoped context is real (`req.identity` + the request id
hook), fastify owns the chain head for instrumentation, and the platform layer
owns the I/O wrappers. Add the OpenTelemetry SDK + fastify instrumentation at
the root and a pg pool event hook — no slice code changes.

Supply-chain posture, the Node analogue of the sibling's govulncheck:

```sh
pnpm audit --audit-level=high        # actionable vulns only (make vuln)
pnpm lint                            # typescript-eslint, the static-analysis lane
```

`make ci` runs the linter and tests; the vulnerability check runs as a
separate CI job (network-heavy) rather than in every local gate.

## 6. Runtime debug affordances

- `/readyz` + request logs (with IDs) are the default incident toolkit: readyz
  tells you which dependency is dead; request logs tell you who.
- The outbox worker's failures are _rows_, not swallowed promises: `fail()`
  writes `last_error` + backoff into `email_jobs`; a dead-lettered job is
  visible in `email_jobs` (status `failed`) and in logs, never silently
  dropped. The claim → send → retry/dead-letter progression is the worker's
  structured log line at each status change.
- WS close 4001 and SSE teardown are client-observable events — the socket
  itself is the signal; the error handler above stays HTTP-only.

## Do / Don't

- **Do** throw wrapped `AppError`s up; log once, where context lives.
- **Do** keep codes stable — they are API contract (same tier as URLs).
- **Don't** log inside `domain.ts`; the domain returns values and throws
  typed errors.
- **Don't** put secrets in logs; env parsing is where scrubbing happens, once.

## See also

- [Auth Architecture](06-auth-architecture.md) — the codes in action
- [Testing & CI](11-testing-and-ci.md) — the gate
- [Project Skeleton](13-project-skeleton.md) — `.env.example`
