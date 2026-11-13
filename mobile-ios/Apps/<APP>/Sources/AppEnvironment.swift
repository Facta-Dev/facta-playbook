import NotesData
import NotesDomain
import NotesUI

/// The composition root: it picks the adapter and hands the use case to the
/// view model. Nothing below the app knows which implementation is live.
@MainActor
struct AppEnvironment {
    let makeNotesModel: @MainActor () -> NotesModel

    static let live = AppEnvironment {
        NotesModel(loadNotes: LoadNotes(repository: InMemoryNoteRepository()))
    }
}
