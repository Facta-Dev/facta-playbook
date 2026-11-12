// <app> — the notes store.
//
// One typed store over the durable storage area (PLAYBOOK §7.1): a schema
// version, an ordered list of migrations that run once when the store is
// opened, and a corrupt-record policy that drops the record instead of
// taking the worker down.

import { type Note } from "../contracts/notes";
import { NOTES_STORAGE_VERSION, NotesRecordSchema, type NotesRecord } from "../contracts/storage";
import { type StorageArea } from "../platform/storage";
import { asNoteId } from "../shared-kernel/ids";
import { type Logger } from "../telemetry/logger";

/** The storage key, namespaced by slice. */
export const NOTES_STORAGE_KEY = "notes:record";

/** Where a dropped record is kept for one release cycle. */
export const NOTES_BACKUP_KEY = "notes:record.backup";

const EPOCH = "1970-01-01T00:00:00.000Z";

/** One migration step, from version `from` to version `to`. */
interface Migration {
  readonly from: number;
  readonly to: number;
  readonly migrate: (raw: unknown) => unknown;
}

/**
 * The ordered migrations. A v1 record kept the note body under `text` and
 * had no `updatedAt`; v2 uses `body` and stamps `updatedAt`.
 */
const MIGRATIONS: readonly Migration[] = [
  {
    from: 1,
    to: 2,
    migrate: (raw) => {
      const record = isRecord(raw) ? raw : {};
      const notes = Array.isArray(record.notes) ? record.notes : [];
      return {
        ...record,
        version: 2,
        updatedAt: typeof record.updatedAt === "string" ? record.updatedAt : EPOCH,
        // Rebuild each note from known fields only: the contract is strict, so
        // carrying the legacy `text` key over would fail validation and drop
        // the record it was meant to rescue.
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
      };
    },
  },
];

export interface NotesPort {
  /** The notes, oldest first. */
  list(): Promise<Note[]>;
  /** Appends a note. */
  add(input: { title: string; body: string }): Promise<Note>;
}

export interface NotesStoreOptions {
  readonly area: StorageArea;
  readonly logger: Logger;
}

export function createNotesStore(options: NotesStoreOptions): NotesPort {
  const { area, logger } = options;
  let opened: Promise<NotesRecord> | undefined;
  let queue: Promise<unknown> = Promise.resolve();

  // The worker is the only writer, but two messages can overlap on an await.
  const run = <T>(task: () => Promise<T>): Promise<T> => {
    const next = queue.then(task, task);
    queue = next.catch(() => undefined);
    return next;
  };

  const load = async (): Promise<NotesRecord> => {
    const raw = await area.get<unknown>(NOTES_STORAGE_KEY);
    if (raw === undefined) {
      const fresh = emptyRecord();
      await area.set(NOTES_STORAGE_KEY, fresh);
      return fresh;
    }
    const version = readVersion(raw);
    if (version > NOTES_STORAGE_VERSION) {
      logger.log("error", "notes record written by a newer version; dropped", { version });
      return await drop(raw);
    }
    const migrated = version === NOTES_STORAGE_VERSION ? raw : migrate(raw, version);
    const parsed = NotesRecordSchema.safeParse(migrated);
    if (!parsed.success) {
      logger.log("error", "notes record corrupt; dropped", { issue: parsed.error.issues[0]?.code });
      return await drop(raw);
    }
    if (version !== NOTES_STORAGE_VERSION) {
      await area.set(NOTES_STORAGE_KEY, parsed.data);
    }
    return parsed.data;
  };

  // A corrupt record is kept under a backup key for one release cycle and
  // the store starts empty: a broken record never reaches the worker.
  const drop = async (raw: unknown): Promise<NotesRecord> => {
    await area.set(NOTES_BACKUP_KEY, raw);
    const fresh = emptyRecord();
    await area.set(NOTES_STORAGE_KEY, fresh);
    return fresh;
  };

  const open = (): Promise<NotesRecord> => {
    opened ??= load().catch((error: unknown) => {
      // A storage failure must not poison the store for the rest of the
      // life of the worker: the next call tries again.
      opened = undefined;
      throw error;
    });
    return opened;
  };

  return {
    list: async () => (await open()).notes,
    add: (input) =>
      run(async () => {
        const record = await open();
        const now = new Date().toISOString();
        const note: Note = {
          id: asNoteId(crypto.randomUUID()),
          title: input.title,
          body: input.body,
          createdAt: now,
          updatedAt: now,
        };
        const next: NotesRecord = {
          ...record,
          notes: [...record.notes, note],
          updatedAt: now,
        };
        await area.set(NOTES_STORAGE_KEY, next);
        opened = Promise.resolve(next);
        return note;
      }),
  };
}

function emptyRecord(): NotesRecord {
  return { version: NOTES_STORAGE_VERSION, notes: [], updatedAt: new Date().toISOString() };
}

/** A record without a version is a v1 record. */
function readVersion(raw: unknown): number {
  if (isRecord(raw) && typeof raw.version === "number") return raw.version;
  return 1;
}

function migrate(raw: unknown, from: number): unknown {
  let current = raw;
  let version = from;
  for (const step of MIGRATIONS) {
    if (step.from !== version) continue;
    current = step.migrate(current);
    version = step.to;
  }
  return current;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
