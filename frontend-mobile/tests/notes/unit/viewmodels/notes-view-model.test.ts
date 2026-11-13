// <app> — the notes view model.
//
// The presentation state and the commands, without a renderer: the view
// model is framework-free, so a node test covers it entirely.

import { describe, expect, it } from "vitest";
import {
  createNotesViewModel,
  type NotesViewModel,
} from "../../../../packages/feature-notes/src/viewmodels/notes-view-model";
import type { NotesRepositoryPort } from "../../../../packages/feature-notes/src/application/ports";
import type { Note } from "../../../../packages/feature-notes/src/domain/note";

const existing: Note = {
  id: "note_1",
  title: "Shopping list",
  body: "milk",
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
};

function repository(seed: readonly Note[] = []): NotesRepositoryPort {
  const notes: Note[] = [...seed];
  return {
    list() {
      return Promise.resolve([...notes]);
    },
    save(note) {
      notes.unshift(note);
      return Promise.resolve();
    },
  };
}

function viewModel(seed: readonly Note[] = []): NotesViewModel {
  return createNotesViewModel({
    repository: repository(seed),
    newId: () => "note_new",
    now: () => "2026-01-02T00:00:00.000Z",
  });
}

describe("the notes view model", () => {
  it("starts in the loading status", () => {
    expect(viewModel().getState().status).toBe("loading");
  });

  it("becomes ready when there are notes", async () => {
    const model = viewModel([existing]);
    await model.load();
    expect(model.getState().status).toBe("ready");
    expect(model.getState().notes).toHaveLength(1);
  });

  it("becomes empty when there are none", async () => {
    const model = viewModel();
    await model.load();
    expect(model.getState().status).toBe("empty");
    expect(model.getState().notes).toEqual([]);
  });

  it("becomes an error when the storage fails", async () => {
    const broken: NotesRepositoryPort = {
      list() {
        return Promise.reject(new Error("the disk is full"));
      },
      save() {
        return Promise.reject(new Error("the disk is full"));
      },
    };
    const model = createNotesViewModel({
      repository: broken,
      newId: () => "note_new",
      now: () => "2026-01-02T00:00:00.000Z",
    });
    await model.load();
    expect(model.getState().status).toBe("error");
    expect(model.getState().errorMessage).toBe("the disk is full");
  });

  it("adds a note and puts it first", async () => {
    const model = viewModel([existing]);
    await model.load();
    await model.add({ title: "Idea", body: "ship it" });
    const state = model.getState();
    expect(state.status).toBe("ready");
    expect(state.notes).toHaveLength(2);
    expect(state.notes[0]?.id).toBe("note_new");
  });

  it("keeps the last state when an add is refused", async () => {
    const model = viewModel([existing]);
    await model.load();
    await model.add({ title: "", body: "ship it" });
    const state = model.getState();
    expect(state.status).toBe("error");
    expect(state.notes).toHaveLength(1);
  });

  it("notifies the subscribers of a state change", async () => {
    const model = viewModel([existing]);
    let notifications = 0;
    const unsubscribe = model.subscribe(() => {
      notifications += 1;
    });
    await model.load();
    expect(notifications).toBeGreaterThan(0);
    unsubscribe();
    const seen = notifications;
    await model.load();
    expect(notifications).toBe(seen);
  });
});
