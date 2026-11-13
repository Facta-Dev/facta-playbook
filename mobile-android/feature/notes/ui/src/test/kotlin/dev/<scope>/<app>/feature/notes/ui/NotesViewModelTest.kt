package dev.<scope>.<app>.feature.notes.ui

import dev.<scope>.<app>.core.kernel.ErrorCode
import dev.<scope>.<app>.core.kernel.AppError
import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.core.kernel.Outcome
import dev.<scope>.<app>.core.uicore.UiStatus
import dev.<scope>.<app>.feature.notes.contracts.NotesRoute
import dev.<scope>.<app>.feature.notes.domain.LoadNotes
import dev.<scope>.<app>.feature.notes.domain.Note
import dev.<scope>.<app>.feature.notes.domain.NoteRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class NotesViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(repository: NoteRepository) =
        NotesViewModel(LoadNotes(repository), dispatcher)

    @Test
    fun loadsNotesOnStart() = runTest(dispatcher) {
        val note = Note(NoteId("1"), "first")
        val viewModel = viewModel(object : NoteRepository {
            override suspend fun list() = Outcome.Success(listOf(note))
        })

        assertEquals(UiStatus.Loading, viewModel.state.value.status)
        advanceUntilIdle()
        assertEquals(UiStatus.Ready, viewModel.state.value.status)
        assertEquals(listOf(note), viewModel.state.value.notes)
    }

    @Test
    fun mapsAnEmptyReadToEmpty() = runTest(dispatcher) {
        val viewModel = viewModel(object : NoteRepository {
            override suspend fun list() = Outcome.Success(emptyList<Note>())
        })

        advanceUntilIdle()
        assertEquals(UiStatus.Empty, viewModel.state.value.status)
    }

    @Test
    fun mapsAnOfflineFailureToOffline() = runTest(dispatcher) {
        val viewModel = viewModel(object : NoteRepository {
            override suspend fun list() = Outcome.Failure(AppError(ErrorCode.Offline, "no connection"))
        })

        advanceUntilIdle()
        assertEquals(UiStatus.Offline, viewModel.state.value.status)
    }

    @Test
    fun openEmitsOneNavigationEffect() = runTest(dispatcher) {
        val viewModel = viewModel(object : NoteRepository {
            override suspend fun list() = Outcome.Success(emptyList<Note>())
        })
        advanceUntilIdle()

        viewModel.onEvent(NotesEvent.Open(NoteId("1")))

        assertEquals(NotesRoute.Detail("1"), (viewModel.effects.first() as NotesEffect.NavigateToDetail).route)
    }
}
