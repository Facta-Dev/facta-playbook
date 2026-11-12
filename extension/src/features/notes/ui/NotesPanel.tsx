// <app> — the notes side panel.
//
// A small, accessible view: a heading, a form, the list and the states the
// list can be in. The strings are English literals for now; a slice with
// translations moves them to chrome.i18n keys (PLAYBOOK §10).

import { useEffect, useRef } from "react";
import { NOTE_BODY_MAX_LENGTH, NOTE_TITLE_MAX_LENGTH } from "../domain/note";
import { type NotesViewModel } from "../viewmodels/notes-view-model";
import { useNotesViewModel } from "./useNotesViewModel";

export interface NotesPanelProps {
  readonly viewModel: NotesViewModel;
}

export function NotesPanel({ viewModel }: NotesPanelProps) {
  const { state, load, submit, setTitle, setBody } = useNotesViewModel(viewModel);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    load();
  }, [load]);

  // The focus goes back to the title after every attempt, so the user can
  // correct the note without reaching for the mouse.
  useEffect(() => {
    if (!state.submitting) titleRef.current?.focus();
  }, [state.submitting]);

  return (
    <section aria-labelledby="notes-heading">
      <h1 id="notes-heading">Notes</h1>

      <form
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label htmlFor="note-title">Title</label>
        <input
          id="note-title"
          ref={titleRef}
          value={state.title}
          maxLength={NOTE_TITLE_MAX_LENGTH}
          required
          onChange={(event) => setTitle(event.target.value)}
        />

        <label htmlFor="note-body">Note</label>
        <textarea
          id="note-body"
          value={state.body}
          maxLength={NOTE_BODY_MAX_LENGTH}
          onChange={(event) => setBody(event.target.value)}
        />

        <button type="submit" disabled={state.submitting}>
          {state.submitting ? "Saving…" : "Add note"}
        </button>
      </form>

      {state.error !== null && <p role="alert">{state.error}</p>}

      <ul aria-live="polite" aria-busy={state.status === "loading"}>
        {state.status === "loading" && <li>Loading notes…</li>}
        {state.status === "empty" && <li>No notes yet.</li>}
        {state.status === "error" && state.error === null && (
          <li>The notes could not be loaded.</li>
        )}
        {state.notes.map((note) => (
          <li key={note.id}>
            <strong>{note.title}</strong>
            {note.body !== "" && <p>{note.body}</p>}
          </li>
        ))}
      </ul>
    </section>
  );
}
