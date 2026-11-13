import Kernel
import Testing
@testable import NotesDomain

private struct StubRepository: NoteRepository {
    let result: @Sendable () async throws -> [Note]

    func load() async throws -> [Note] {
        try await result()
    }
}

@Suite("LoadNotes")
struct LoadNotesTests {
    @Test func returnsTheRepositoryNotes() async {
        let note = Note(id: NoteID(rawValue: "note-1"), title: "First")
        let useCase = LoadNotes(repository: StubRepository { [note] })

        let outcome = try await useCase()

        #expect(outcome.value == [note])
    }

    @Test func preservesAnExpectedAppError() async {
        let useCase = LoadNotes(repository: StubRepository {
            throw AppError(code: .offline, correlationID: "corr-1")
        })

        let outcome = try await useCase()

        #expect(outcome.error == AppError(code: .offline, correlationID: "corr-1"))
    }

    @Test func mapsAnUnexpectedErrorToUnknown() async {
        struct Boom: Error {}
        let useCase = LoadNotes(repository: StubRepository { throw Boom() })

        let outcome = try? await useCase()

        #expect(outcome?.error?.code == .unknown)
    }

    @Test func rethrowsACancellation() async {
        let useCase = LoadNotes(repository: StubRepository {
            throw CancellationError()
        })

        await #expect(throws: CancellationError.self) {
            try await useCase()
        }
    }
}
