// <app> — the notes screen.
//
// A React Native view. It renders the view-model state and calls the
// view-model commands: it never imports a repository, an adapter or a
// native module. The strings are English literals; a slice with
// translations moves them to the localization keys of the app.

import { useState } from "react";
import { FlatList, Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import type { NotesViewState } from "../viewmodels/notes-view-model";

export interface NotesScreenProps {
  /** The state to render. */
  readonly state: NotesViewState;
  /** Called when the user asks for the notes to be reloaded. */
  readonly onRefresh: () => void;
  /** Called with the draft the user typed. */
  readonly onAdd: (title: string, body: string) => void;
}

/** The notes screen. */
export function NotesScreen({ state, onRefresh, onAdd }: NotesScreenProps) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");

  const submit = (): void => {
    onAdd(title, body);
    setTitle("");
    setBody("");
  };

  return (
    <View style={styles.screen}>
      <Text style={styles.heading} accessibilityRole="header">
        Notes
      </Text>

      <TextInput
        style={styles.input}
        value={title}
        onChangeText={setTitle}
        placeholder="Title"
        accessibilityLabel="Note title"
        maxLength={120}
      />
      <TextInput
        style={styles.input}
        value={body}
        onChangeText={setBody}
        placeholder="Note"
        accessibilityLabel="Note body"
        multiline
        maxLength={10_000}
      />
      <Pressable
        style={styles.button}
        onPress={submit}
        accessibilityRole="button"
        accessibilityLabel="Add note"
      >
        <Text style={styles.buttonLabel}>Add note</Text>
      </Pressable>

      {state.status === "loading" ? (
        <Text style={styles.status}>Loading…</Text>
      ) : state.status === "empty" ? (
        <Text style={styles.status}>No notes yet.</Text>
      ) : state.status === "error" ? (
        <Text style={styles.status}>{state.errorMessage ?? "Something went wrong."}</Text>
      ) : (
        <FlatList
          data={state.notes}
          keyExtractor={(item) => item.id}
          renderItem={({ item }) => (
            <View style={styles.note}>
              <Text style={styles.noteTitle}>{item.title}</Text>
              <Text style={styles.noteBody}>{item.body}</Text>
            </View>
          )}
        />
      )}

      <Pressable
        style={styles.button}
        onPress={onRefresh}
        accessibilityRole="button"
        accessibilityLabel="Reload notes"
      >
        <Text style={styles.buttonLabel}>Reload</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, padding: 16, gap: 8 },
  heading: { fontSize: 24, fontWeight: "600" },
  input: { borderWidth: 1, borderColor: "#c7c7cc", borderRadius: 8, padding: 12 },
  button: { backgroundColor: "#0a7ea4", borderRadius: 8, padding: 12 },
  buttonLabel: { color: "#ffffff", textAlign: "center", fontWeight: "600" },
  status: { paddingVertical: 12 },
  note: { paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: "#e5e5ea" },
  noteTitle: { fontSize: 16, fontWeight: "600" },
  noteBody: { fontSize: 14 },
});
