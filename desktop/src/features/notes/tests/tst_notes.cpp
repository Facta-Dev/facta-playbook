#include "desktop/infrastructure/storage/note_store.h"
#include "desktop/kernel/error_codes.h"
#include "desktop/notes/adapters/sqlite_note_repository.h"
#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/domain/note.h"
#include "desktop/notes/presentation/notes_list_model.h"
#include "desktop/sdk/notes_client.h"

#include <QTemporaryDir>
#include <QTest>
#include <QVector>

namespace {

using desktop::kernel::ErrorCode;
using desktop::kernel::Result;
using desktop::notes::application::AddNote;
using desktop::notes::application::ListNotes;
using desktop::notes::application::NoteRepository;
using desktop::notes::domain::Note;

// A repository in memory: the use cases are checked without a database.
class FakeNoteRepository : public NoteRepository {
public:
    Result<QVector<Note>> list() const override {
        return Result<QVector<Note>>(notes);
    }

    Result<Note> add(const Note& note) override {
        notes.append(note);
        return Result<Note>(note);
    }

    QVector<Note> notes;
};

Result<Note> makeNote(const QString& title, const QString& body) {
    return Note::create(desktop::kernel::NoteId(QStringLiteral("id-1")), title, body,
                        desktop::kernel::Timestamp::now(), desktop::kernel::Timestamp::now());
}

}  // namespace

class NotesTests : public QObject {
    Q_OBJECT

private slots:
    void rejectsAnEmptyTitle();
    void rejectsALongTitle();
    void rejectsALongBody();
    void rejectsMarkup();
    void keepsAValidNote();
    void addsANoteThroughTheUseCase();
    void listsTheNotesThroughTheUseCase();
    void showsTheTitleOfTheNoteInTheList();
    void storesAndListsThroughTheSqliteRepository();
    void buildsTheRoutesOfTheModule();
};

void NotesTests::rejectsAnEmptyTitle() {
    const Result<Note> note = makeNote(QStringLiteral("   "), QStringLiteral("a body"));
    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesTests::rejectsALongTitle() {
    const Result<Note> note =
        makeNote(QString(121, QLatin1Char('x')), QStringLiteral("a body"));
    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesTests::rejectsALongBody() {
    const Result<Note> note =
        makeNote(QStringLiteral("a title"), QString(10001, QLatin1Char('x')));
    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesTests::rejectsMarkup() {
    const Result<Note> note =
        makeNote(QStringLiteral("<b>a title</b>"), QStringLiteral("a body"));
    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesTests::keepsAValidNote() {
    const Result<Note> note = makeNote(QStringLiteral("  a title  "), QStringLiteral("a body"));
    QVERIFY(note.ok());
    QCOMPARE(note.value().title(), QStringLiteral("a title"));
    QCOMPARE(note.value().body(), QStringLiteral("a body"));
    QCOMPARE(note.value().id().toString(), QStringLiteral("id-1"));
}

void NotesTests::addsANoteThroughTheUseCase() {
    FakeNoteRepository repository;
    const AddNote addNote(repository);

    const Result<Note> added = addNote.execute(QStringLiteral("a title"), QStringLiteral("a body"));
    QVERIFY(added.ok());
    QCOMPARE(repository.notes.size(), 1);
    QCOMPARE(repository.notes.first().title(), QStringLiteral("a title"));

    const Result<Note> rejected = addNote.execute(QString(), QStringLiteral("a body"));
    QVERIFY(!rejected.ok());
    QCOMPARE(repository.notes.size(), 1);
}

void NotesTests::listsTheNotesThroughTheUseCase() {
    FakeNoteRepository repository;
    const AddNote addNote(repository);
    const ListNotes listNotes(repository);

    QVERIFY(addNote.execute(QStringLiteral("first"), QStringLiteral("body")).ok());
    QVERIFY(addNote.execute(QStringLiteral("second"), QStringLiteral("body")).ok());

    const Result<QVector<Note>> listed = listNotes.execute();
    QVERIFY(listed.ok());
    QCOMPARE(listed.value().size(), 2);
}

void NotesTests::showsTheTitleOfTheNoteInTheList() {
    desktop::notes::presentation::NotesListModel model;
    const Result<Note> note = makeNote(QStringLiteral("a title"), QStringLiteral("a body"));
    QVERIFY(note.ok());

    model.setNotes(QVector<Note>{note.value()});
    QCOMPARE(model.rowCount(), 1);
    QCOMPARE(model.data(model.index(0, 0), Qt::DisplayRole).toString(),
             QStringLiteral("a title"));
}

void NotesTests::storesAndListsThroughTheSqliteRepository() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());

    const auto opened = desktop::infrastructure::storage::NoteStore::open(
        directory.filePath(QStringLiteral("notes.sqlite")));
    QVERIFY(opened.ok());

    desktop::notes::adapters::SqliteNoteRepository repository(opened.value());
    const AddNote addNote(repository);
    const ListNotes listNotes(repository);

    QVERIFY(addNote.execute(QStringLiteral("a title"), QStringLiteral("a body")).ok());

    const Result<QVector<Note>> listed = listNotes.execute();
    QVERIFY(listed.ok());
    QCOMPARE(listed.value().size(), 1);
    QCOMPARE(listed.value().first().title(), QStringLiteral("a title"));
    QCOMPARE(listed.value().first().body(), QStringLiteral("a body"));
}

void NotesTests::buildsTheRoutesOfTheModule() {
    QCOMPARE(desktop::sdk::NotesClient::notesPath(), QStringLiteral("/api/v1/app/notes"));
    QCOMPARE(desktop::sdk::NotesClient::notePath(QStringLiteral("abc")),
             QStringLiteral("/api/v1/app/notes/abc"));
}

QTEST_MAIN(NotesTests)
#include "tst_notes.moc"
