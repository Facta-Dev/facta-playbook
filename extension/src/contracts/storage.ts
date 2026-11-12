// <app> — the storage contract.
//
// Every durable record carries its schema version, so a later build can
// migrate it (PLAYBOOK §7.2). The version constant lives with the schema:
// the store and the contract can never disagree about it.

import { z } from "zod";
import { NoteSchema } from "./notes";

/** The version of the notes record this build reads and writes. */
export const NOTES_STORAGE_VERSION = 2;

export const NotesRecordSchema = z.strictObject({
  version: z.number().int().positive(),
  notes: z.array(NoteSchema),
  updatedAt: z.iso.datetime(),
});

export type NotesRecord = z.infer<typeof NotesRecordSchema>;
