# Persistence & Migrations

Postgres via pgx/v5, typed queries via sqlc, schema changes via annotated goose migrations —
and a hard rule: **migrations never run at server startup**.

## 1. The pool, once, at the root

`cmd/api/main.go` builds `pgxpool` from env config and injects it into
adapters. Nothing else in the codebase names `pgxpool`. `readyz` runs
`SELECT 1` with the pool and answers 503 `degraded` on failure; `healthz` is
process-only.

```go
pool, err := pgxpool.New(ctx, cfg.DatabaseURL)
// adapters receive a narrow store interface, not *pgxpool.Pool
```

## 2. sqlc: per-slice queries

Sources sit in the slice (`internal/features/authsession/queries.sql`; schema
pointed at `migrations/` via `sqlc.yaml`); output is the committed, generated
`internal/gen/db` (package `db`). Hand-write SQL once, get typed methods —
the Go equivalent of the sibling's compile-time-checked queries:

```sql
-- internal/features/authsession/queries.sql
-- name: InsertSession :exec
INSERT INTO sessions (id, user_id, refresh_token, ip_address, user_agent, expires_at)
VALUES ($1, $2, $3, $4, $5, $6);
```

Rules: queries are per-slice (a slice's queries.sql names only its tables);
adapters in `postgres.go` are the only consumers of `gen/db`; `make generate`
must be a no-op after editing either side (`make gen-check` fails otherwise).
Never hand-edit `internal/gen/**`.

## 3. goose: annotated reversible migrations, tools-only

`migrations/` holds numbered files (`-- +goose Up` / `-- +goose Down` sections; goose v3.29 no longer accepts split `.up/.down` file pairs), applied with `go tool goose` (module tool
dependency — no global install):

```sh
make migrate-up                   # goose up
make migrate-add NAME=add_orders  # stamps <ts>_add_orders.sql (Up + Down sections)
```

Every `up` ships its `down`; the template ships `0001_init` (the whole core
schema) and `0002_profiles` (1:1 profile tables), new files get a timestamp
from `migrate-add`. Down sections are
exercised by the integration suite (apply → down → apply) so reversal stays
honest. `0001_init` creates the whole template schema; after instantiating,
delete the `widgets` table from *your* 0001 — not from a template file you no
longer own.

**Why never at startup**: schema changes belong to a deployment step with a
human or pipeline in the loop — two replicas racing a startup migration, or a
canary applying 0007 while prod runs 0006 code, are failure modes the rule
removes. Startup failing because the schema drifted (readyz degraded) is the
correct signal instead.

## 4. Adapter shape

The pgx adapter is dumb by design — call gen/db, map to domain types,
translate errors:

```go
func (s Store) ByID(ctx context.Context, id WidgetID) (Widget, error) {
    row, err := s.q.GetWidget(ctx, id)
    if errors.Is(err, db.ErrNotFound) { return Widget{}, ErrWidgetMissing }
    if err != nil { return Widget{}, fmt.Errorf("get widget: %w", err) }
    return toDomain(row), nil
}
```

Mapping lives in exactly one direction per adapter file; no SQL outside
the slice's `queries.sql`; no `pgx` types leaking past this file.

## 5. The template schema (parity core)

Tables shipped in `0001`: `users` (role/user_type enum checks, is_active /
is_blocked / is_verified, retry_count, pepper-hashed password, last_login,
language, google_id/github_id — the columns the OAuth flows link by),
`sessions` (opaque refresh token unique, ip/ua, expires_at) — one table, no
legacy duplicate, `email_verifications` (6-digit code, 24h),
`password_resets` (30m), `settings` (key/value + is_sensitive + updated_by),
`audit_logs` (actor, action, entity, payload JSONB, created_at indexed),
`email_jobs` (outbox: to_email, template_key, payload JSONB, status
queued/sending/sent/failed, retry_count, max_retries, scheduled_at), and the
example `widgets` table.

`0002_profiles` adds two 1:1 profile tables keyed by `user_id UUID PRIMARY KEY
REFERENCES users(id) ON DELETE CASCADE`: `companies` (company_name, website,
industry, tax/registration ids, address, phone) and `users_profile`
(full_name, bio, avatar_url, date_of_birth, address, occupation,
preferences JSONB DEFAULT '{}'). Both downs drop cleanly — the integration
suite's apply→down→apply pass exercises them.

Indexes: unique on emails/tokens/keys, FK cascades from users, and
`created_at` on audit logs + a partial index on `email_jobs (status,
scheduled_at) WHERE status = 'queued'` for the worker claim.

## 6. Transactions across ports

A use case that must write two tables atomically gets a port that says so —
not a global transaction middleware. The narrow shape: the port method takes
the unit of work; the adapter wraps it in one pgx `Begin`.

**Shipped state:** the outbox `Enqueue` still runs on the pool, not on a
caller-supplied `pgx.Tx`. The auth write and the `email_jobs` insert are
therefore not atomic; treat this section as the target shape, not a property
of the current signup/reset paths.

```go
type SessionStorer interface {
    RotateTx(ctx context.Context, old, new Session) error // one tx inside
}
```

Cross-slice transactions are a boundary smell: promote the operation (its
queries live where the tables are owned) rather than opening multi-slice
transactions.

## See also

- [Playbooks](12-playbooks.md) — migrate-add step by step
- [Dependency Rules](04-dependency-rules.md) — the SQL-ownership invariants
- [Auth Architecture](06-auth-architecture.md) — what each table serves