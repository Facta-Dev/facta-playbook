// <app> — the notes slice.
//
// The barrel of the slice: the composition root (app/App.tsx) imports from
// here, never from a layer inside (PLAYBOOK_FRONTEND_MOBILE.md §4.1).

export {
  NOTE_BODY_MAX_LENGTH,
  NOTE_TITLE_MAX_LENGTH,
  createNote,
  validateBody,
  validateTitle,
} from "./domain/note";
export type { Note, NoteDraft, NoteError, NoteErrorCode } from "./domain/note";

export { addNote, listNotes } from "./application/use-cases";
export type { NotesDependencies } from "./application/use-cases";
export type { NotesError, NotesErrorCode, NotesRepositoryPort } from "./application/ports";

export { createInMemoryNotesRepository } from "./adapters/out/in-memory-notes-repository";

export { NOTES_STATUSES, createNotesViewModel } from "./viewmodels/notes-view-model";
export type {
  NotesStatus,
  NotesViewModel,
  NotesViewModelOptions,
  NotesViewState,
} from "./viewmodels/notes-view-model";

export { NotesScreen } from "./views/notes-screen";
export type { NotesScreenProps } from "./views/notes-screen";
export { useNotesViewModel } from "./views/use-notes-view-model";
