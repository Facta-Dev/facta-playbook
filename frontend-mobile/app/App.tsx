// <app> — the composition root.
//
// The one place that knows every layer: it builds the adapter, hands it to
// the view model and renders the view. Nothing below this file imports an
// adapter, and nothing above it exists.

import { useEffect } from "react";
import { StatusBar, StyleSheet, View } from "react-native";
import {
  NotesScreen,
  createInMemoryNotesRepository,
  createNotesViewModel,
  useNotesViewModel,
} from "@<scope>/feature-notes";

let counter = 0;

/** A fresh identifier for a note. */
function newNoteId(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  counter += 1;
  return `note_${Date.now().toString(36)}_${counter.toString(36)}`;
}

const viewModel = createNotesViewModel({
  repository: createInMemoryNotesRepository(),
  newId: newNoteId,
  now: () => new Date().toISOString(),
});

export default function App() {
  const { state, load, add } = useNotesViewModel(viewModel);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <View style={styles.root}>
      <StatusBar barStyle="dark-content" />
      <NotesScreen state={state} onRefresh={load} onAdd={add} />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#ffffff" },
});
