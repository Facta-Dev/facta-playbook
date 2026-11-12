// <app> — the notes slice.
//
// The barrel of the slice: the composition roots import from here, never
// from a layer inside (PLAYBOOK §3.4).

export { createNotesViewModel } from "./viewmodels/notes-view-model";
export {
  type NotesViewModel,
  type NotesViewState,
  type NotesStatus,
} from "./viewmodels/notes-view-model";
export { NotesPanel } from "./ui/NotesPanel";
export { createRpcNoteRepository } from "./adapters/out/rpc-note-repository";
export { addNote, listNotes } from "./application/use-cases";
export { type NoteRepository, type NotesError } from "./application/ports";
export { createNote, type Note, type NoteDraft } from "./domain/note";
