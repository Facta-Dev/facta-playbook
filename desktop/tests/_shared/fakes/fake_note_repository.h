#pragma once

// The in-memory repository used by the notes suites: the use cases and the
// view model are checked against a port, never against a database.

#include "desktop/kernel/error_codes.h"
#include "desktop/kernel/result.h"
#include "desktop/notes/application/note_repository.h"
#include "desktop/notes/domain/note.h"

#include <QString>
#include <QVector>

namespace desktop::tests {

class FakeNoteRepository final : public notes::application::NoteRepository {
public:
    kernel::Result<QVector<notes::domain::Note>> list() const override {
        if (listError != kernel::ErrorCode::Unknown) {
            return kernel::Result<QVector<notes::domain::Note>>(listError, listErrorMessage);
        }
        return kernel::Result<QVector<notes::domain::Note>>(notes);
    }

    kernel::Result<notes::domain::Note> add(const notes::domain::Note& note) override {
        if (addError != kernel::ErrorCode::Unknown) {
            return kernel::Result<notes::domain::Note>(addError, addErrorMessage);
        }
        notes.append(note);
        return kernel::Result<notes::domain::Note>(note);
    }

    QVector<notes::domain::Note> notes;

    // The failure the fake injects on the next call of the port.
    kernel::ErrorCode listError = kernel::ErrorCode::Unknown;
    QString listErrorMessage;
    kernel::ErrorCode addError = kernel::ErrorCode::Unknown;
    QString addErrorMessage;
};

}  // namespace desktop::tests
