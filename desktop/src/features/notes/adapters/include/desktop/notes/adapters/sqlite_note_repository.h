#pragma once

#include "desktop/infrastructure/storage/note_store.h"
#include "desktop/notes/application/note_repository.h"

namespace desktop::notes::adapters {

// The repository over the local projection cache (playbook 9): the server
// holds the truth, this database makes the app usable offline.
class SqliteNoteRepository final : public application::NoteRepository {
public:
    explicit SqliteNoteRepository(infrastructure::storage::NoteStore store);

    [[nodiscard]] kernel::Result<QVector<domain::Note>> list() const override;
    [[nodiscard]] kernel::Result<domain::Note> add(const domain::Note& note) override;

private:
    infrastructure::storage::NoteStore store_;
};

}  // namespace desktop::notes::adapters
