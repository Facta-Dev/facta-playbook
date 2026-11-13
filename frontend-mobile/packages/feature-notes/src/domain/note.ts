// <app> — the Note entity.
//
// The rules of a note live here and nowhere else: the title is short, the
// body is bounded and the text is plain — a note never carries markup. The
// entity is pure TypeScript: no React, no React Native, no Expo, no
// storage (PLAYBOOK_FRONTEND_MOBILE.md §4.1).

import { err, ok, type Result } from "@<scope>/core";

/** The longest a title may be. */
export const NOTE_TITLE_MAX_LENGTH = 120;

/** The longest a body may be. */
export const NOTE_BODY_MAX_LENGTH = 10_000;

/** The reason a note was refused. */
export type NoteErrorCode = "invalid-title" | "invalid-body";

/** The reason a note was refused. */
export interface NoteError {
  readonly code: NoteErrorCode;
  readonly message: string;
}

/** A note, as the domain sees it. */
export interface Note {
  readonly id: string;
  readonly title: string;
  readonly body: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

/** A note that is not written yet: the two fields the user types. */
export interface NoteDraft {
  readonly title: string;
  readonly body: string;
}

/** An HTML tag, opening or closing. A note is plain text. */
const HTML_TAG = /<\s*\/?\s*[a-z][^>]*>/i;

/** The title of a note: one character up to {@link NOTE_TITLE_MAX_LENGTH}. */
export function validateTitle(title: string): Result<string, NoteError> {
  const trimmed = title.trim();
  if (trimmed.length === 0) {
    return err({ code: "invalid-title", message: "The title is required." });
  }
  if (trimmed.length > NOTE_TITLE_MAX_LENGTH) {
    return err({
      code: "invalid-title",
      message: `The title is at most ${NOTE_TITLE_MAX_LENGTH} characters.`,
    });
  }
  if (HTML_TAG.test(title)) {
    return err({ code: "invalid-title", message: "The title cannot contain HTML." });
  }
  return ok(trimmed);
}

/** The body of a note: plain text up to {@link NOTE_BODY_MAX_LENGTH}. */
export function validateBody(body: string): Result<string, NoteError> {
  if (body.length > NOTE_BODY_MAX_LENGTH) {
    return err({
      code: "invalid-body",
      message: `The body is at most ${NOTE_BODY_MAX_LENGTH} characters.`,
    });
  }
  if (HTML_TAG.test(body)) {
    return err({ code: "invalid-body", message: "The body cannot contain HTML." });
  }
  return ok(body);
}

/**
 * A note, built from what the outside world says it is. The invariants are
 * checked here, so a note that reaches the application always holds.
 */
export function createNote(input: {
  id: string;
  title: string;
  body: string;
  createdAt: string;
  updatedAt: string;
}): Result<Note, NoteError> {
  const title = validateTitle(input.title);
  if (!title.ok) return err(title.error);
  const body = validateBody(input.body);
  if (!body.ok) return err(body.error);
  return ok({
    id: input.id,
    title: title.value,
    body: body.value,
    createdAt: input.createdAt,
    updatedAt: input.updatedAt,
  });
}
