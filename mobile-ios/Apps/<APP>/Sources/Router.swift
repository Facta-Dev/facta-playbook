import NotesContracts
import Observation

/// The app-owned navigation state. A typed path, not a string table: a route
/// added to `NotesRoute` breaks every switch that must handle it.
@MainActor
@Observable
final class Router {
    var path: [NotesRoute] = []
}
