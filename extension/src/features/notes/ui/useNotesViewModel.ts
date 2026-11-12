// <app> — the React binding of the notes view model.
//
// The view model is framework-agnostic (src/features/notes/viewmodels); this
// hook is the only place where React enters the slice.

import { useCallback, useSyncExternalStore } from "react";
import { type NotesViewModel } from "../viewmodels/notes-view-model";

export function useNotesViewModel(viewModel: NotesViewModel) {
  const state = useSyncExternalStore(viewModel.subscribe, viewModel.getState, viewModel.getState);

  const load = useCallback(() => {
    void viewModel.load();
  }, [viewModel]);

  const submit = useCallback(() => {
    void viewModel.submit();
  }, [viewModel]);

  const setTitle = useCallback(
    (title: string) => {
      viewModel.setTitle(title);
    },
    [viewModel],
  );

  const setBody = useCallback(
    (body: string) => {
      viewModel.setBody(body);
    },
    [viewModel],
  );

  return { state, load, submit, setTitle, setBody };
}
