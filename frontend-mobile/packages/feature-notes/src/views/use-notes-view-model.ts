// <app> — the React binding of the notes view model.
//
// The view model is framework-free (src/viewmodels); this hook is the only
// place where React enters the slice.

import { useCallback, useSyncExternalStore } from "react";
import type { NotesViewModel } from "../viewmodels/notes-view-model";

/** Subscribes a React tree to a notes view model. */
export function useNotesViewModel(viewModel: NotesViewModel) {
  const state = useSyncExternalStore(viewModel.subscribe, viewModel.getState, viewModel.getState);

  const load = useCallback(() => {
    void viewModel.load();
  }, [viewModel]);

  const add = useCallback(
    (title: string, body: string) => {
      void viewModel.add({ title, body });
    },
    [viewModel],
  );

  return { state, load, add };
}
