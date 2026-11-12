// <app> — the notes DTO.
//
// The shape the service worker stores and the shapes the UI renders. It is
// validated on the way in and on the way out, so a half-written record can
// never reach a surface.

import { z } from "zod";
import { asNoteId } from "../shared-kernel/ids";

/** The wire bounds: a superset of the domain bounds in
 * `features/notes/domain/note.ts`, which are the stricter product rule. */
export const NOTE_TITLE_MAX_LENGTH = 200;
export const NOTE_BODY_MAX_LENGTH = 20_000;

export const NoteSchema = z.strictObject({
  id: z.string().min(1).transform(asNoteId),
  title: z.string().min(1).max(NOTE_TITLE_MAX_LENGTH),
  body: z.string().max(NOTE_BODY_MAX_LENGTH),
  createdAt: z.iso.datetime(),
  updatedAt: z.iso.datetime(),
});

export type Note = z.infer<typeof NoteSchema>;
