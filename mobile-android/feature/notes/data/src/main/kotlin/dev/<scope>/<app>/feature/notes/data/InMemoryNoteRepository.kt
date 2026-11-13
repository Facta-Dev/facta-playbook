package dev.<scope>.<app>.feature.notes.data

import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.core.kernel.Outcome
import dev.<scope>.<app>.feature.notes.domain.Note
import dev.<scope>.<app>.feature.notes.domain.NoteRepository

/**
 * The template adapter. Replacing it with Room, a network client or a
 * cache is a `:app` wiring change; the domain never sees the swap.
 */
class InMemoryNoteRepository : NoteRepository {
    private val notes = listOf(
        Note(NoteId("1"), "Welcome to <app>"),
        Note(NoteId("2"), "Your second note"),
    )

    override suspend fun list(): Outcome<List<Note>> = Outcome.Success(notes)
}
