// The notes slice: the invariants of the entity and the use cases over a fake
// repository. No widget, no storage.

#include "desktop/kernel/error_codes.h"
#include "desktop/kernel/ids.h"
#include "desktop/kernel/time.h"
#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/domain/note.h"

#include "fake_note_repository.h"

#include <QTest>

namespace {

using desktop::kernel::ErrorCode;
using desktop::kernel::NoteId;
using desktop::kernel::Result;
using desktop::kernel::Timestamp;
using desktop::notes::application::AddNote;
using desktop::notes::application::ListNotes;
using desktop::notes::domain::Note;
using desktop::tests::FakeNoteRepository;

Result<Note> makeNote(const QString& title, const QString& body) {
    return Note::create(NoteId(QStringLiteral("n-1")), title, body, Timestamp::now(),
                        Timestamp::now());
}

}  // namespace

class NotesSliceTests : public QObject {
    Q_OBJECT

private slots:
    void rejectsAnEmptyTitle();
    void rejectsALongTitle();
    void rejectsALongBody();
    void rejectsMarkup();
    void trimsTheTitle();
    void keepsAValidNote();
    void addsANoteThroughTheUseCase();
    void doesNotStoreARejectedNote();
    void listsTheNotesThroughTheUseCase();
    void reportsTheFailureOfTheRepository();
};

void NotesSliceTests::rejectsAnEmptyTitle() {
    const Result<Note> note = makeNote(QString(), QStringLiteral("a body"));

    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesSliceTests::rejectsALongTitle() {
    const Result<Note> note =
        makeNote(QString(Note::kMaxTitleLength + 1, QLatin1Char('x')), QStringLiteral("a body"));

    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesSliceTests::rejectsALongBody() {
    const Result<Note> note = makeNote(QStringLiteral("a title"),
                                        QString(Note::kMaxBodyLength + 1, QLatin1Char('x')));

    QVERIFY(!note.ok());
    QCOMPARE(note.errorCode(), ErrorCode::ValidationFailed);
}

void NotesSliceTests::rejectsMarkup() {
    QVERIFY(!makeNote(QStringLiteral("<b>title</b>"), QStringLiteral("a body")).ok());
    QVERIFY(!makeNote(QStringLiteral("a title"), QStringLiteral("<script>body</script>")).ok());
}

void NotesSliceTests::trimsTheTitle() {
    const Result<Note> note = makeNote(QStringLiteral("  a title  "), QStringLiteral("a body"));

    QVERIFY(note.ok());
    QCOMPARE(note.value().title(), QStringLiteral("a title"));
}

void NotesSliceTests::keepsAValidNote() {
    const Result<Note> note = makeNote(QStringLiteral("a title"), QStringLiteral("a body"));

    QVERIFY(note.ok());
    QCOMPARE(note.value().title(), QStringLiteral("a title"));
    QCOMPARE(note.value().body(), QStringLiteral("a body"));
    QCOMPARE(note.value().id().value(), QStringLiteral("n-1"));
}

void NotesSliceTests::addsANoteThroughTheUseCase() {
    FakeNoteRepository repository;
    const AddNote addNote(repository);

    const Result<Note> added = addNote.execute(QStringLiteral("a title"), QStringLiteral("a body"));

    QVERIFY(added.ok());
    QCOMPARE(added.value().title(), QStringLiteral("a title"));
    QCOMPARE(repository.notes.size(), 1);
    QVERIFY(added.value().id().isValid());
}

void NotesSliceTests::doesNotStoreARejectedNote() {
    FakeNoteRepository repository;
    const AddNote addNote(repository);

    const Result<Note> added = addNote.execute(QString(), QStringLiteral("a body"));

    QVERIFY(!added.ok());
    QCOMPARE(added.errorCode(), ErrorCode::ValidationFailed);
    QCOMPARE(repository.notes.size(), 0);
}

void NotesSliceTests::listsTheNotesThroughTheUseCase() {
    FakeNoteRepository repository;
    const AddNote addNote(repository);
    const ListNotes listNotes(repository);

    QVERIFY(addNote.execute(QStringLiteral("first"), QStringLiteral("a body")).ok());
    QVERIFY(addNote.execute(QStringLiteral("second"), QStringLiteral("another body")).ok());

    const Result<QVector<Note>> listed = listNotes.execute();

    QVERIFY(listed.ok());
    QCOMPARE(listed.value().size(), 2);
    QCOMPARE(listed.value().first().title(), QStringLiteral("first"));
    QCOMPARE(listed.value().last().title(), QStringLiteral("second"));
}

void NotesSliceTests::reportsTheFailureOfTheRepository() {
    FakeNoteRepository repository;
    repository.listError = ErrorCode::Storage;
    repository.listErrorMessage = QStringLiteral("disk full");
    const ListNotes listNotes(repository);

    const Result<QVector<Note>> listed = listNotes.execute();

    QVERIFY(!listed.ok());
    QCOMPARE(listed.errorCode(), ErrorCode::Storage);
    QCOMPARE(listed.errorMessage(), QStringLiteral("disk full"));
}

QTEST_MAIN(NotesSliceTests)
#include "tst_notes_slice.moc"
