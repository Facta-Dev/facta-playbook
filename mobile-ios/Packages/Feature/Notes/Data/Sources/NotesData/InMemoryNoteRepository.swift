import Kernel
import NotesDomain

/// The template adapter: two fixed notes, no disk, no network. Replace it with
/// the real store; the port and the use case stay unchanged.
public struct InMemoryNoteRepository: NoteRepository {
    public init() {}

    public func load() async throws -> [Note] {
        [
            Note(id: NoteID(rawValue: "note-1"), title: "First note"),
            Note(id: NoteID(rawValue: "note-2"), title: "Second note"),
        ]
    }
}
