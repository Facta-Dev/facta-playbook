// <app> — the notes view model.
//
// Presentation state and commands, with no React, no DOM and no chrome
// (PLAYBOOK §3.1). A view subscribes to it; a test drives it directly.

import { type Note } from "../domain/note";
import { type NoteRepository } from "../application/ports";
import { addNote, listNotes } from "../application/use-cases";

/** The status of the list, as the surfaces of PLAYBOOK §10 name it. */
export type NotesStatus = "loading" | "ready" | "empty" | "error";

export interface NotesViewState {
  readonly status: NotesStatus;
  readonly notes: readonly Note[];
  /** The message of the last failure, or null. */
  readonly error: string | null;
  /** The title being typed. */
  readonly title: string;
  /** The body being typed. */
  readonly body: string;
  /** True while a note is being stored. */
  readonly submitting: boolean;
}

export interface NotesViewModel {
  /** The current state. The reference changes only when the state changes. */
  readonly getState: () => NotesViewState;
  /** Subscribes to the state; the returned function unsubscribes. */
  readonly subscribe: (listener: () => void) => () => void;
  readonly setTitle: (title: string) => void;
  readonly setBody: (body: string) => void;
  /** Reads the notes. */
  readonly load: () => Promise<void>;
  /** Validates and stores what is being typed. */
  readonly submit: () => Promise<void>;
}

const INITIAL: NotesViewState = {
  status: "loading",
  notes: [],
  error: null,
  title: "",
  body: "",
  submitting: false,
};

export function createNotesViewModel(repository: NoteRepository): NotesViewModel {
  let state: NotesViewState = INITIAL;
  const listeners = new Set<() => void>();

  const set = (patch: Partial<NotesViewState>): void => {
    state = { ...state, ...patch };
    for (const listener of listeners) listener();
  };

  return {
    getState: () => state,

    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    setTitle: (title) => set({ title }),
    setBody: (body) => set({ body }),

    load: async () => {
      set({ status: "loading", error: null });
      const result = await listNotes(repository);
      if (!result.ok) {
        set({ status: "error", notes: [], error: result.error.message });
        return;
      }
      set({
        status: result.value.length === 0 ? "empty" : "ready",
        notes: result.value,
        error: null,
      });
    },

    submit: async () => {
      if (state.submitting) return;
      set({ submitting: true, error: null });
      const result = await addNote(repository, { title: state.title, body: state.body });
      if (!result.ok) {
        set({ submitting: false, error: result.error.message });
        return;
      }
      const notes = [...state.notes, result.value];
      set({
        notes,
        status: "ready",
        title: "",
        body: "",
        submitting: false,
        error: null,
      });
    },
  };
}
