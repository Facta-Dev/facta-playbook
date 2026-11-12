#include "desktop/notes/application/list_notes.h"

namespace desktop::notes::application {

ListNotes::ListNotes(const NoteRepository& repository) : repository_(repository) {}

kernel::Result<QVector<domain::Note>> ListNotes::execute() const {
    return repository_.list();
}

}  // namespace desktop::notes::application
