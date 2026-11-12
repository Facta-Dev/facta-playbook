#include "desktop/notes/application/add_note.h"

#include "desktop/kernel/time.h"

#include <QUuid>
#include <utility>

namespace desktop::notes::application {

AddNote::AddNote(NoteRepository& repository) : repository_(repository) {}

kernel::Result<domain::Note> AddNote::execute(const QString& title, const QString& body) const {
    const kernel::NoteId id(QUuid::createUuid().toString(QUuid::WithoutBraces));
    const kernel::Timestamp now = kernel::Timestamp::now();

    const kernel::Result<domain::Note> note =
        domain::Note::create(id, title, body, now, now);
    if (!note.ok()) {
        return kernel::Result<domain::Note>(note.errorCode(), note.errorMessage());
    }

    return repository_.add(note.value());
}

}  // namespace desktop::notes::application
