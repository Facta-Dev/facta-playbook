// <app> — the notes view model.
//
// The view model holds the presentation state and the commands, with no
// React, no DOM and no chrome (PLAYBOOK §3.1). These tests drive it directly
// against a fake repository and pin every status of the list.

import { describe, expect, it } from "vitest";
import {
  createNotesViewModel,
  type NotesViewModel,
} from "../../../../../src/features/notes/viewmodels/notes-view-model";
import {
  type NoteRepository,
  type NotesError,
} from "../../../../../src/features/notes/application/ports";
import { type Note, type NoteDraft } from "../../../../../src/features/notes/domain/note";
import { asNoteId } from "../../../../../src/shared-kernel/ids";
import { err, ok, type Result } from "../../../../../src/shared-kernel/result";

function makeNote(id: string, title = "a title"): Note {
  return {
    id: asNoteId(id),
    title,
    body: "a body",
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: "2024-01-01T00:00:00.000Z",
  };
}

/** A repository that keeps the notes in memory. */
function createRepository(initial: Note[] = []): NoteRepository & { readonly notes: () => Note[] } {
  const notes = [...initial];
  return {
    list(): Promise<Result<Note[], NotesError>> {
      return Promise.resolve(ok([...notes]));
    },
    add(draft: NoteDraft): Promise<Result<Note, NotesError>> {
      const note: Note = {
        id: asNoteId(`note-${notes.length + 1}`),
        title: draft.title,
        body: draft.body,
        createdAt: "2024-01-01T00:00:00.000Z",
        updatedAt: "2024-01-01T00:00:00.000Z",
      };
      notes.push(note);
      return Promise.resolve(ok(note));
    },
    notes: () => [...notes],
  };
}

/** A repository whose reads always fail. */
function createFailingRepository(message: string): NoteRepository {
  return {
    list(): Promise<Result<Note[], NotesError>> {
      return Promise.resolve(err({ code: "storage-failed", message }));
    },
    add(): Promise<Result<Note, NotesError>> {
      return Promise.resolve(err({ code: "storage-failed", message }));
    },
  };
}

describe("notes view model", () => {
  it("starts in the loading status", () => {
    const viewModel = createNotesViewModel(createRepository());
    expect(viewModel.getState().status).toBe("loading");
    expect(viewModel.getState().notes).toEqual([]);
    expect(viewModel.getState().error).toBeNull();
  });

  it("goes to the empty status when there is no note", async () => {
    const viewModel = createNotesViewModel(createRepository());
    await viewModel.load();
    expect(viewModel.getState().status).toBe("empty");
    expect(viewModel.getState().error).toBeNull();
  });

  it("drops a stored note that breaks the domain rules", async () => {
    const viewModel = createNotesViewModel(
      createRepository([{ ...makeNote("note-1"), title: "<b>a title</b>" }]),
    );
    await viewModel.load();
    expect(viewModel.getState().notes).toEqual([]);
    expect(viewModel.getState().status).toBe("empty");
  });

  it("goes to the ready status when there are notes", async () => {
    const viewModel = createNotesViewModel(createRepository([makeNote("note-1")]));
    await viewModel.load();
    expect(viewModel.getState().status).toBe("ready");
    expect(viewModel.getState().notes).toHaveLength(1);
  });

  it("goes to the error status when the notes cannot be read", async () => {
    const viewModel = createNotesViewModel(createFailingRepository("The notes could not be read."));
    await viewModel.load();
    expect(viewModel.getState().status).toBe("error");
    expect(viewModel.getState().error).toBe("The notes could not be read.");
    expect(viewModel.getState().notes).toEqual([]);
  });

  it("keeps the title and the body that are being typed", () => {
    const viewModel = createNotesViewModel(createRepository());
    viewModel.setTitle("a title");
    viewModel.setBody("a body");
    expect(viewModel.getState().title).toBe("a title");
    expect(viewModel.getState().body).toBe("a body");
  });

  it("stores the note that was typed and clears the form", async () => {
    const repository = createRepository();
    const viewModel = createNotesViewModel(repository);
    viewModel.setTitle("a title");
    viewModel.setBody("a body");

    await viewModel.submit();

    const state = viewModel.getState();
    expect(state.status).toBe("ready");
    expect(state.notes).toHaveLength(1);
    expect(state.notes[0]?.title).toBe("a title");
    expect(state.title).toBe("");
    expect(state.body).toBe("");
    expect(state.submitting).toBe(false);
    expect(state.error).toBeNull();
    expect(repository.notes()).toHaveLength(1);
  });

  it("keeps the note that was typed when the store refuses it", async () => {
    const viewModel = createNotesViewModel(
      createFailingRepository("The note could not be stored."),
    );
    viewModel.setTitle("a title");

    await viewModel.submit();

    const state = viewModel.getState();
    expect(state.error).toBe("The note could not be stored.");
    expect(state.submitting).toBe(false);
    expect(state.notes).toEqual([]);
  });

  it("refuses to store a note without a title", async () => {
    const repository = createRepository();
    const viewModel = createNotesViewModel(repository);
    viewModel.setBody("a body");

    await viewModel.submit();

    expect(viewModel.getState().error).toBe("The title is required.");
    expect(viewModel.getState().submitting).toBe(false);
    expect(repository.notes()).toEqual([]);
  });

  it("notifies the subscribers until they unsubscribe", () => {
    const viewModel = createNotesViewModel(createRepository());
    let calls = 0;
    const unsubscribe = viewModel.subscribe(() => {
      calls += 1;
    });

    viewModel.setTitle("a title");
    expect(calls).toBe(1);

    unsubscribe();
    viewModel.setTitle("another title");
    expect(calls).toBe(1);
  });

  it("ignores a second submit while one is running", async () => {
    const repository = createRepository();
    const viewModel: NotesViewModel = createNotesViewModel(repository);
    viewModel.setTitle("a title");

    await Promise.all([viewModel.submit(), viewModel.submit()]);

    expect(repository.notes()).toHaveLength(1);
  });
});
