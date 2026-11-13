import Kernel

/// A note as the domain understands it: an identity and a title. No transport
/// shape, no persistence attribute, no SwiftUI type.
public struct Note: Identifiable, Equatable, Sendable {
    public let id: NoteID
    public let title: String

    public init(id: NoteID, title: String) {
        self.id = id
        self.title = title
    }
}
