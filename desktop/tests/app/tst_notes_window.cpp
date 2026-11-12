// The notes window: the widget that shows the list of the view model. The
// window is built and shown headless (offscreen platform).

#include "desktop/app/notes_window.h"

#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/presentation/notes_view_model.h"

#include "fake_note_repository.h"

#include <QLabel>
#include <QListView>
#include <QTest>

namespace {

using desktop::notes::application::AddNote;
using desktop::notes::application::ListNotes;
using desktop::notes::presentation::NotesViewModel;
using desktop::tests::FakeNoteRepository;

// The view model and the window, wired as the composition root wires them.
struct Fixture {
    FakeNoteRepository repository;
    NotesViewModel viewModel;
    desktop::app::NotesWindow window;

    Fixture() : viewModel(ListNotes(repository), AddNote(repository)), window(&viewModel) {
        QObject::connect(&viewModel, &NotesViewModel::notesLoaded, &window,
                         &desktop::app::NotesWindow::onNotesLoaded);
        QObject::connect(&viewModel, &NotesViewModel::errorRaised, &window,
                         &desktop::app::NotesWindow::onErrorRaised);
        window.show();
    }
};

}  // namespace

class NotesWindowTests : public QObject {
    Q_OBJECT

private slots:
    void showsTheEmptyStateWithoutNotes();
    void listsTheNoteAddedThroughTheViewModel();
    void hidesTheEmptyStateWhenThereAreNotes();
    void showsTheErrorOfTheViewModel();
};

void NotesWindowTests::showsTheEmptyStateWithoutNotes() {
    Fixture fixture;

    fixture.viewModel.refresh();

    QListView* list = fixture.window.findChild<QListView*>();
    QVERIFY(list != nullptr);
    QCOMPARE(list->model()->rowCount(), 0);

    QLabel* emptyState = fixture.window.findChild<QLabel*>("uiKitEmptyState");
    QVERIFY(emptyState != nullptr);
    QVERIFY(emptyState->isVisible());
}

void NotesWindowTests::listsTheNoteAddedThroughTheViewModel() {
    Fixture fixture;

    fixture.viewModel.addNote(QStringLiteral("a title"), QStringLiteral("a body"));

    QListView* list = fixture.window.findChild<QListView*>();
    QVERIFY(list != nullptr);
    QCOMPARE(list->model()->rowCount(), 1);
    QCOMPARE(list->model()->data(list->model()->index(0, 0)).toString(),
             QStringLiteral("a title"));
}

void NotesWindowTests::hidesTheEmptyStateWhenThereAreNotes() {
    Fixture fixture;

    fixture.viewModel.addNote(QStringLiteral("a title"), QStringLiteral("a body"));

    QLabel* emptyState = fixture.window.findChild<QLabel*>("uiKitEmptyState");
    QVERIFY(emptyState != nullptr);
    QVERIFY(!emptyState->isVisible());
}

void NotesWindowTests::showsTheErrorOfTheViewModel() {
    Fixture fixture;
    fixture.repository.listError = desktop::kernel::ErrorCode::Storage;
    fixture.repository.listErrorMessage = QStringLiteral("the disk is full");

    fixture.viewModel.refresh();

    QLabel* error = fixture.window.findChild<QLabel*>("uiKitErrorLabel");
    QVERIFY(error != nullptr);
    QVERIFY(error->isVisible());
    QCOMPARE(error->text(), QStringLiteral("the disk is full"));
}

QTEST_MAIN(NotesWindowTests)
#include "tst_notes_window.moc"
