# Errors, Config & Observability

One error envelope, one typed-config path, one logging primitive — and a
deliberate cut: no OpenTelemetry in the template.

## 1. The error envelope

Every non-2xx body is `{"detail": string, "code": STABLE_CODE}` — stable
codes are the `Code` enum in `app/shared/apperr.py`, domain errors carry
them, and the handlers installed by `install_error_handlers(app)` map them
to HTTP statuses in exactly one place:

```python
# app/shared/apperr.py
class Code(StrEnum):
    VALIDATION_FAILED = "VALIDATION_FAILED"
    UNAUTHENTICATED = "UNAUTHENTICATED"
    FORBIDDEN = "FORBIDDEN"
    NOT_FOUND = "NOT_FOUND"
    CONFLICT = "CONFLICT"
    RATE_LIMITED = "RATE_LIMITED"
    INTERNAL = "INTERNAL"
    AUTH_EMAIL_TAKEN = "AUTH_EMAIL_TAKEN"
    # … plus the rest of the AUTH_* family and WIDGET_NOT_FOUND
```

```python
# app/platform/envelope.py — the only writer of the envelope
def error_body(code: Code | str, detail: str) -> dict[str, str]:
    return {"detail": detail, "code": code.value if isinstance(code, Code) else code}
```

Rules: adapters never invent codes; use cases raise `AppError` through the
`E(code, detail, status)` helper, and the envelope is written only by the
handlers in `app/platform/envelope.py` — grep for `"detail"` and you find
one definition, one writer (the `AppError`, `RequestValidationError` and
`StarletteHTTPException` handlers; unmatched paths answer the same 404
`NOT_FOUND` envelope). Client code keys on the _code_, not the message.

## 2. Error handling posture

- `AppError` is the single error shape crossing boundaries (`code`,
  `detail`, `status`); it is a dataclass exception declared with
  `eq=False`, so it compares by identity.
- Translate, don't string-match: adapters map _mechanism_ errors to domain
  errors (SQLAlchemy's `IntegrityError` → `AUTH_EMAIL_TAKEN` in
  `authsignup/postgres.py`); use cases match on `code`, never on messages.
- The exception handlers are the recovery point: a pydantic validation
  failure becomes 422 `VALIDATION_FAILED` with a fixed detail string (the
  field-level detail stays server-side), an `HTTPException` keeps its status
  (mapped to `NOT_FOUND` for 404, `INTERNAL` otherwise), and the catch-all
  `Exception` handler logs the trace once and answers 500
  `{"detail": "internal server error", "code": "INTERNAL"}`. Nothing but the
  envelope reaches the client — a failure nobody modelled degrades to a
  generic 500 instead of leaking a traceback.
- No error strings in conditionals; no `raise ValueError("user message")`
  past a slice boundary — that is what the stable codes are for.

## 3. Config: pydantic-settings into one object

`app/platform/config.py` owns a single `Settings` (a `BaseSettings` subclass)
loaded once at each root (`bin/api.py`, `bin/worker.py` read it the same
way), parsed from the `<APP>_`-prefixed environment:

```python
model_config = SettingsConfigDict(
    env_prefix="<APP>_",
    env_file=".env",
    env_file_encoding="utf-8",
    extra="ignore",
)
```

Unknown variables are ignored; list-valued settings (`CORS_ORIGINS`,
`ALLOWED_ADMIN_EMAILS`) are JSON arrays in the environment. A
`model_validator` fails the boot closed:

- `database_url` must use the `postgresql+asyncpg://` driver — a sync DSN
  is a configuration error, not a runtime one.
- `jwt_secret` must be at least 32 characters with at least 8 distinct
  characters, and must not be one of the known placeholders.
- `offline` forbids SMTP credentials entirely (INV-T7).

`.env.example` documents every key with placeholder values only, and the
`.env` file itself is git-ignored. Validation happens at boot, so a
mistyped key fails in milliseconds, not at first use.

## 4. Observability: stdlib JSON logging + health gates

`app/platform/logging.py` replaces the root logger's handlers with a single
`JsonFormatter` — one JSON object per line on stdout, with the fields
operators query (`ts`, `level`, `logger`, `msg`, `exc`, plus `request_id`
on request paths). No logging dependency, no third-party serializer.

The middleware mints or propagates an `x-request-id`, binds it to the
request's task through a `ContextVar` (`set_request_id`), and echoes it on
every response. A logging filter copies that task-local id onto every
record, so every log line emitted while serving the request carries the
same `request_id` the client sees in the response header. Log at the
boundary where context exists; throw errors rather than logging and
re-throwing.

Ship-time gates:

- `GET /healthz` — process up, no external calls (it also backs the
  container probe: the runtime image carries no `curl`, the `HEALTHCHECK`
  uses `urllib.request`).
- `GET /readyz` — `SELECT 1` plus a read of `alembic_version` through the
  pool; failure → 503 envelope. This is the deploy-signal surface: since
  migrations never run at startup, a schema drift shows up here, loudly.

## 5. The otel cut + `pip-audit`

OpenTelemetry is **deliberately cut** to keep the template's dependency
surface minimal (and to avoid the "tracing everywhere, sampled nowhere"
trap). The seams that make otel a clean _addition_ when you need it are
already there: the middleware chain is the single entry point for
instrumentation, request-scoped identity is real (`Identity` from the
dependency), and the platform layer owns the I/O wrappers (`db.py`,
`email/`, `oauthx.py`). Add the OpenTelemetry SDK and instrument the
middleware and the session factory — no slice code changes.

Supply-chain posture, the Python analog of the sibling's govulncheck:

```sh
uv run pip-audit                # actionable vulns only (make vuln)
uv run ruff check .             # static-analysis lane (make lint)
```

`make ci` runs the linters and the tests; the vulnerability check runs as a
separate CI job (network-heavy) rather than in every local gate.

## 6. Runtime debug affordances

- `/readyz` + request logs (with IDs) are the default incident toolkit:
  readyz tells you which dependency is dead; request logs tell you who.
- The outbox worker's failures are _rows_, not swallowed promises:
  `fail()` writes `last_error` + backoff into `email_jobs`; a dead-lettered
  job is visible in `email_jobs` (status `failed`) and in the logs, never
  silently dropped. The claim → send → retry/dead-letter progression is the
  worker's structured log line at each status change.
- WS close 4001 and SSE teardown are client-observable events — the socket
  itself is the signal; the error handlers above stay HTTP-only.

## Do / Don't

- **Do** raise wrapped `AppError`s up; log once, where context lives.
- **Do** keep codes stable — they are API contract (same tier as URLs).
- **Don't** log inside `domain.py`; the domain returns values and throws
  typed errors.
- **Don't** put secrets in logs; env parsing is where scrubbing happens,
  once.

## See also

- [Auth Architecture](06-auth-architecture.md) — the codes in action
- [Testing & CI](11-testing-and-ci.md) — the gate
- [Project Skeleton](13-project-skeleton.md) — `.env.example`
