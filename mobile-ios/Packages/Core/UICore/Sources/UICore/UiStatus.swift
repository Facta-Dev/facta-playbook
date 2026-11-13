/// The closed status set every screen renders. A `switch` that misses a case
/// does not compile, so a new status cannot ship without a screen decision.
public enum UiStatus: Equatable, Sendable {
    case loading
    case ready
    case empty
    case offline
    case forbidden
    case notFound
    case error(correlationID: String?)
    case policyBlocked
}
