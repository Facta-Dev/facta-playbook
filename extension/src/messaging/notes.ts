// <app> — the notes handlers.
//
// The inbound adapter of the notes slice: it turns a validated request into
// a call on the store. The store is a port; the composition root wires the
// implementation.

import { type Note } from "../contracts/notes";
import { type HandlerTable } from "./server";

/** What the notes handlers need from the store. */
export interface NotesPort {
  list(): Promise<Note[]>;
  add(input: { title: string; body: string }): Promise<Note>;
}

export function createNotesHandlers(port: NotesPort): HandlerTable {
  return {
    "notes.list": async (_payload, _context) => ({ notes: await port.list() }),
    "notes.add": async (payload, _context) => ({
      note: await port.add({ title: payload.title, body: payload.body }),
    }),
  };
}
