// <app> — the ports of the notes slice.
//
// The slice declares what it needs from the outside world; the adapter
// decides how to get it. Swapping the in-memory repository for a remote
// one is an adapter change, never a use-case change.

import type { Note } from "../domain/note";

/** Where the notes live. */
export interface NotesRepositoryPort {
  /** Every note, newest first. */
  list(): Promise<readonly Note[]>;
  /** Stores a note, replacing the one with the same id. */
  save(note: Note): Promise<void>;
}

/** The reason a use case failed. */
export type NotesErrorCode = "invalid-note" | "storage-failure";

/** The reason a use case failed. */
export interface NotesError {
  readonly code: NotesErrorCode;
  readonly message: string;
}
