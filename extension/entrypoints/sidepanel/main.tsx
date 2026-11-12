// <app> — side panel composition root.
//
// The panel mounts the notes UI and wires the view model to the repository.
// Everything else lives in src/features/notes (PLAYBOOK §3.4). Reads and
// writes go through the service worker over the message protocol: the worker
// is the only writer of the stored record (PLAYBOOK §7.1).
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  createNotesViewModel,
  createRpcNoteRepository,
  NotesPanel,
} from "../../src/features/notes";
import { createRpcClient } from "../../src/messaging/client";
import { createRuntime } from "../../src/platform/runtime";

const container = document.getElementById("root");
if (!container) {
  throw new Error("side panel root element is missing");
}

const repository = createRpcNoteRepository(createRpcClient(createRuntime()));

createRoot(container).render(
  <StrictMode>
    <NotesPanel viewModel={createNotesViewModel(repository)} />
  </StrictMode>,
);
