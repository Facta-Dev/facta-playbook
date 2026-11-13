package dev.<scope>.<app>.core.kernel

/** A typed note id. The compiler keeps it apart from every other id. */
@JvmInline
value class NoteId(val value: String)
