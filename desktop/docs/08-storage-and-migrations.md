# 08 — Storage & Migrations

## What the local database is

A **projection cache**, not the source of truth. The server owns the data;
SQLite holds what the server has sent, so the app can start instantly and
work offline. A row in SQLite may be stale; it may be a write that has not
been accepted by the server yet (see
[09 — Offline & Outbox](09-offline-and-outbox.md)).

The rule that follows: **the local schema may never be the only copy of a
rule.** A title longer than 120 characters is rejected by
`domain::Note::create`, not by a `CHECK` constraint that SQLite happens to
enforce.

## Where the file lives

`AppPaths::databaseFile()` resolves through `QStandardPaths`, so the location
is the operating system's convention. It asks for `AppLocalDataLocation`, with
the organisation and application names taken from `QApplication`, so on Linux
the file lands in `~/.local/share/<scope>/<app>/`; macOS and Windows use
their own conventions. The function to change is `AppPaths::dataDir()`.

**Nothing is ever written next to the executable.** An installed app lives in
a read-only directory on Linux, in `/Applications` on macOS and under
`Program Files` on Windows. The full path list — config, data, cache, log,
runtime, secrets, lock — is in
`src/platform/include/desktop/platform/app_paths.h`.

## The store

`desktop::infrastructure::storage::NoteStore` owns the connection and the
schema. It is in the infrastructure layer because it knows SQL, and the
domain knows nothing about it. Opening it creates the directory, opens a
**named** connection (not the default one, so two stores in one process do
not share a handle), and sets two pragmas:

| Setting | Value | Why |
| --- | --- | --- |
| `journal_mode` | `WAL` | The reader (the list) does not block the writer (a sync) |
| `synchronous` | `NORMAL` | In WAL mode this survives a crash, not a power cut — the right trade for a cache |
| `user_version` | the schema version | The migration pointer |

A missing `QSQLITE` driver is fatal and says so.

## Migrations

The schema version lives in SQLite's own `PRAGMA user_version`. There is no
migration table, no framework and no down path.

```cpp
QVector<Migration> NoteStore::migrations() {
    return QVector<Migration>{
        {1, R"(CREATE TABLE IF NOT EXISTS notes (
                   id TEXT PRIMARY KEY NOT NULL,
                   title TEXT NOT NULL,
                   body TEXT NOT NULL,
                   created_at TEXT NOT NULL,
                   updated_at TEXT NOT NULL
               ))"},
    };
}
```

`migrate()` reads the current version, applies every migration with a
higher version in order, and writes `PRAGMA user_version` after each one.

The rules:

1. **Forward only.** A downgrade is "restore the backup" or "reinstall". A
   `down` migration that runs on a user's data is a data-loss bug waiting
   for a bad release.
2. **One migration per change.** Never edit a shipped migration; add a new
   one. Editing migration 1 leaves every installed client on an unknown
   schema.
3. **Additive by default.** Add a column with a default, add a table, add an
   index. A rewrite of the `notes` table means a migration that copies the
   rows — acceptable, but it must be written as a new migration.
4. **The version is the migration's number**, and it is written to
   `user_version` inside the same run, so an interrupted upgrade resumes
   from the last applied step.

To add a migration, append `{2, R"(ALTER TABLE notes ADD COLUMN pinned
INTEGER NOT NULL DEFAULT 0)"}` to the list and bump nothing else: the
version *is* the index.

> **Known gap.** `migrate()` runs on the GUI thread at startup
> ([07](07-threading-and-the-gui-thread.md)). It is fast today because the
> schema is one table; a migration that rewrites rows must move to the
> worker, and the app must show progress instead of a frozen window.

## The contract between the layers

| Layer | Type | Knows about |
| --- | --- | --- |
| Domain | `desktop::notes::domain::Note` | The rules, the lengths, the markup rejection |
| Contracts | `desktop::contracts::NoteDto` | The wire shape: `id`, `title`, `body`, `created_at`, `updated_at` |
| Infrastructure | `desktop::infrastructure::storage::NoteStore` | The columns, the SQL, the pragmas |
| Adapter | `desktop::notes::adapters::SqliteNoteRepository` | The translation between the two |

The adapter is the only place where the two shapes meet: it reads the DTOs,
calls `Note::fromDto` on each one and returns domain objects. `fromDto`
re-applies the invariants to data that comes from storage, so a row that a
previous version wrote, or that a bug wrote, cannot enter the domain with a
5 000-character title.

## Backup, integrity and retention

| Concern | Rule |
| --- | --- |
| Backup | The database is a cache: it can be deleted. Never put something in it that exists nowhere else |
| Corruption | SQLite's WAL recovers a crashed process. A corrupt file is deleted and re-fetched from the server |
| Retention | Rows the server no longer returns are deleted on the next sync |
| Secrets | Never in the database — the token lives in the keystore ([07](07-threading-and-the-gui-thread.md)) |

## Testing the storage

`src/infrastructure/tests/tst_runtime.cpp` covers the store: open, migrate,
list, upsert. Two rules for the test suite: **use a temporary file**, never
the user's database (`QTemporaryDir` gives a path that disappears with the
test), and **assert the version, not the SQL** — a test that asserts
`PRAGMA user_version == 2` survives a reworded migration, while a test that
asserts the exact `CREATE TABLE` text breaks on every formatting change.

## Do / don't

| Do | Don't |
| --- | --- |
| Add a migration for every schema change | Edit a shipped migration |
| Keep the rules in the domain | Rely on a `CHECK` constraint for a business rule |
| Resolve paths through `QStandardPaths` | Write next to the executable |
| Delete the cache when it is corrupt | Try to repair it with ad-hoc SQL |
| Use `QTemporaryDir` in the tests | Open the user's database in a test |

## See also

- [Offline & Outbox](09-offline-and-outbox.md) — what happens to a write.
- [Threading & the GUI Thread](07-threading-and-the-gui-thread.md) — the
  blocking startup call.
- [Data Flows](data-flows.md) — the hop into SQLite.
