// <app> — the notes contracts.
//
// The zod schemas of the notes slice. They live in src/contracts and are
// re-exported here, so the slice has one place to reach for its shapes and
// nobody writes a second schema (PLAYBOOK §3.4).
//
// The length constants are not re-exported: the slice has its own, stricter
// invariants in src/features/notes/domain/note.ts. The contract bounds what
// may travel on the wire; the domain bounds what may be a note.

export { NoteSchema, type Note as NoteDto } from "../../../contracts/notes";

export {
  NotesAddPayloadSchema,
  NotesAddResultSchema,
  NotesListPayloadSchema,
  NotesListResultSchema,
  type NotesAddPayload,
  type NotesAddResult,
  type NotesListPayload,
  type NotesListResult,
} from "../../../contracts/protocol";
