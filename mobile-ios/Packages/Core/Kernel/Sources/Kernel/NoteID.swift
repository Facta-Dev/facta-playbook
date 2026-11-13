/// The identity of a note. A typed value, not a bare `String`: a note id can
/// never be passed where a user id is expected.
public struct NoteID: Hashable, Codable, Sendable, RawRepresentable {
    public let rawValue: String

    public init(rawValue: String) {
        self.rawValue = rawValue
    }
}
