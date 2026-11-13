package dev.<scope>.<app>.feature.notes.domain

import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.core.kernel.Outcome

data class Note(
    val id: NoteId,
    val title: String,
)

/** The domain port. The adapter lives in `:feature:notes:data`. */
interface NoteRepository {
    suspend fun list(): Outcome<List<Note>>
}

/** The use case the ViewModel calls; it depends on the port, not the adapter. */
class LoadNotes(private val repository: NoteRepository) {
    suspend operator fun invoke(): Outcome<List<Note>> = repository.list()
}
