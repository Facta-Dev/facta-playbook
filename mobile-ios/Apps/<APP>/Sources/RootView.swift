import Kernel
import NotesContracts
import NotesUI
import SwiftUI

@MainActor
struct RootView: View {
    private let environment: AppEnvironment
    @State private var router = Router()
    @State private var model: NotesModel

    init(environment: AppEnvironment) {
        self.environment = environment
        _model = State(initialValue: environment.makeNotesModel())
    }

    var body: some View {
        NavigationStack(path: $router.path) {
            NotesScreen(model: model) { id in
                router.path.append(.detail(id: id.rawValue))
            }
            .navigationDestination(for: NotesRoute.self) { route in
                switch route {
                case .list:
                    EmptyView()
                case .detail(let id):
                    NoteDetailPlaceholder(id: id)
                }
            }
        }
    }
}

private struct NoteDetailPlaceholder: View {
    let id: String

    var body: some View {
        Text(id)
            .navigationTitle("Note")
    }
}
