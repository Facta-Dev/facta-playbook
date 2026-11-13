import Kernel

/// Loads every note. The use case is the only thing the ViewModel calls; it
/// never sees the repository implementation.
public struct LoadNotes: Sendable {
    private let repository: any NoteRepository

    public init(repository: any NoteRepository) {
        self.repository = repository
    }

    public func callAsFunction() async throws -> Outcome<[Note]> {
        do {
            return .success(try await repository.load())
        } catch let error as AppError {
            return .failure(error)
        } catch is CancellationError {
            // Cancellation is structural, not a failure: rethrow it so the
            // caller's task unwinds (AGENTS.md, "Errors are values").
            throw CancellationError()
        } catch {
            return .failure(AppError(code: .unknown))
        }
    }
}
