# Errors, Config & Observability

One error envelope, one typed-config path, one logging primitive — and a
deliberate cut: no OpenTelemetry in the template.

## 1. The error envelope

Every non-2xx body is `{"detail": string, "code": STABLE_CODE}` — stable
codes are constants in `internal/shared/apperr`, domain errors carry them,
and `httpx` maps code → HTTP status in exactly one place:

```go
// internal/shared/apperr
const (
    AuthEmailNotVerified   = "AUTH_EMAIL_NOT_VERIFIED"  // → 403
    AuthInvalidCredentials = "AUTH_INVALID_CREDENTIALS" // → 401
    AuthRateLimit          = "AUTH_RATE_LIMITED"        // → 429
)
// httpx.WriteErr(w, apperr.ErrEmailNotVerified, "verify your email first")
// → 403 {"detail":"...","code":"AUTH_EMAIL_NOT_VERIFIED"}
```

Rules: adapters never invent codes; use cases return sentinel-wrapped errors;
the envelope is written only by `httpx` helpers — grep for `"detail":` and you
find one definition. Client code keys on the *code*, not the message.

## 2. Error handling posture

- Wrap with `%w` when context is added; never re-wrap a sentinel with `%s`.
- Match with `errors.Is` for sentinels and `errors.As` for typed causes
  (pgx constraint violations mapped to domain violations in `postgres.go`).
- `httpx.Recover` turns panics into a 500 envelope + slog error (stack logged
  once, at the recovery point).
- No error strings in switch statements; no `fmt.Errorf` without a verb.

## 3. Config: env/v11 into one struct

`internal/platform/config` owns a single typed struct loaded once at each
root (`cmd/api`, `cmd/worker` validate the same way):

```go
type Config struct {
    Bind    string `env:"<APP>_BIND" envDefault:":8080"`
    DBURL   string `env:"<APP>_DATABASE_URL,required"`
    Pepper  string `env:"<APP>_PASSWORD_PEPPER"`
    Offline bool   `env:"<APP>_OFFLINE" envDefault:"false"`
}
```

`.env.example` documents every key with placeholder values only (JWT_SECRET
placeholder-guard pattern — startup refuses known placeholder values). Secrets
never ship in files under version control. Validation happens at boot, so a
mistyped key fails in 200ms, not at first use.

## 4. Observability: slog + health gates

`slog` with the JSON handler is the only logging dependency; per-request logs
carry `request_id` (set by `httpx` middleware in both the handler chain and
the worker loop). Log at the boundary where context exists; return errors
rather than logging and re-returning.

Ship-time gates:

- `GET /healthz` — process up, no external calls.
- `GET /readyz` — `SELECT 1` via the pgx pool; failure → 503 `{degraded}`.
  This is the deploy-signal surface: since migrations never run at startup,
  a schema drift shows up here, loudly.

## 5. The otel cut + govulncheck

OpenTelemetry is **deliberately cut** to keep the template's dependency surface
minimal (and to avoid the "tracing everywhere, sampled nowhere" trap). The
seams that make otel a clean *addition* when you need it are already there:
context propagation is real (`ctx` first parameter everywhere) and the
platform layer owns I/O wrappers. Add `go.opentelemetry.io/otel` +
`otelhttp` middleware at the chain head and a pgx tracer hook — no slice code
changes.

Supply-chain posture, the Go analogue of the sibling's cargo-deny:

```sh
go tool govulncheck ./...            # actionable vulns only, call-graph aware
go tool golangci-lint run            # includes gosec-class rules
```

`make ci` runs the linter and tests; the vulnerability check runs as a
separate CI job (network-heavy) rather than in every local gate.

## 6. Runtime debug affordances

- `/readyz` + `slog` are the default incident toolkit: readyz tells you which
  dependency is dead; request logs (with IDs) tell you who.
- The outbox worker logs claim → send → retry/dead-letter as structured rows;
  a dead-lettered job is visible in `email_jobs` and in logs, not swallowed.
- WS/SSE lifecycles log connect/sub/close at Debug; 4001 closes at Info.

## Do / Don't

- **Do** return wrapped errors up; log once, where context lives.
- **Do** keep codes stable — they are API contract (same tier as URLs).
- **Don't** `slog` inside `domain.go`; the domain returns values and errors.
- **Don't** put secrets in logs; env parsing is where scrubbing happens, once.

## See also

- [Auth Architecture](06-auth-architecture.md) — the codes in action
- [Testing & CI](11-testing-and-ci.md) — the gate
- [Project Skeleton](13-project-skeleton.md) — `.env.example`