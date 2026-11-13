import Testing
@testable import Kernel

@Suite("Outcome")
struct OutcomeTests {
    @Test func mapTransformsTheSuccessValue() {
        let outcome: Outcome<Int> = .success(1)
        #expect(outcome.map { $0 + 1 }.value == 2)
    }

    @Test func mapPreservesAFailure() {
        let error = AppError(code: .offline)
        let outcome: Outcome<Int> = .failure(error)
        #expect(outcome.map { $0 + 1 }.error == error)
    }

    @Test func accessorsExposeOneBranchOnly() {
        let success: Outcome<String> = .success("ok")
        #expect(success.value == "ok")
        #expect(success.error == nil)

        let failure: Outcome<String> = .failure(AppError(code: .notFound))
        #expect(failure.value == nil)
        #expect(failure.error?.code == .notFound)
    }
}
