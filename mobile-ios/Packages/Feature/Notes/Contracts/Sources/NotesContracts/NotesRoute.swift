/// The typed routes the notes slice publishes to the rest of the app.
///
/// A route is a value, never a string path: the compiler checks every
/// `navigationDestination` switch, and a renamed case breaks the app at build
/// time instead of at runtime.
public enum NotesRoute: Hashable, Sendable {
    case list
    case detail(id: String)
}
