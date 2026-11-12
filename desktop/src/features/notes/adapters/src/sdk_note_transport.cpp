#include "desktop/notes/adapters/sdk_note_transport.h"

namespace desktop::notes::adapters {

SdkNoteTransport::SdkNoteTransport(sdk::NotesClient& client) : client_(client) {}

void SdkNoteTransport::fetchNotes(Callback callback) {
    client_.listNotes().then([callback](const kernel::Result<QVector<contracts::NoteDto>>& result) {
        if (!result.ok()) {
            callback(kernel::Result<QVector<domain::Note>>(result.errorCode(),
                                                           result.errorMessage()));
            return;
        }

        QVector<domain::Note> notes;
        notes.reserve(result.value().size());
        for (const contracts::NoteDto& dto : result.value()) {
            const kernel::Result<domain::Note> note = domain::Note::fromDto(dto);
            if (!note.ok()) {
                callback(kernel::Result<QVector<domain::Note>>(note.errorCode(),
                                                               note.errorMessage()));
                return;
            }
            notes.append(note.value());
        }
        callback(kernel::Result<QVector<domain::Note>>(notes));
    });
}

}  // namespace desktop::notes::adapters
