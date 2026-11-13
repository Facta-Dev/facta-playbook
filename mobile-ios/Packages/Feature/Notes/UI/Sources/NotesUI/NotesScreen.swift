import Kernel
import NotesDomain
import SwiftUI
import UICore

/// The notes list. A pure function of `NotesModel.state`: every status has a
/// view, and every user action becomes an event. No repository, no network,
/// no business rule lives here.
public struct NotesScreen: View {
    private let model: NotesModel
    private let onOpen: (NoteID) -> Void

    public init(model: NotesModel, onOpen: @escaping (NoteID) -> Void) {
        self.model = model
        self.onOpen = onOpen
    }

    public var body: some View {
        content
            .navigationTitle("Notes")
            .task { model.send(.refresh) }
            .task {
                for await effect in model.effects {
                    if case .openDetail(let id) = effect {
                        onOpen(id)
                    }
                }
            }
    }

    @ViewBuilder
    private var content: some View {
        switch model.state.status {
        case .loading:
            ProgressView("Loading notes")
        case .ready:
            list
        case .empty:
            ContentUnavailableView("No notes", systemImage: "note.text")
        case .offline:
            ContentUnavailableView("Offline", systemImage: "wifi.slash")
        case .forbidden:
            ContentUnavailableView("Forbidden", systemImage: "lock")
        case .notFound:
            ContentUnavailableView("Not found", systemImage: "questionmark")
        case .error(let correlationID):
            ContentUnavailableView(
                "Something went wrong",
                systemImage: "exclamationmark.triangle",
                description: Text(correlationID ?? "No correlation id")
            )
        case .policyBlocked:
            ContentUnavailableView("Blocked by policy", systemImage: "hand.raised")
        }
    }

    private var list: some View {
        List(model.state.notes) { note in
            Button(note.title) { model.send(.open(note.id)) }
        }
    }
}
