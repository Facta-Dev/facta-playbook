// <app> — the notes ports.
//
// The application layer talks to the outside world through interfaces it
// owns. The implementation lives in adapters/out (PLAYBOOK §3.1).

import { type Note, type NoteDraft, type NoteError } from "../domain/note";
import { type Result } from "../../../shared-kernel/result";

/** A failure that is not a business rule: the storage layer refused. */
export interface StorageError {
  readonly code: "storage-failed";
  readonly message: string;
}

/** Every failure the notes application can report. */
export type NotesError = StorageError | NoteError;

/** Where the notes live. */
export interface NoteRepository {
  /** Every note, oldest first. */
  list(): Promise<Result<Note[], NotesError>>;
  /** Stores a note and returns it as it was stored. */
  add(draft: NoteDraft): Promise<Result<Note, NotesError>>;
}
