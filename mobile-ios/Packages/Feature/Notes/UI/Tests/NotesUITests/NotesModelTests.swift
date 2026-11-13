import Kernel
import NotesDomain
import Testing
import UICore
@testable import NotesUI

private struct StubRepository: NoteRepository {
    let notes: [Note]

    func load() async throws -> [Note] {
        notes
    }
}

private struct FailingRepository: NoteRepository {
    let result: @Sendable () async throws -> [Note]

    func load() async throws -> [Note] {
        try await result()
    }
}

@MainActor
@Suite("NotesModel")
struct NotesModelTests {
    private func makeModel(
        notes: [Note] = [Note(id: NoteID(rawValue: "note-1"), title: "First")]
    ) -> NotesModel {
        NotesModel(loadNotes: LoadNotes(repository: StubRepository(notes: notes)))
    }

    @Test func mapsANonEmptyListToReady() async {
        let model = makeModel()

        await model.refresh()

        #expect(model.state.status == .ready)
        #expect(model.state.notes.map(\.title) == ["First"])
    }

    @Test func mapsAnEmptyListToEmpty() async {
        let model = makeModel(notes: [])

        await model.refresh()

        #expect(model.state.status == .empty)
    }

    @Test func mapsAPolicyErrorToPolicyBlocked() async {
        let model = NotesModel(loadNotes: LoadNotes(repository: FailingRepository {
            throw AppError(code: .policy)
        }))

        await model.refresh()

        #expect(model.state.status == .policyBlocked)
    }

    @Test func aCancelledRefreshShowsNoFailure() async {
        let model = NotesModel(loadNotes: LoadNotes(repository: FailingRepository {
            throw CancellationError()
        }))

        await model.refresh()

        #expect(model.state.status == .loading)
    }

    @Test func openYieldsExactlyOneEffect() async {
        let model = makeModel()
        var iterator = model.effects.makeAsyncIterator()

        model.send(.open(NoteID(rawValue: "note-1")))

        #expect(await iterator.next() == .openDetail(NoteID(rawValue: "note-1")))
    }
}
