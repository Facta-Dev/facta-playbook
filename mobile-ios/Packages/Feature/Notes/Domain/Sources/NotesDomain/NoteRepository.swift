/// The port the use case needs. Consumer-defined: it lives next to the use
/// case, and the adapter in `NotesData` satisfies it.
public protocol NoteRepository: Sendable {
    func load() async throws -> [Note]
}
