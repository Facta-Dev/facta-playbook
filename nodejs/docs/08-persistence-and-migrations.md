# Persistence & Migrations

Postgres via `pg`, typed queries via Kysely, schema changes via paired
node-pg-migrate migrations — and a hard rule: **migrations never run at server
startup**.

## 1. The pool, once, at the root

`src/bin/api.ts` calls `newPool(cfg)` from `platform/pg` and injects the pool
into adapters. Nothing else in the codebase imports `pg`. `readyz` runs
`SELECT 1` through the pool (2s timeout) and answers a 503 envelope on
failure; `healthz` is process-only.

```ts
const pool = await newPool(cfg);
// adapters receive a narrow store interface, not the pg.Pool
```

## 2. Kysely: per-slice queries

The sqlc analog. Each slice owns a `queries.ts` that declares its tables' row
types as a `Database` map and exports named, type-checked query functions:

```ts
// src/features/widgets/queries.ts
export interface WidgetsTable {
  id: string;
  name: string;
  kind: WidgetKind;
  owner_user_id: string;
  created_at: ColumnType<Date, string | undefined, never>;
}
export interface Database {
  widgets: WidgetsTable;
}

// widgetInsert :exec
export const widgetInsert = (db: Kysely<Database>, p: {...}): Promise<unknown> =>
  db.insertInto("widgets").values({ id: p.id, /* ... */ }).execute();
```

Rules: queries are per-slice (a slice's `queries.ts` names only its tables;
the typed `Database` map is the compile-time check sqlc gave the sibling
template — the documented codegen delta: hand-written typed builders instead
of a generated client); adapters in `postgres.ts` are the only consumers of
the queries module; column names are snake_case, exactly as in the
migrations. `make generate` regenerates the OpenAPI models (`src/gen/api`) —
the only codegen this template commits, guarded by `make gen-check`. Never
hand-edit `src/gen/**`.

## 3. node-pg-migrate: paired reversible migrations, tools-only

`migrations/` holds numbered `.mjs` files, each exporting **both** `up` and
`down` (one file, paired sections — node-pg-migrate's native shape), applied
with the `node-pg-migrate` CLI already in `node_modules` (no global install):

```sh
make migrate-up                   # node-pg-migrate up
make migrate-add NAME=add_orders  # stamps <ts>_add_orders.mjs (up + down exports)
```

Every `up` ships its `down`; the template ships `0001_init` (the whole core
schema) and `0002_profiles` (1:1 profile tables), new files get a timestamp
from `migrate-add`. Down sections are exercised by the integration suite
(apply → down → apply) so reversal stays honest. `0001_init` creates the whole
template schema; after instantiating, delete the `widgets` table from _your_
0001 — not from a template file you no longer own.

**Why never at startup**: schema changes belong to a deployment step with a
human or pipeline in the loop — two replicas racing a startup migration, or a
canary applying 0007 while prod runs 0006 code, are failure modes the rule
removes. The compose boot path makes it concrete: a one-shot `migrations`
service (`node-pg-migrate up`) runs first and the API waits on
`service_completed_successfully`; `make migrate-up` is the local-dev
equivalent. Startup failing because the schema drifted (`readyz` degraded) is
the correct signal instead.

## 4. Adapter shape

The pg adapter is dumb by design — call the slice's queries, map to domain
types, translate errors:

```ts
async get(id: string, ownerId: string): Promise<Widget | null> {
  const r = await widgetGetByID(this.#db, { id, ownerId });
  return r === null ? null : conv(r);
}
```

Mapping lives in exactly one direction per adapter file (`conv`); no raw SQL
outside the slice's `queries.ts`; no `pg`/`kysely` types leaking past this
file. Not-found is `null` flowing inward, thrown `AppError`s flow outward.
Constraint translations happen here too: the signup adapter maps Postgres
`23505` (unique_violation) onto the domain's `AUTH_EMAIL_TAKEN` — one `if`,
no dependency, and the use case stays DB-agnostic.

## 5. The template schema (parity core)

Tables shipped in `0001`: `users` (role/user_type `CHECK` constraints — TEXT,
not native enums, so down-migrations and typed queries stay trivial —
is_active / is_blocked / is_verified, retry_count, pepper-hashed password,
last_login, language, google_id/github_id — the columns the OAuth flows link
by), `sessions` (opaque 64-char refresh token unique, ip/ua, is_active,
expires_at) — one table, no legacy duplicate, `email_verifications` (6-digit
code, 24h, brute-force attempt counter), `password_resets` (30m, the `email`
column keeps the rate limit honest even for unknown users),
`settings` (key/value + is_sensitive + updated_by), `audit_logs` (actor,
action, entity, payload JSONB, created_at indexed), `email_jobs` (outbox:
to_email, template_key, payload JSONB, status queued/sending/sent/failed,
retry_count, max_retries, last_error, scheduled_at), and the example
`widgets` table (kind CHECK `sprocket/cog/flange`, owner FK).

`0002_profiles` adds two 1:1 profile tables keyed by `user_id UUID PRIMARY KEY
REFERENCES users(id) ON DELETE CASCADE`: `companies` (company_name, website,
industry, tax/registration ids, address, phone) and `users_profile` (full_name,
bio, avatar_url, date_of_birth, address, occupation, preferences JSONB DEFAULT
'{}'). Both downs drop cleanly — the integration suite's apply→down→apply pass
exercises them.

Indexes: unique on emails/tokens/keys, FK cascades from users, `created_at`
DESC on audit logs + users, and a partial `idx_email_jobs_queued ON
email_jobs (scheduled_at) WHERE status = 'queued'` for the worker claim (plus
the partial `idx_sessions_user_active` for revoke-all/touch).

## 6. Transactions across ports

A use case that must write two tables atomically gets a port that says so —
not a global transaction middleware. Kysely's `db.transaction()` wraps the
unit of work _inside the adapter_; the port method stays narrow:

```ts
// ports.ts — the consumer asks for atomicity, not for a tx handle
rotateSession(sessionId: string, oldToken: string, next: SessionRow): Promise<boolean>;

// postgres.ts — guarded in-place rotation ships as ONE statement:
// the WHERE on the old token is a compare-and-swap, so a replayed
// token can only ever win once. Multi-write cases use
//   await this.#db.transaction().execute(async (tx) => {...})
```

**Shipped state:** the outbox `enqueue` still uses the pool directly, not a
caller-supplied transaction. The auth write and the `email_jobs` insert are
therefore not atomic; treat this section as the target shape, not a property
of the current signup/reset paths.

Cross-slice transactions are a boundary smell: promote the operation (its
queries live where the tables are owned) rather than opening multi-slice
transactions.

## See also

- [Playbooks](12-playbooks.md) — migrate-add step by step
- [Dependency Rules](04-dependency-rules.md) — the SQL-ownership invariants
- [Auth Architecture](06-auth-architecture.md) — what each table serves
