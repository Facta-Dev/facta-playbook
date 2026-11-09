# Persistence & Migrations

Postgres via SQLAlchemy 2 (async) + asyncpg, hand-written parameterized SQL
per slice, schema changes via paired Alembic revisions — and a hard rule:
**migrations never run at server startup**.

## 1. The engine, once, at the root

`src/app/bin/api.py` calls `Database.connect(settings)` from
`app/platform/db.py` and injects the database into the adapters. Nothing
else in the codebase imports `sqlalchemy` or `asyncpg` directly except the
slice adapters, and only through `queries.py`.

```python
engine = create_async_engine(settings.database_url, pool_pre_ping=True)
factory = async_sessionmaker(engine, expire_on_commit=False)
```

`Database` owns the engine and hands out short-lived sessions
(`database.session()`); the caller owns commit/rollback. `readyz` runs
`SELECT 1` **and** reads `alembic_version` (a schema that was never
migrated is not ready) and answers a 503 envelope on failure; `healthz` is
process-only.

## 2. Per-slice queries: hand-written `text()`

The sqlc/Kysely analog. Each slice owns a `queries.py` that declares its
tables' SQL as module-level `sqlalchemy.text()` constants with named
parameters:

```python
# src/app/features/widgets/queries.py
GET = text(
    """
    SELECT id, name, kind, owner_user_id, created_at
    FROM widgets
    WHERE id = :widget_id AND owner_user_id = :owner_id
    """
)
```

Rules: queries are per-slice (a slice's `queries.py` names only its tables);
adapters in `postgres.py` are the only consumers of the queries module;
column names are snake_case, exactly as in the migrations. There is **no
code generation** for queries in this template — the parameters are checked
by `mypy --strict` against the adapter's dict, not by a generated client;
that is the documented codegen delta from the sibling templates. The only
committed codegen artifact is the OpenAPI contract (`specs/openapi.yaml`),
regenerated with `make generate` and guarded by `make gen-check`. Never
hand-edit `specs/openapi.yaml`.

## 3. Alembic: paired reversible revisions, tools-only

`migrations/versions/` holds numbered revision files, each exporting **both**
`upgrade()` and `downgrade()`, applied with the `alembic` CLI already in the
dev dependencies (no global install):

```sh
make migrate-up                   # alembic upgrade head
make migrate-add NAME=add_orders  # alembic revision -m add_orders
```

`migrations/env.py` reads the connection from the environment
(`<APP>_DATABASE_URL`, defaulting to the local dev DSN) and drives an async
engine with a `NullPool` — it does **not** import `Settings`, so the
migration environment and the app environment cannot drift apart by
accident. `alembic.ini` sets `script_location = migrations` and
`prepend_sys_path = src`, and leaves `sqlalchemy.url` empty on purpose.

Every `upgrade` ships its `downgrade`; the template ships `0001_init` (the
whole core schema) and `0002_profiles` (the profile column and the
lookup indexes), new files get their revision id from `migrate-add`.
`0001_init` creates the whole template schema; after instantiating, delete
the `widgets` table from _your_ 0001 — not from a template file you no longer
own.

**Why never at startup**: schema changes belong to a deployment step with a
human or pipeline in the loop — two replicas racing a startup migration, or a
canary applying a new revision while prod runs the old code, are failure
modes the rule removes. The compose boot path makes it concrete: a one-shot
`migrations` service (`entrypoint: ["alembic", "upgrade", "head"]`) runs
first and the API waits on `service_completed_successfully`; `make migrate-up`
is the local-dev equivalent. Startup failing because the schema drifted
(`readyz` degraded) is the correct signal instead.

## 4. Adapter shape

The Postgres adapter is dumb by design — call the slice's queries, map to
domain types, translate errors:

```python
async def get(self, widget_id: str, owner_id: str) -> Widget | None:
    async with self._db.session() as session:
        result = await session.execute(queries.GET, {"widget_id": widget_id, "owner_id": owner_id})
        row = result.mappings().first()
    return None if row is None else _widget(row)
```

Mapping lives in exactly one direction per adapter file (`_widget`); no raw
SQL outside the slice's `queries.py`; no SQLAlchemy types leaking past this
file. Not-found is `None` flowing inward, thrown `AppError`s flow outward.
Constraint translations happen here too: the signup adapter catches
SQLAlchemy's `IntegrityError` and maps it onto the domain's
`AUTH_EMAIL_TAKEN` — one `except` clause, no dependency, and the use case
stays DB-agnostic.

## 5. The template schema (parity core)

Tables shipped in `0001`: `users` (unique `email`, role as TEXT with a
server default — not a native enum, so downgrades and typed queries stay
trivial — `is_active` / `is_blocked` / `is_verified`, `retry_count`,
argon2-hashed password, `last_login`, `language`, unique `google_id` /
`github_id` — the columns the OAuth flows link by), `sessions` (opaque
128-char refresh token, unique; ip/ua, `is_active`, `expires_at`) — one
table, no legacy duplicate, `email_verifications` (6-digit code, 24 h,
brute-force attempt counter), `password_resets` (30 min, the `email` column
keeps the rate limit honest even for unknown users), `email_jobs` (outbox:
`to_email`, `template_key`, payload JSONB, status
queued/sending/sent/failed, `retry_count`, `max_retries`, `last_error`,
`scheduled_at`), `audit_logs` (actor, action, entity, entity id, payload
JSONB, ip, user agent, `created_at`), and the example `widgets` table (kind
CHECK `sprocket/cog/flange`, owner FK to `users`).

`0002_profiles` adds `users.profile_picture` plus the lookup indexes
`sessions_expiry` and `email_jobs_status`. Both revisions downgrade
cleanly — drop the index, drop the column, drop the tables in reverse
dependency order.

Indexes: unique on emails/tokens/provider ids, FK cascades from `users`,
`created_at` on the owner-scoped tables, and `email_jobs_due (status,
scheduled_at)` for the worker claim.

## 6. Transactions across ports

A use case that must write two tables atomically gets a port that says so —
not a global transaction middleware. The **port method is the transaction
boundary**: the adapter opens one session, stages every write, and commits
once.

```python
# ports.py — the consumer asks for atomicity, not for a session handle
async def register(self, user, code, message, now) -> None:
    """Insert user + code + outbox job atomically."""


# postgres.py — the adapter owns the transaction
async with self._db.session() as session:
    try:
        await _write(session, user)
        await _write_code(session, code, now)
        await enqueue_email(session, message, now)  # stages on THIS session
        await session.commit()
    except IntegrityError as exc:
        await session.rollback()
        raise E(Code.AUTH_EMAIL_TAKEN, "email is already registered", 409) from exc
```

The outbox enqueue is part of that transaction: `enqueue_email(session,
message, now)` only _stages_ the `email_jobs` INSERT on the caller's session
and never commits, so a crash cannot separate the business write from the
email. Cross-slice transactions are a boundary smell: promote the operation
(its queries live where the tables are owned) rather than opening
multi-slice transactions.

## See also

- [Playbooks](12-playbooks.md) — migrate-add step by step
- [Dependency Rules](04-dependency-rules.md) — the SQL-ownership invariants
- [Auth Architecture](06-auth-architecture.md) — what each table serves
