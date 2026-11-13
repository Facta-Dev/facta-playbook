/// The closed set of failure codes the app maps to user-visible states.
public enum ErrorCode: String, CaseIterable, Sendable {
    case offline
    case forbidden
    case policy
    case notFound
    case conflict
    case validation
    case server
    case unknown
}

/// An expected failure: a stable code plus the server correlation id when one
/// exists. Never carries a user-visible message — copy lives in the UI layer.
public struct AppError: Error, Equatable, Sendable {
    public let code: ErrorCode
    public let correlationID: String?

    public init(code: ErrorCode, correlationID: String? = nil) {
        self.code = code
        self.correlationID = correlationID
    }
}
