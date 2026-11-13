package dev.<scope>.<app>.feature.notes.domain

import dev.<scope>.<app>.core.kernel.AppError
import dev.<scope>.<app>.core.kernel.ErrorCode
import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.core.kernel.Outcome
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LoadNotesTest {
    @Test
    fun returnsTheRepositoryNotes() = runBlocking {
        val note = Note(NoteId("1"), "first")
        val useCase = LoadNotes(object : NoteRepository {
            override suspend fun list(): Outcome<List<Note>> = Outcome.Success(listOf(note))
        })

        assertEquals(Outcome.Success(listOf(note)), useCase())
    }

    @Test
    fun propagatesThePortFailure() = runBlocking {
        val error = AppError(ErrorCode.Offline, "no connection")
        val useCase = LoadNotes(object : NoteRepository {
            override suspend fun list(): Outcome<List<Note>> = Outcome.Failure(error)
        })

        val outcome = useCase()
        assertTrue(outcome is Outcome.Failure && outcome.error == error)
    }
}
