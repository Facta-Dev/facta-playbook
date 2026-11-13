// <app> — the notes use cases.
//
// The use cases are tested against a fake repository: no network, no
// storage, no device. The fake is the port, so the test proves the use
// case, not the adapter.

import { describe, expect, it } from "vitest";
import { addNote, listNotes } from "../../../../packages/feature-notes/src/application/use-cases";
import type { NotesRepositoryPort } from "../../../../packages/feature-notes/src/application/ports";
import type { Note } from "../../../../packages/feature-notes/src/domain/note";

const dependencies = {
  newId: () => "note_new",
  now: () => "2026-01-01T00:00:00.000Z",
};

function fakeRepository(seed: readonly Note[] = []): NotesRepositoryPort & { saved: Note[] } {
  const notes: Note[] = [...seed];
  return {
    saved: notes,
    list() {
      return Promise.resolve([...notes]);
    },
    save(note) {
      notes.unshift(note);
      return Promise.resolve();
    },
  };
}

const existing: Note = {
  id: "note_1",
  title: "Shopping list",
  body: "milk",
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
};

describe("listNotes", () => {
  it("returns the notes of the repository", async () => {
    const result = await listNotes(fakeRepository([existing]));
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toHaveLength(1);
      expect(result.value[0]?.id).toBe("note_1");
    }
  });

  it("returns an empty list, not an error, when there is nothing", async () => {
    const result = await listNotes(fakeRepository());
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.value).toEqual([]);
  });

  it("turns a storage failure into a value", async () => {
    const broken: NotesRepositoryPort = {
      list() {
        return Promise.reject(new Error("the disk is full"));
      },
      save() {
        return Promise.reject(new Error("the disk is full"));
      },
    };
    const result = await listNotes(broken);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe("storage-failure");
      expect(result.error.message).toBe("the disk is full");
    }
  });
});

describe("addNote", () => {
  it("stores a valid note and returns it", async () => {
    const repository = fakeRepository();
    const result = await addNote(repository, { title: "Idea", body: "ship it" }, dependencies);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.id).toBe("note_new");
      expect(result.value.createdAt).toBe("2026-01-01T00:00:00.000Z");
    }
    expect(repository.saved).toHaveLength(1);
  });

  it("refuses an invalid draft and stores nothing", async () => {
    const repository = fakeRepository();
    const result = await addNote(repository, { title: "  ", body: "ship it" }, dependencies);
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-note");
    expect(repository.saved).toHaveLength(0);
  });

  it("turns a storage failure into a value", async () => {
    const broken: NotesRepositoryPort = {
      list() {
        return Promise.resolve([]);
      },
      save() {
        return Promise.reject(new Error("the disk is full"));
      },
    };
    const result = await addNote(broken, { title: "Idea", body: "" }, dependencies);
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("storage-failure");
  });
});
