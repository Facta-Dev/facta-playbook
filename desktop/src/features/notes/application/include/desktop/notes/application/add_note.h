#pragma once

#include "desktop/notes/application/note_repository.h"

#include <QString>

namespace desktop::notes::application {

// The use case of a new note: the domain validates it, the repository stores
// it.
class AddNote {
public:
    explicit AddNote(NoteRepository& repository);

    [[nodiscard]] kernel::Result<domain::Note> execute(const QString& title,
                                                       const QString& body) const;

private:
    NoteRepository& repository_;
};

}  // namespace desktop::notes::application
