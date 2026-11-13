package dev.<scope>.<app>.core.kernel

import org.junit.Assert.assertEquals
import org.junit.Test

class OutcomeTest {
    @Test
    fun successCarriesTheValue() {
        val outcome: Outcome<Int> = Outcome.Success(1)
        assertEquals(1, (outcome as Outcome.Success).value)
    }

    @Test
    fun failureCarriesTheClosedCode() {
        val outcome: Outcome<Int> = Outcome.Failure(AppError(ErrorCode.NotFound, "missing"))
        assertEquals(ErrorCode.NotFound, (outcome as Outcome.Failure).error.code)
    }
}
