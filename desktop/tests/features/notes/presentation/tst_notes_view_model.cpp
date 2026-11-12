// The view model of the notes: the states and the commands, driven by the
// signals it emits. No widget is created here.

#include "desktop/notes/presentation/notes_view_model.h"

#include "desktop/kernel/ids.h"
#include "desktop/kernel/time.h"
#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/domain/note.h"

#include "fake_note_repository.h"

#include <QSignalSpy>
#include <QTest>
#include <QVector>

namespace {

using desktop::kernel::ErrorCode;
using desktop::kernel::NoteId;
using desktop::kernel::Timestamp;
using desktop::notes::application::AddNote;
using desktop::notes::application::ListNotes;
using desktop::notes::domain::Note;
using desktop::notes::presentation::NotesViewModel;
using desktop::tests::FakeNoteRepository;

// The repository and the view model that talks to it.
struct Fixture {
    FakeNoteRepository repository;
    NotesViewModel viewModel;

    Fixture() : viewModel(ListNotes(repository), AddNote(repository)) {}

    void addStoredNote(const QString& title) {
        const Note note =
            Note::create(NoteId(QStringLiteral("n-1")), title, QStringLiteral("a body"),
                         Timestamp::now(), Timestamp::now())
                .value();
        repository.notes.append(note);
    }
};

}  // namespace

class NotesViewModelTests : public QObject {
    Q_OBJECT

private slots:
    void startsInTheLoadingState();
    void refreshMovesToTheEmptyState();
    void refreshMovesToTheReadyState();
    void refreshSignalsTheLoadedNotes();
    void addNoteStoresTheNoteAndReloadsTheList();
    void addNoteWithAnInvalidTitleFails();
    void permissionDeniedMovesToTheForbiddenState();
    void aNetworkFailureMovesToTheOfflineState();
    void aMissingResourceMovesToTheNotFoundState();
    void anUnexpectedFailureMovesToTheErrorState();
    void everyStateTransitionIsSignalled();
};

void NotesViewModelTests::startsInTheLoadingState() {
    const Fixture fixture;

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Loading);
    QCOMPARE(fixture.viewModel.count(), 0);
}

void NotesViewModelTests::refreshMovesToTheEmptyState() {
    Fixture fixture;

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Empty);
    QCOMPARE(fixture.viewModel.count(), 0);
}

void NotesViewModelTests::refreshMovesToTheReadyState() {
    Fixture fixture;
    fixture.addStoredNote(QStringLiteral("a title"));

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Ready);
    QCOMPARE(fixture.viewModel.count(), 1);
}

void NotesViewModelTests::refreshSignalsTheLoadedNotes() {
    Fixture fixture;
    QSignalSpy loaded(&fixture.viewModel, &NotesViewModel::notesLoaded);
    QSignalSpy counted(&fixture.viewModel, &NotesViewModel::countChanged);

    fixture.viewModel.refresh();

    QCOMPARE(loaded.count(), 1);
    QCOMPARE(counted.count(), 1);
}

void NotesViewModelTests::addNoteStoresTheNoteAndReloadsTheList() {
    Fixture fixture;
    QSignalSpy loaded(&fixture.viewModel, &NotesViewModel::notesLoaded);

    fixture.viewModel.addNote(QStringLiteral("a title"), QStringLiteral("a body"));

    QCOMPARE(fixture.repository.notes.size(), 1);
    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Ready);
    QCOMPARE(fixture.viewModel.count(), 1);
    QCOMPARE(loaded.count(), 1);
}

void NotesViewModelTests::addNoteWithAnInvalidTitleFails() {
    Fixture fixture;
    QSignalSpy failed(&fixture.viewModel, &NotesViewModel::errorRaised);

    fixture.viewModel.addNote(QString(), QStringLiteral("a body"));

    QCOMPARE(failed.count(), 1);
    QCOMPARE(failed.first().at(0).toString(), QStringLiteral("the title is empty"));
    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Error);
    QCOMPARE(fixture.repository.notes.size(), 0);
}

void NotesViewModelTests::permissionDeniedMovesToTheForbiddenState() {
    Fixture fixture;
    fixture.repository.listError = ErrorCode::PermissionDenied;
    fixture.repository.listErrorMessage = QStringLiteral("notes.read is missing");
    QSignalSpy failed(&fixture.viewModel, &NotesViewModel::errorRaised);

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Forbidden);
    QCOMPARE(failed.count(), 1);
    QCOMPARE(failed.first().at(0).toString(), QStringLiteral("notes.read is missing"));
}

void NotesViewModelTests::aNetworkFailureMovesToTheOfflineState() {
    Fixture fixture;
    fixture.repository.listError = ErrorCode::Network;

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Offline);
}

void NotesViewModelTests::aMissingResourceMovesToTheNotFoundState() {
    Fixture fixture;
    fixture.repository.listError = ErrorCode::NotFound;

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::NotFound);
}

void NotesViewModelTests::anUnexpectedFailureMovesToTheErrorState() {
    Fixture fixture;
    fixture.repository.listError = ErrorCode::Storage;

    fixture.viewModel.refresh();

    QCOMPARE(fixture.viewModel.state(), NotesViewModel::State::Error);
}

void NotesViewModelTests::everyStateTransitionIsSignalled() {
    Fixture fixture;
    QVector<NotesViewModel::State> states;
    connect(&fixture.viewModel, &NotesViewModel::stateChanged, &fixture.viewModel,
            [&states, &fixture] { states.append(fixture.viewModel.state()); });

    fixture.viewModel.refresh();

    QCOMPARE(states.size(), 2);
    QCOMPARE(states.at(0), NotesViewModel::State::Loading);
    QCOMPARE(states.at(1), NotesViewModel::State::Empty);
}

QTEST_MAIN(NotesViewModelTests)
#include "tst_notes_view_model.moc"
