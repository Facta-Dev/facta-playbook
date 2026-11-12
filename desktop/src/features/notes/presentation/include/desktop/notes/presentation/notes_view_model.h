#pragma once

#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/presentation/notes_list_model.h"

#include <QObject>
#include <QString>

namespace desktop::notes::presentation {

// The view model of the notes (playbook 3.3): it exposes the state to the
// view and the commands to the view, and it never knows a widget.
class NotesViewModel : public QObject {
    Q_OBJECT
    Q_PROPERTY(State state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString statusMessage READ statusMessage NOTIFY stateChanged)
    Q_PROPERTY(int count READ count NOTIFY countChanged)

public:
    enum class State {
        Loading,
        Ready,
        Empty,
        Forbidden,
        NotFound,
        Offline,
        Error,
        PolicyBlocked
    };
    Q_ENUM(State)

    NotesViewModel(application::ListNotes listNotes, application::AddNote addNote,
                   QObject* parent = nullptr);

    [[nodiscard]] State state() const noexcept { return state_; }
    [[nodiscard]] QString statusMessage() const noexcept { return statusMessage_; }
    [[nodiscard]] int count() const noexcept { return model_->rowCount(); }

    // The model behind the list of the view.
    [[nodiscard]] NotesListModel* notesModel() const noexcept { return model_; }

    // The commands of the view.
    Q_INVOKABLE void refresh();
    Q_INVOKABLE void addNote(const QString& title, const QString& body);

signals:
    // The notes are ready: the view refreshes what it shows.
    void notesLoaded();

    // Something went wrong: the view shows the message.
    void errorRaised(const QString& message);

    void stateChanged();
    void countChanged();

private:
    void setState(State state, const QString& message);
    void fail(kernel::ErrorCode code, const QString& message);

    application::ListNotes listNotes_;
    application::AddNote addNote_;
    NotesListModel* model_;
    State state_ = State::Loading;
    QString statusMessage_;
};

}  // namespace desktop::notes::presentation
