# Persistence, Repositories & Migrations

The repository-trait/adapter pattern, compile-time-checked SQL with a committed
offline cache, and reversible migrations.

## 1. The repository trait (port)

A port in `<app>-ports/src/domain/`. Pure-domain signature; no `sqlx` types.

```rust
#[async_trait]
pub trait WidgetRepository: Send + Sync + 'static {
    async fn find(&self, id: WidgetId) -> Result<Option<Widget>>;
    async fn list(&self) -> Result<Vec<Widget>>;
    async fn upsert(&self, widget: Widget) -> Result<Widget>;
    async fn delete(&self, id: WidgetId) -> Result<bool>;
}
```

## 2. The Postgres adapter

The `Pg*` adapter in `<app>-ports/src/adapters/` is the **only** place SQL
lives. It uses `query_as!` (compile-time checked against the live schema) and
maps `sqlx::Error` into `core::Error` at the boundary:

```rust
fn map(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound,
        other => Error::Internal(other.to_string()),
    }
}
```

A private `*Row` (`#[derive(FromRow)]`) plus an `into_domain()` data mapper keep
the DB shape out of the domain — the anti-corruption seam.

## 3. Compile-time-checked SQL + `.sqlx/` offline cache

`sqlx` verifies every `query!`/`query_as!` against the real schema at build
time. To keep CI hermetic (no DB needed to *build*), the verified metadata is
committed as an offline cache in `.sqlx/`.

**The rule:** after editing any SQL, run `just sqlx-prepare` and commit `.sqlx/`,
or CI cannot build offline. A stale cache fails the build — schema and queries
cannot drift.

```mermaid
flowchart LR
    EDIT["edit SQL"] --> PREP["just sqlx-prepare<br/>(needs DATABASE_URL)"]
    PREP --> COMMIT["commit .sqlx/"]
    COMMIT --> CI["CI builds offline"]
    style PREP fill:#fef7e0,stroke:#f9ab00
```

## 4. Reversible, paired migrations

Every forward migration ships with its exact inverse. Scaffold a pair with
`just migrate-add <name>`, which writes `NNNN_<name>.up.sql` and
`NNNN_<name>.down.sql`. A deploy can be rolled back deterministically.

```sql
-- 0001_init.up.sql
CREATE TABLE widgets (id UUID PRIMARY KEY, name TEXT NOT NULL, ...);

-- 0001_init.down.sql
DROP TABLE widgets;
```

The composition root runs migrations on boot: `sqlx::migrate!("../../migrations")`.

## 5. One Postgres, transactions

A single `PgPool` shared as `Arc` through `AppState`. For a write spanning
multiple statements, the adapter opens a transaction from the pool and commits
atomically. Repositories take the pool by clone (cheap — inner `Arc`).

**Shipped state:** the outbox `enqueue` still executes on the pool, not on a
caller-supplied transaction. The domain write and the `email_jobs` insert are
therefore not atomic; treat the paragraph above as the target shape, not a
property of the current signup/reset paths.

## 6. Upsert + delta change-set

For partial updates, compute a delta (only changed fields) and upsert. This
avoids lost updates and keeps the write set minimal. The domain validates the
delta; the adapter persists it.

## Do / Don't

- **Do** commit `.sqlx/` after every SQL change.
- **Do** ship `.down.sql` for every `.up.sql`.
- **Don't** return `sqlx::Error` or `PgRow` from a trait method.
- **Don't** put SQL anywhere but `ports/adapters/`.

## See also

- [Crate topology](03-crate-topology.md)
- [Testing & CI](11-testing-and-ci.md)
- [Playbooks — ship a migration](12-playbooks.md)