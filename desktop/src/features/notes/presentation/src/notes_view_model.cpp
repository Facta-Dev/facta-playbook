#include "desktop/notes/presentation/notes_view_model.h"

#include "desktop/kernel/error_codes.h"

#include <utility>

namespace desktop::notes::presentation {

NotesViewModel::NotesViewModel(application::ListNotes listNotes, application::AddNote addNote,
                               QObject* parent)
    : QObject(parent),
      listNotes_(std::move(listNotes)),
      addNote_(std::move(addNote)),
      model_(new NotesListModel(this)) {}

void NotesViewModel::refresh() {
    setState(State::Loading, QStringLiteral("Loading the notes"));

    const kernel::Result<QVector<domain::Note>> listed = listNotes_.execute();
    if (!listed.ok()) {
        fail(listed.errorCode(), listed.errorMessage());
        return;
    }

    model_->setNotes(listed.value());
    setState(model_->rowCount() == 0 ? State::Empty : State::Ready, QString());
    emit countChanged();
    emit notesLoaded();
}

void NotesViewModel::addNote(const QString& title, const QString& body) {
    const kernel::Result<domain::Note> added = addNote_.execute(title, body);
    if (!added.ok()) {
        fail(added.errorCode(), added.errorMessage());
        return;
    }
    refresh();
}

void NotesViewModel::setState(State state, const QString& message) {
    if (state_ == state && statusMessage_ == message) {
        return;
    }
    state_ = state;
    statusMessage_ = message;
    emit stateChanged();
}

void NotesViewModel::fail(kernel::ErrorCode code, const QString& message) {
    switch (code) {
    case kernel::ErrorCode::PermissionDenied:
        setState(State::Forbidden, message);
        break;
    case kernel::ErrorCode::NotFound:
        setState(State::NotFound, message);
        break;
    case kernel::ErrorCode::Network:
        setState(State::Offline, message);
        break;
    default:
        setState(State::Error, message);
        break;
    }
    emit errorRaised(message);
}

}  // namespace desktop::notes::presentation
