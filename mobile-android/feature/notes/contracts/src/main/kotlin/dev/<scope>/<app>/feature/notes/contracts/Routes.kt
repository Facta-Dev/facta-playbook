package dev.<scope>.<app>.feature.notes.contracts

import androidx.navigation3.runtime.NavKey
import kotlinx.serialization.Serializable

/**
 * The typed route table of the notes slice. Every destination is a
 * [NavKey]; navigation never passes a string.
 */
@Serializable
sealed interface NotesRoute : NavKey {
    @Serializable
    data object List : NotesRoute

    @Serializable
    data class Detail(val id: String) : NotesRoute
}
