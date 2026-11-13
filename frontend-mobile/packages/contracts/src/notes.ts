// <app> — the notes DTO schemas.
//
// The wire format is defined once, here, and validated with zod on both
// ends of the network. A screen never builds a DTO by hand and never
// trusts a payload it did not parse.

import { z } from "zod";

/** The longest a note title may be. Mirrors the domain invariant. */
export const NOTE_TITLE_MAX_LENGTH = 120;

/** The longest a note body may be. Mirrors the domain invariant. */
export const NOTE_BODY_MAX_LENGTH = 10_000;

/** A note as it travels over the network. */
export const NoteDto = z.object({
  id: z.string().min(1),
  title: z.string().min(1).max(NOTE_TITLE_MAX_LENGTH),
  body: z.string().max(NOTE_BODY_MAX_LENGTH),
  createdAt: z.string().min(1),
  updatedAt: z.string().min(1),
});

/** A page of notes. */
export const NoteListDto = z.object({
  items: z.array(NoteDto),
  total: z.number().int().nonnegative(),
});

/** The payload of a create-note request. */
export const CreateNoteInput = z.object({
  title: z.string().min(1).max(NOTE_TITLE_MAX_LENGTH),
  body: z.string().max(NOTE_BODY_MAX_LENGTH),
});

/** A note as it travels over the network. */
export type NoteDto = z.infer<typeof NoteDto>;

/** A page of notes. */
export type NoteListDto = z.infer<typeof NoteListDto>;

/** The payload of a create-note request. */
export type CreateNoteInput = z.infer<typeof CreateNoteInput>;
