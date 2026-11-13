// <app> — the in-memory notes repository.
//
// The out adapter of the slice: it answers the port with an array. It is
// what the app runs with until a real storage adapter (SQLite, MMKV, the
// SDK) replaces it — the use cases do not change.

import type { NotesRepositoryPort } from "../../application/ports";
import type { Note } from "../../domain/note";

/**
 * A repository that keeps the notes in memory, newest first.
 *
 * The seed is copied: the caller keeps its own array.
 */
export function createInMemoryNotesRepository(seed: readonly Note[] = []): NotesRepositoryPort {
  const notes: Note[] = [...seed];

  return {
    list(): Promise<readonly Note[]> {
      return Promise.resolve([...notes]);
    },
    save(note: Note): Promise<void> {
      const index = notes.findIndex((candidate) => candidate.id === note.id);
      if (index >= 0) {
        notes[index] = note;
        return Promise.resolve();
      }
      notes.unshift(note);
      return Promise.resolve();
    },
  };
}
