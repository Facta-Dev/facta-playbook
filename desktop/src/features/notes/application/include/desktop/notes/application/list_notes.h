#pragma once

#include "desktop/notes/application/note_repository.h"

namespace desktop::notes::application {

// The use case of the list of notes.
class ListNotes {
public:
    explicit ListNotes(const NoteRepository& repository);

    [[nodiscard]] kernel::Result<QVector<domain::Note>> execute() const;

private:
    const NoteRepository& repository_;
};

}  // namespace desktop::notes::application
