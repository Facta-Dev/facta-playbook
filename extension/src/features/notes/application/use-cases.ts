// <app> — the notes use cases.
//
// The application layer: it orchestrates the domain and the ports. A use
// case returns a Result; it never throws for an expected failure
// (PLAYBOOK §6.1).

import { createNote, validateBody, validateTitle, type Note } from "../domain/note";
import { err, ok, type Result } from "../../../shared-kernel/result";
import { type NoteRepository, type NotesError } from "./ports";

/**
 * Every note, oldest first. The domain is the last word on what a surface
 * may render: a note that breaks the rules (markup, an over-long title)
 * is dropped here, whichever adapter produced it.
 */
export async function listNotes(repository: NoteRepository): Promise<Result<Note[], NotesError>> {
  const stored = await repository.list();
  if (!stored.ok) return stored;
  const notes: Note[] = [];
  for (const item of stored.value) {
    const note = createNote(item);
    if (note.ok) notes.push(note.value);
  }
  return ok(notes);
}

/**
 * Stores a note. The draft is validated by the domain first, so an invalid
 * note never reaches the storage layer.
 */
export async function addNote(
  repository: NoteRepository,
  draft: { title: string; body: string },
): Promise<Result<Note, NotesError>> {
  const title = validateTitle(draft.title);
  if (!title.ok) return err(title.error);
  const body = validateBody(draft.body);
  if (!body.ok) return err(body.error);
  return await repository.add({ title: title.value, body: body.value });
}
