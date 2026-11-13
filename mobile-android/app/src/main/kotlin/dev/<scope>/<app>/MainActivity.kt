package dev.<scope>.<app>

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.runtime.rememberNavBackStack
import androidx.navigation3.ui.NavDisplay
import androidx.savedstate.serialization.SavedStateConfiguration
import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.feature.notes.contracts.NotesRoute
import dev.<scope>.<app>.feature.notes.data.InMemoryNoteRepository
import dev.<scope>.<app>.feature.notes.domain.LoadNotes
import dev.<scope>.<app>.feature.notes.ui.NoteDetailScreen
import dev.<scope>.<app>.feature.notes.ui.NotesEffect
import dev.<scope>.<app>.feature.notes.ui.NotesScreen
import dev.<scope>.<app>.feature.notes.ui.NotesViewModel
import kotlinx.serialization.modules.SerializersModule
import kotlinx.serialization.modules.polymorphic
import kotlinx.serialization.modules.subclass

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                App()
            }
        }
    }
}

/**
 * The composition root: the one place that chooses an adapter, builds the
 * view model, owns the back stack and maps routes to screens.
 */
@Composable
fun App() {
    val backStack = rememberNavBackStack(
        SavedStateConfiguration {
            serializersModule = SerializersModule {
                polymorphic(NavKey::class) {
                    subclass(NotesRoute.List::class, NotesRoute.List.serializer())
                    subclass(NotesRoute.Detail::class, NotesRoute.Detail.serializer())
                }
            }
        },
        NotesRoute.List,
    )
    val viewModel: NotesViewModel = viewModel {
        NotesViewModel(LoadNotes(InMemoryNoteRepository()))
    }
    val state by viewModel.state.collectAsStateWithLifecycle()

    LaunchedEffect(viewModel) {
        viewModel.effects.collect { effect ->
            when (effect) {
                is NotesEffect.NavigateToDetail -> backStack.add(effect.route)
            }
        }
    }

    NavDisplay(
        backStack = backStack,
        onBack = { backStack.removeLastOrNull() },
        entryProvider = entryProvider {
            entry<NotesRoute.List> {
                NotesScreen(state = state, onEvent = viewModel::onEvent)
            }
            entry<NotesRoute.Detail> { route ->
                NoteDetailScreen(state = state, noteId = NoteId(route.id))
            }
        },
    )
}
