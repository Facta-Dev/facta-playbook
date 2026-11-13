// <app> — the notes use cases.
//
// A use case is a plain function: it takes the ports it needs, returns a
// Result. It never imports React, React Native or Expo, and it never
// throws at the caller — a failed write is a value.

import { err, ok, type Result } from "@<scope>/core";
import { createNote, type Note, type NoteDraft } from "../domain/note";
import type { NotesError, NotesRepositoryPort } from "./ports";

/** The values a use case needs from the platform, injected by the caller. */
export interface NotesDependencies {
  /** A fresh identifier for a new note. */
  readonly newId: () => string;
  /** The current timestamp, as an ISO string. */
  readonly now: () => string;
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : "The storage failed.";
}

/** Every note, newest first. */
export async function listNotes(
  repository: NotesRepositoryPort,
): Promise<Result<readonly Note[], NotesError>> {
  try {
    return ok(await repository.list());
  } catch (cause) {
    return err({ code: "storage-failure", message: messageOf(cause) });
  }
}

/**
 * A new note, validated and stored.
 *
 * The domain checks the invariants before anything is written, so an
 * invalid note never reaches the storage.
 */
export async function addNote(
  repository: NotesRepositoryPort,
  draft: NoteDraft,
  dependencies: NotesDependencies,
): Promise<Result<Note, NotesError>> {
  const timestamp = dependencies.now();
  const created = createNote({
    id: dependencies.newId(),
    title: draft.title,
    body: draft.body,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  if (!created.ok) {
    return err({ code: "invalid-note", message: created.error.message });
  }
  try {
    await repository.save(created.value);
  } catch (cause) {
    return err({ code: "storage-failure", message: messageOf(cause) });
  }
  return ok(created.value);
}
