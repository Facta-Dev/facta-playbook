package dev.<scope>.<app>.feature.notes.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.<scope>.<app>.core.kernel.AppError
import dev.<scope>.<app>.core.kernel.ErrorCode
import dev.<scope>.<app>.core.kernel.NoteId
import dev.<scope>.<app>.core.kernel.Outcome
import dev.<scope>.<app>.core.uicore.UiStatus
import dev.<scope>.<app>.feature.notes.contracts.NotesRoute
import dev.<scope>.<app>.feature.notes.domain.LoadNotes
import dev.<scope>.<app>.feature.notes.domain.Note
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Everything the screen renders. */
@Immutable
data class NotesState(
    val status: UiStatus = UiStatus.Loading,
    val notes: List<Note> = emptyList(),
) {
    fun note(id: NoteId): Note? = notes.firstOrNull { it.id == id }
}

/** What the user did. */
sealed interface NotesEvent {
    data object Refresh : NotesEvent
    data class Open(val id: NoteId) : NotesEvent
}

/** What the screen must do once, outside its own rendering. */
sealed interface NotesEffect {
    data class NavigateToDetail(val route: NotesRoute.Detail) : NotesEffect
}

class NotesViewModel(
    private val loadNotes: LoadNotes,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val _state = MutableStateFlow(NotesState())
    val state: StateFlow<NotesState> = _state.asStateFlow()

    private val _effects = Channel<NotesEffect>(Channel.BUFFERED)
    val effects: Flow<NotesEffect> = _effects.receiveAsFlow()

    init {
        onEvent(NotesEvent.Refresh)
    }

    fun onEvent(event: NotesEvent) {
        when (event) {
            NotesEvent.Refresh -> refresh()
            is NotesEvent.Open -> _effects.trySend(NotesEffect.NavigateToDetail(NotesRoute.Detail(event.id.value)))
        }
    }

    private fun refresh() {
        viewModelScope.launch {
            _state.value = _state.value.copy(status = UiStatus.Loading)
            _state.value = when (val outcome = withContext(dispatcher) { loadNotes() }) {
                is Outcome.Success -> NotesState(
                    status = if (outcome.value.isEmpty()) UiStatus.Empty else UiStatus.Ready,
                    notes = outcome.value,
                )
                is Outcome.Failure -> NotesState(status = outcome.error.toStatus())
            }
        }
    }
}

private fun AppError.toStatus(): UiStatus = when (code) {
    ErrorCode.NotFound -> UiStatus.NotFound
    ErrorCode.Offline -> UiStatus.Offline
    ErrorCode.Forbidden -> UiStatus.Forbidden
    ErrorCode.PolicyBlocked -> UiStatus.PolicyBlocked
    ErrorCode.Unknown -> UiStatus.Error(correlationId)
}

@Composable
fun NotesScreen(state: NotesState, onEvent: (NotesEvent) -> Unit) {
    when (val status = state.status) {
        UiStatus.Loading -> StatusMessage(stringResource(R.string.notes_loading))
        UiStatus.Ready -> NoteList(state.notes, onEvent)
        UiStatus.Empty -> StatusMessage(stringResource(R.string.notes_empty))
        UiStatus.Offline -> StatusMessage(stringResource(R.string.notes_offline))
        UiStatus.Forbidden -> StatusMessage(stringResource(R.string.notes_forbidden))
        UiStatus.NotFound -> StatusMessage(stringResource(R.string.notes_not_found))
        is UiStatus.Error -> {
            val correlationId = status.correlationId ?: stringResource(R.string.notes_unknown_correlation_id)
            StatusMessage(stringResource(R.string.notes_error, correlationId))
        }
        UiStatus.PolicyBlocked -> StatusMessage(stringResource(R.string.notes_policy_blocked))
    }
}

@Composable
fun NoteDetailScreen(state: NotesState, noteId: NoteId) {
    val note = state.note(noteId)
    if (note == null) {
        StatusMessage(stringResource(R.string.note_detail_not_found))
    } else {
        Text(text = note.title, modifier = Modifier.fillMaxSize().padding(16.dp))
    }
}

@Composable
private fun NoteList(notes: List<Note>, onEvent: (NotesEvent) -> Unit) {
    LazyColumn(modifier = Modifier.fillMaxSize()) {
        items(notes, key = { it.id.value }) { note ->
            Text(
                text = note.title,
                modifier = Modifier
                    .fillMaxSize()
                    .clickable { onEvent(NotesEvent.Open(note.id)) }
                    .padding(16.dp),
            )
        }
    }
}

@Composable
private fun StatusMessage(message: String) {
    Text(text = message, modifier = Modifier.fillMaxSize().padding(16.dp))
}
