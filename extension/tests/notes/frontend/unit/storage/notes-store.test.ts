// <app> — the notes store.
//
// The typed store owns the schema version, the migrations and the
// corrupt-record policy (PLAYBOOK §7.1, §7.2). These tests pin the two
// behaviors that decide what a user sees after an update: a v1 record is
// upgraded, a corrupt record is dropped instead of taking the worker down.

import { describe, expect, it } from "vitest";
import {
  createNotesStore,
  NOTES_BACKUP_KEY,
  NOTES_STORAGE_KEY,
} from "../../../../../src/storage/notes-store";
import { NOTES_STORAGE_VERSION } from "../../../../../src/contracts/storage";
import { createFakeStorage } from "../../../../_shared/fakes/fake-storage";
import { createLogger } from "../../../../../src/telemetry/logger";

describe("notes store", () => {
  // A v1 record keeps the body under "text" and has no "updatedAt": the
  // migration has to carry the body over, stamp the update time and leave
  // nothing the strict contract does not know about behind.
  it("upgrades a v1 record: the body comes from the legacy text", async () => {
    const area = createFakeStorage({
      [NOTES_STORAGE_KEY]: {
        notes: [
          {
            id: "note-1",
            title: "a title",
            text: "the legacy body",
            createdAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      },
    });
    const store = createNotesStore({ area, logger: createLogger() });

    const notes = await store.list();

    expect(notes).toHaveLength(1);
    expect(notes[0]?.body).toBe("the legacy body");
    expect(notes[0]?.updatedAt).toBe("2024-01-01T00:00:00.000Z");
  });

  it("writes the upgraded record back with the current version", async () => {
    const area = createFakeStorage({
      [NOTES_STORAGE_KEY]: {
        notes: [
          {
            id: "note-1",
            title: "a title",
            text: "the legacy body",
            createdAt: "2024-01-01T00:00:00.000Z",
          },
        ],
      },
    });
    const store = createNotesStore({ area, logger: createLogger() });

    await store.list();

    const stored = area.values()[NOTES_STORAGE_KEY] as { version: number };
    expect(stored.version).toBe(NOTES_STORAGE_VERSION);
  });

  it("drops a corrupt record and keeps it under the backup key", async () => {
    const corrupt = { version: 2, notes: "not a list", updatedAt: "not a date" };
    const area = createFakeStorage({ [NOTES_STORAGE_KEY]: corrupt });
    const store = createNotesStore({ area, logger: createLogger() });

    const notes = await store.list();

    expect(notes).toEqual([]);
    expect(area.values()[NOTES_BACKUP_KEY]).toEqual(corrupt);
  });

  it("starts empty after a corrupt record was dropped", async () => {
    const area = createFakeStorage({
      [NOTES_STORAGE_KEY]: { version: 2, notes: "not a list", updatedAt: "not a date" },
    });
    const store = createNotesStore({ area, logger: createLogger() });

    await store.list();

    const stored = area.values()[NOTES_STORAGE_KEY] as { version: number; notes: unknown[] };
    expect(stored.version).toBe(NOTES_STORAGE_VERSION);
    expect(stored.notes).toEqual([]);
  });

  it("drops a record written by a newer version", async () => {
    const area = createFakeStorage({
      [NOTES_STORAGE_KEY]: {
        version: NOTES_STORAGE_VERSION + 1,
        notes: [],
        updatedAt: "2024-01-01T00:00:00.000Z",
      },
    });
    const store = createNotesStore({ area, logger: createLogger() });

    expect(await store.list()).toEqual([]);
    expect(area.values()[NOTES_BACKUP_KEY]).toBeDefined();
  });

  it("appends a note to the record", async () => {
    const area = createFakeStorage();
    const store = createNotesStore({ area, logger: createLogger() });

    const note = await store.add({ title: "a title", body: "a body" });

    expect(note.title).toBe("a title");
    expect(note.body).toBe("a body");
    expect(await store.list()).toHaveLength(1);
  });
});
