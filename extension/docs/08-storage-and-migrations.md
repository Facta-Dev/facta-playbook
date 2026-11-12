# Storage & Migrations

Three storage areas, one typed store per slice, and an explicit policy for
the records that cannot be read.

## 1. The three areas

| Area                     | Lifetime                                  | Written by                     | Holds                             |
| ------------------------ | ----------------------------------------- | ------------------------------ | --------------------------------- |
| `chrome.storage.local`   | Durable, survives restarts and updates    | The worker and the panel       | The notes record, the user policy |
| `chrome.storage.session` | Volatile, cleared when the browser closes | The mirror of the durable area | A cache the worker may rebuild    |
| `chrome.storage.managed` | Read-only, set by the administrator       | The administrator              | The managed policy                |

`src/platform/storage.ts` is the only file that names `chrome.storage`. It
exposes three small interfaces — `StorageArea` (`get`/`set`/`remove`),
`ManagedArea` (`read`) and `StorageAdapter` (the three areas) — so a feature
never sees a `chrome.storage` type.

## 2. The write-through mirror

`createWriteThrough(durable, mirror, delayMs = 250)` writes to the durable
area first and mirrors into the volatile one after a debounce. The worker
builds it once:

```ts
const local = createWriteThrough(storage.local, storage.session);
```

The ceiling is stated in the source: a debounce timer does not survive the
worker, so a mirror write can be lost on termination. The mirror is a cache,
never the source of truth.

## 3. The typed store

`src/storage/notes-store.ts` is the reference implementation:

```text
NOTES_STORAGE_KEY   = "notes:record"
NOTES_BACKUP_KEY    = "notes:record.backup"
NOTES_STORAGE_VERSION = 2      (src/contracts/storage.ts)
```

Its responsibilities:

1. **Versioned records.** The record carries `version`, `notes` and
   `updatedAt`; `NotesRecordSchema` is a strict object, so an unexpected key
   is a parse failure, not a silent extra field.
2. **Ordered migrations.** `MIGRATIONS` is a list of steps, each with a
   `from` and a `to` version. The store applies the steps that match the
   version it found.
3. **Corrupt-record policy.** A record that fails to parse is _dropped_, not
   thrown: the raw value is written to `notes:record.backup` and the store
   starts empty. A broken record never reaches the worker.
4. **A newer record is also dropped** (with the same backup), because a
   downgrade must not corrupt the newer data.
5. **Serialised writes.** `run()` chains the writes through a promise queue,
   so two overlapping `add` calls cannot interleave.
6. **A failed load is retried.** The memoised load promise is cleared when
   it rejects, so a transient storage error does not poison the worker.

## 4. The migration step

The shipped `1 → 2` step renames the note body from `text` to `body` and
stamps `updatedAt`. It rebuilds each note from its named fields instead of
spreading the legacy object, because `NoteSchema` is strict: a surviving
`text` key fails `NotesRecordSchema.safeParse` and the store takes the drop
path — the old notes land in `notes:record.backup` and the store starts
empty. The step that ships avoids exactly that:

```ts
notes: notes.map((note) => {
  const item = isRecord(note) ? note : {};
  const createdAt = typeof item.createdAt === "string" ? item.createdAt : EPOCH;
  return {
    id: item.id,
    title: item.title,
    body:
      typeof item.body === "string"
        ? item.body
        : typeof item.text === "string"
          ? item.text
          : "",
    createdAt,
    updatedAt: typeof item.updatedAt === "string" ? item.updatedAt : createdAt,
  };
}),
```

The rule generalises: **a step removes the keys it replaces.** A strict
schema plus a spread is a migration that drops the data it was meant to
rescue.

Run the `storage-migration` skill
(`.claude/skills/storage-migration/SKILL.md`) and the
`storage-migration-reviewer` agent
(`.claude/agents/client/storage-migration-reviewer.md`) before shipping any
schema change.

## 5. Ownership of a key

A storage key belongs to a slice. The `notes` slice owns `notes:record` and
`notes:record.backup`; the policy module owns `policy:settings`
(`USER_POLICY_KEY`). Two slices never share a key — a shared key is a shared
slice in disguise.

## 6. Worked example

The worker reads through the store, which parses with the shared contract,
so a record written by an older build is migrated or dropped before it can
reach the UI. `listNotes` then re-applies the domain rules to what the
worker answers — the panel reads through the messaging client and never
touches the record itself.

## Do / Don't

| Do                                              | Don't                                        |
| ----------------------------------------------- | -------------------------------------------- |
| Bump `NOTES_STORAGE_VERSION` and add a step     | Change the shape without a migration         |
| Keep the schema strict                          | Accept unknown keys silently                 |
| Drop and back up a record that cannot be parsed | Throw and take the worker down               |
| Serialize the writes of one store               | Write the same key from two contexts at once |

## See also

- [Service Worker Lifetime](02-service-worker-lifetime.md)
- [Policy](09-policy-admin-over-user.md)
- [Playbooks](13-playbooks.md)
