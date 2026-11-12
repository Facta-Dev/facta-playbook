#include "desktop/notes/adapters/sqlite_note_repository.h"

#include <utility>

namespace desktop::notes::adapters {

SqliteNoteRepository::SqliteNoteRepository(infrastructure::storage::NoteStore store)
    : store_(std::move(store)) {}

kernel::Result<QVector<domain::Note>> SqliteNoteRepository::list() const {
    const kernel::Result<QVector<contracts::NoteDto>> stored = store_.listNotes();
    if (!stored.ok()) {
        return kernel::Result<QVector<domain::Note>>(stored.errorCode(), stored.errorMessage());
    }

    QVector<domain::Note> notes;
    notes.reserve(stored.value().size());
    for (const contracts::NoteDto& dto : stored.value()) {
        const kernel::Result<domain::Note> note = domain::Note::fromDto(dto);
        if (!note.ok()) {
            return kernel::Result<QVector<domain::Note>>(note.errorCode(), note.errorMessage());
        }
        notes.append(note.value());
    }
    return kernel::Result<QVector<domain::Note>>(notes);
}

kernel::Result<domain::Note> SqliteNoteRepository::add(const domain::Note& note) {
    const kernel::Result<contracts::NoteDto> stored = store_.upsertNote(note.toDto());
    if (!stored.ok()) {
        return kernel::Result<domain::Note>(stored.errorCode(), stored.errorMessage());
    }
    return domain::Note::fromDto(stored.value());
}

}  // namespace desktop::notes::adapters
