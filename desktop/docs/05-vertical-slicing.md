# 05 — Vertical Slicing

## What a slice is

A slice is **one capability, cut through every layer**: the domain rule, the
use case, the port, the adapters, the view model, the view and the tests. It
is vertical, not horizontal: the template does not have a `domain/` folder
holding the entities of every feature and a `adapters/` folder holding the
adapters of every feature.

The template ships one slice, `notes`, which is the reference for this
chapter.

## The anatomy of the shipped slice

```
src/features/notes/
├── CMakeLists.txt
├── domain/
│   ├── include/desktop/notes/domain/note.h
│   └── src/note.cpp
├── application/
│   ├── include/desktop/notes/application/note_repository.h   ← the port
│   ├── include/desktop/notes/application/list_notes.h
│   ├── include/desktop/notes/application/add_note.h
│   ├── src/list_notes.cpp
│   └── src/add_note.cpp
├── adapters/
│   ├── include/desktop/notes/adapters/sqlite_note_repository.h
│   ├── include/desktop/notes/adapters/sdk_note_transport.h
│   ├── src/sqlite_note_repository.cpp
│   └── src/sdk_note_transport.cpp
├── presentation/
│   ├── include/desktop/notes/presentation/notes_view_model.h
│   ├── include/desktop/notes/presentation/notes_list_model.h
│   └── src/…
└── tests/tst_notes.cpp
```

Read it as four questions:

| Folder | The question it answers |
| --- | --- |
| `domain/` | What is a note, and what makes one valid? |
| `application/` | What can the app do with notes? What does it need from the outside? |
| `adapters/` | How do notes reach a database or a server? |
| `presentation/` | What does the user see, and what can they do? |

## The rules of a slice

1. **The slice owns its port.** `note_repository.h` is in the slice, in the
   application folder. Nothing else declares an interface for this slice.
2. **The slice owns its domain.** `Note` and its invariants live in the
   slice. The shared kernel holds only what every slice needs: `Result`,
   `ErrorCode`, `StrongId`, `Timestamp`.
3. **The slice owns its tests.** `src/features/notes/tests/tst_notes.cpp`
   sits next to the code it tests, and the cross-layer suites live under
   `tests/features/notes/`.
4. **The slice does not own the app.** The window (`src/app/`) is the
   composition root; it wires the slice's view model to a concrete adapter.
5. **A slice does not import another slice.** Two slices that need each
   other share a kernel type or a contract, not a call. The compile-time
   dependency between two features is a design smell.
6. **The layer folders inside a slice are not the global layers.** The
   global layers (`src/domain`, `src/application`, …) do not exist in this
   template: the slice *is* the layer decomposition, per capability.

## The two adapters of the slice

The slice has two implementations of the same port, and that is the point:

| Adapter | Implements | Talks to |
| --- | --- | --- |
| `SqliteNoteRepository` | `NoteRepository` | `NoteStore` (SQLite, WAL) |
| `SdkNoteTransport` | `NoteRepository` | `desktop::sdk::NotesClient` (HTTP) |

Both satisfy the same interface, so the use cases do not know which one is
bound. The composition root chooses:

```cpp
desktop::notes::adapters::SqliteNoteRepository repository(opened.value());
desktop::notes::application::ListNotes listNotes(repository);
```

Swapping to the SDK adapter is a one-line change in `main.cpp` — and nothing
in the domain, the use cases or the view model changes. That substitutability
is the deliverable of the port; if changing the adapter touched the use
case, the port was in the wrong place.

> **Known gap.** `SdkNoteTransport` is compiled and unit-tested, but the
> composition root binds the SQLite adapter. The application layer is
> therefore currently offline-first with no server path wired.

## Adding a slice

The recipe, in order:

1. **Name the capability.** `attachments`, not `upload`.
2. **Create the folder** `src/features/<name>/` with the four sub-folders.
3. **Write the domain** and its invariants first, with a test that fails.
4. **Declare the port** in `application/` — in domain types.
5. **Write the use cases** against the port, with a fake in the test.
6. **Write the adapter** and its test.
7. **Write the view model** and its test (headless, no widget).
8. **Wire it** in `src/app/main.cpp` — and only there.
9. **Add the CMake target** with the same link rules as `notes`
   ([ADR 0002](adr/0002-one-static-library-per-layer.md)).
10. **Add the suite** to `tests/CMakeLists.txt`.

The full recipe with the commands is in [13 — Playbooks](13-playbooks.md).

## What stays out of a slice

| Belongs elsewhere | Why |
| --- | --- |
| A widget shared by two slices | `src/ui-kit/` — shared, no domain knowledge |
| An HTTP client used by two slices | `packages/sdk-cpp/` |
| A table used by two slices | `src/infrastructure/` — a shared store, if it ever happens |
| A type used by two slices | `src/shared-kernel/` — only if it is genuinely universal |

## Do / don't

| Do | Don't |
| --- | --- |
| Give each slice its own port, adapter and tests | Create a global `repositories/` folder |
| Keep the slice's tests next to the slice | Put every test in one top-level folder |
| Bind the adapter in `main.cpp` | Construct an adapter inside a use case |
| Let two slices share a kernel type | Let slice A include slice B's headers |

## See also

- [Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md)
- [Repository Layout & Project Skeleton](06-repository-layout-and-project-skeleton.md)
- [Playbooks](13-playbooks.md) — the step-by-step recipe.
