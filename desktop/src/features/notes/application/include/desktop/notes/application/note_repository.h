#pragma once

#include "desktop/kernel/result.h"
#include "desktop/notes/domain/note.h"

#include <QVector>

namespace desktop::notes::application {

// The port of the notes storage (playbook 3.2): the use cases talk to this
// interface, the adapters implement it. Errors cross the port as values.
class NoteRepository {
public:
    virtual ~NoteRepository() = default;

    // Every note of the local projection.
    [[nodiscard]] virtual kernel::Result<QVector<domain::Note>> list() const = 0;

    // Stores a note and returns the stored one.
    [[nodiscard]] virtual kernel::Result<domain::Note> add(const domain::Note& note) = 0;
};

}  // namespace desktop::notes::application
