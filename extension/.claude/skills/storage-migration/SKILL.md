---
name: "<app> Storage Migration"
description: "Migrates a durable <app> extension record — the version bump, the pure (vN) to (vN+1) migration, the fixture of the previous version and the backup-key path — so an existing installation keeps its data."
division: client
---

# <app> Storage Migration

A reusable workflow for changing the shape of a durable record in the `<app>` extension. Every shipped installation already holds the old shape on disk, so a shape change without a version bump is a data-loss defect that no test in the repository will catch. This skill produces the version bump, the pure migration, the fixture of the previous version and the corrupt-record path together.

## When to Activate

- A durable record's shape changes: a field is added, removed, renamed or retyped.
- A record gains a new storage key, or a key's meaning changes.
- A migration chain has a gap — a version was shipped with no migration.
- A reviewer asks whether an existing installation survives a change.

Do not activate for a change that touches no durable state: a pure function, a UI state or a message payload that is never persisted needs no migration.

## Core Principles

- Version first. The record's version constant — `NOTES_STORAGE_VERSION` and its siblings — is bumped in the same change as the shape, and the migration is named for the pair it bridges.
- The migration is pure. It is a function of its input: no clock, no `chrome`, no I/O, no randomness. The caller passes the timestamp in. A pure migration is idempotent when re-run and testable without a browser.
- The chain is total. Every version the extension has ever shipped reaches the current one, in order, with no gap. A version with no migration is a record that no code path can read.
- Fail soft, keep the bytes. A record that fails to parse is moved to the backup key — `notes:record.backup` — and kept for one release cycle; it is never deleted and never allowed to take the service worker down.
- One writer. Durable state is mutated by the service worker alone, under a queued read-modify-write, so two panels acting at once cannot lose an update.

## Rules

- Anti-hallucination: the version constants, the key names and the migration file names must exist in the tree; do not invent a version that was never shipped.
- Every claim needs a source: cite the local store file for the current version and the official Chrome documentation for the storage area semantics.
- Do not emit the final output as a fenced code block when raw output is expected.
- A shape change without a version bump and a migration is a BLOCK.
- A migration that reads the clock, performs I/O or touches a global is a REVISE: pass the timestamp in and keep the function pure.
- Every migration ships with a fixture of the previous version, so the chain is exercised from the oldest supported shape, not only from the newest.
- The corrupt-record path is covered: a fixture that does not parse must land in the backup key and the read must return the empty state, not throw.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| `storage.local` persists across browser restarts, `storage.session` is cleared when the browser closes | `chrome.storage` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/storage |
| `storage.managed` is read-only and set by the administrator through a policy | `chrome.storage` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/storage |
| The storage quota and the `local` area limits | `chrome.storage` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/storage |
| The current version constant and the migration chain | `src/storage/notes-store.ts` | 2024 | local file |
| The backup-key behaviour on a corrupt record | `src/storage/notes-store.ts` | 2024 | local file |
| The storage versioning rules the migration must satisfy | `PLAYBOOK_CHROME_EXTENSION.md` | 2024 | local file |

## Output Format

A migration plan plus a chain summary. Plain text — not a fenced block:

```
MIGRATION  notes:record  v2 -> v3

  version bump:
    src/storage/notes-store.ts — NOTES_STORAGE_VERSION = 3

  migration:
    src/storage/migrations/notes-v2-to-v3.ts
      (record: NotesRecordV2) -> NotesRecordV3
      pure: no chrome, no clock, no I/O; the timestamp is a parameter
      idempotent: re-running it on a v3 record returns it unchanged

  fixture:
    tests/notes/frontend/unit/storage/fixtures/notes-record-v2.json
    the oldest shipped shape, so the whole chain is exercised

  corrupt path:
    a record that fails to parse is moved to notes:record.backup and the
    read returns the empty state

CHAIN
  v1 -> v2  present
  v2 -> v3  present (this change)
  gap: none

CHECKLIST
  [ ] version constant bumped in the same change
  [ ] migration is pure and idempotent
  [ ] the chain has no gap and covers every shipped version
  [ ] a fixture of the previous version exists and is used by the test
  [ ] the corrupt-record path is covered
  [ ] single-writer queue still serialises the read-modify-write
  [ ] pnpm run test green
```

Summary line: `storage-migration: N record(s) migrated, chain gap: none — verdict: APPROVE | REVISE`.
