package dev.<scope>.<app>.core.uicore

/**
 * The closed status set every screen renders. One value, never a set of
 * booleans that can contradict each other.
 */
sealed interface UiStatus {
    data object Loading : UiStatus
    data object Ready : UiStatus
    data object Empty : UiStatus
    data object Offline : UiStatus
    data object Forbidden : UiStatus
    data object NotFound : UiStatus
    data class Error(val correlationId: String? = null) : UiStatus
    data object PolicyBlocked : UiStatus
}
