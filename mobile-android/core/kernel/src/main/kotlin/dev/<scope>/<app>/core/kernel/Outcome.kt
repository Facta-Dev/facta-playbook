package dev.<scope>.<app>.core.kernel

/** The closed set of expected failure codes the app understands. */
enum class ErrorCode {
    NotFound,
    Offline,
    Forbidden,
    PolicyBlocked,
    Unknown,
}

/** An expected failure carried as a value, never as a thrown exception. */
data class AppError(
    val code: ErrorCode,
    val message: String,
    val correlationId: String? = null,
)

/** The result of an operation that can fail in an expected way. */
sealed interface Outcome<out T> {
    data class Success<T>(val value: T) : Outcome<T>
    data class Failure(val error: AppError) : Outcome<Nothing>
}
