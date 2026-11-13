import Kernel
import NotesDomain
import Observation
import UICore

/// What the user can do on the list screen.
public enum NotesEvent: Equatable, Sendable {
    case refresh
    case open(NoteID)
}

/// What leaves the screen exactly once: a navigation request.
public enum NotesEffect: Equatable, Sendable {
    case openDetail(NoteID)
}

/// The immutable value the screen renders. Status is a closed set; the notes
/// array is never `nil` to stand in for a missing status.
public struct NotesViewState: Equatable, Sendable {
    public var status: UiStatus
    public var notes: [Note]

    public init(status: UiStatus = .loading, notes: [Note] = []) {
        self.status = status
        self.notes = notes
    }
}

/// The list screen's view model: state flows down, events flow up, effects
/// leave through a stream consumed once by the host.
@MainActor
@Observable
public final class NotesModel {
    public private(set) var state: NotesViewState
    public let effects: AsyncStream<NotesEffect>

    private let loadNotes: LoadNotes
    private let continuation: AsyncStream<NotesEffect>.Continuation

    public init(loadNotes: LoadNotes) {
        self.state = NotesViewState()
        self.loadNotes = loadNotes
        let (stream, continuation) = AsyncStream<NotesEffect>.makeStream()
        self.effects = stream
        self.continuation = continuation
    }

    public func send(_ event: NotesEvent) {
        switch event {
        case .refresh:
            Task { await refresh() }
        case .open(let id):
            continuation.yield(.openDetail(id))
        }
    }

    public func refresh() async {
        state.status = .loading
        do {
            switch try await loadNotes() {
            case .success(let notes):
                state.notes = notes
                state.status = notes.isEmpty ? .empty : .ready
            case .failure(let error):
                state.status = Self.status(for: error)
            }
        } catch {
            // The use case only throws a cancellation: the task was cancelled,
            // so the screen must not show a failure.
        }
    }

    private static func status(for error: AppError) -> UiStatus {
        switch error.code {
        case .offline: .offline
        case .forbidden: .forbidden
        case .policy: .policyBlocked
        case .notFound: .notFound
        case .conflict, .validation, .server, .unknown:
            .error(correlationID: error.correlationID)
        }
    }
}
