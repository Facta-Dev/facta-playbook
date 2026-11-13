// <app> — the notes view model.
//
// The presentation state of the slice, without a framework: a state, a set
// of commands and a subscription. React enters through the hook adapter in
// views/, React Native never enters the slice at all.
//
// The status is the shared eight of the client contract:
// loading | ready | empty | forbidden | not-found | offline | error |
// policy-blocked (PLAYBOOK_FRONTEND_MOBILE.md §4.1). The example slice
// only reaches the first four; the rest belong to the cases the guards and
// the offline outbox produce.

import { addNote, listNotes } from "../application/use-cases";
import type { NotesRepositoryPort } from "../application/ports";
import type { Note, NoteDraft } from "../domain/note";

/** The statuses a view model can be in. */
export const NOTES_STATUSES = [
  "loading",
  "ready",
  "empty",
  "forbidden",
  "not-found",
  "offline",
  "error",
  "policy-blocked",
] as const;

/** The status of a view model. */
export type NotesStatus = (typeof NOTES_STATUSES)[number];

/** What the view renders. */
export interface NotesViewState {
  /** Where the slice is. */
  readonly status: NotesStatus;
  /** The notes to render, newest first. */
  readonly notes: readonly Note[];
  /** The message of the failure, when there is one. */
  readonly errorMessage?: string | undefined;
}

/** The commands the view can invoke. */
export interface NotesViewModel {
  /** The current state. */
  readonly getState: () => NotesViewState;
  /** Subscribes to state changes; returns the unsubscribe. */
  readonly subscribe: (listener: () => void) => () => void;
  /** Loads the notes. */
  readonly load: () => Promise<void>;
  /** Adds a note. */
  readonly add: (draft: NoteDraft) => Promise<void>;
}

/** What the view model needs from the outside world. */
export interface NotesViewModelOptions {
  /** Where the notes live. */
  readonly repository: NotesRepositoryPort;
  /** A fresh identifier for a new note. */
  readonly newId: () => string;
  /** The current timestamp, as an ISO string. */
  readonly now: () => string;
}

/** The notes view model. */
export function createNotesViewModel(options: NotesViewModelOptions): NotesViewModel {
  const listeners = new Set<() => void>();
  let state: NotesViewState = { status: "loading", notes: [] };

  const setState = (next: NotesViewState): void => {
    state = next;
    for (const listener of [...listeners]) listener();
  };

  const subscribe = (listener: () => void): (() => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };

  const load = async (): Promise<void> => {
    setState({ status: "loading", notes: state.notes, errorMessage: undefined });
    const result = await listNotes(options.repository);
    if (!result.ok) {
      setState({
        status: "error",
        notes: state.notes,
        errorMessage: result.error.message,
      });
      return;
    }
    setState({
      status: result.value.length === 0 ? "empty" : "ready",
      notes: result.value,
      errorMessage: undefined,
    });
  };

  const add = async (draft: NoteDraft): Promise<void> => {
    const result = await addNote(options.repository, draft, {
      newId: options.newId,
      now: options.now,
    });
    if (!result.ok) {
      setState({ ...state, status: "error", errorMessage: result.error.message });
      return;
    }
    const notes = [result.value, ...state.notes];
    setState({ status: "ready", notes, errorMessage: undefined });
  };

  return { getState: () => state, subscribe, load, add };
}
