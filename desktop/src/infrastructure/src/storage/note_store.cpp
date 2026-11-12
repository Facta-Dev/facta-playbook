#include "desktop/infrastructure/storage/note_store.h"

#include "desktop/kernel/error_codes.h"

#include <QDir>
#include <QFileInfo>
#include <QSqlError>
#include <QSqlQuery>
#include <QtGlobal>

namespace desktop::infrastructure::storage {
namespace {

QString nextConnectionName() {
    static quint64 counter = 0;
    ++counter;
    return QStringLiteral("notes-%1").arg(counter);
}

}  // namespace

NoteStore::NoteStore(QSqlDatabase database) : database_(std::move(database)) {}

QVector<Migration> NoteStore::migrations() {
    return QVector<Migration>{
        {1, R"(CREATE TABLE IF NOT EXISTS notes (
                   id TEXT PRIMARY KEY NOT NULL,
                   title TEXT NOT NULL,
                   body TEXT NOT NULL,
                   created_at TEXT NOT NULL,
                   updated_at TEXT NOT NULL
               ))"},
    };
}

kernel::Result<NoteStore> NoteStore::open(const QString& databasePath) {
    if (!QSqlDatabase::isDriverAvailable(QStringLiteral("QSQLITE"))) {
        qFatal("The QSQLITE driver is missing: install the Qt 6 SQLite plugin "
               "(libqsqlite) before running the desktop app.");
    }

    const QFileInfo info(databasePath);
    if (!QDir().mkpath(info.absolutePath())) {
        return kernel::Result<NoteStore>(kernel::ErrorCode::Storage,
                                         QStringLiteral("cannot create the database directory"));
    }

    const QString connectionName = nextConnectionName();
    QSqlDatabase database = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connectionName);
    database.setDatabaseName(databasePath);
    if (!database.open()) {
        const QString message = database.lastError().text();
        QSqlDatabase::removeDatabase(connectionName);
        return kernel::Result<NoteStore>(kernel::ErrorCode::Storage, message);
    }

    QSqlQuery settings(database);
    settings.exec(QStringLiteral("PRAGMA journal_mode=WAL"));
    settings.exec(QStringLiteral("PRAGMA synchronous=NORMAL"));

    NoteStore store(database);
    const kernel::Result<void> migrated = store.migrate();
    if (!migrated.ok()) {
        return kernel::Result<NoteStore>(migrated.errorCode(), migrated.errorMessage());
    }
    return kernel::Result<NoteStore>(store);
}

kernel::Result<int> NoteStore::schemaVersion() const {
    QSqlQuery query(database_);
    if (!query.exec(QStringLiteral("PRAGMA user_version"))) {
        return kernel::Result<int>(kernel::ErrorCode::Storage, query.lastError().text());
    }
    if (!query.next()) {
        return kernel::Result<int>(kernel::ErrorCode::Storage,
                                   QStringLiteral("the database reports no schema version"));
    }
    return kernel::Result<int>(query.value(0).toInt());
}

kernel::Result<void> NoteStore::migrate() {
    const kernel::Result<int> current = schemaVersion();
    if (!current.ok()) {
        return kernel::Result<void>(current.errorCode(), current.errorMessage());
    }

    const int from = current.value();
    for (const Migration& migration : migrations()) {
        if (migration.version <= from) {
            continue;
        }
        QSqlQuery query(database_);
        if (!query.exec(QString::fromLatin1(migration.sql))) {
            return kernel::Result<void>(kernel::ErrorCode::Storage, query.lastError().text());
        }
        QSqlQuery settings(database_);
        settings.exec(QStringLiteral("PRAGMA user_version = %1").arg(migration.version));
    }
    return kernel::Result<void>();
}

kernel::Result<QVector<contracts::NoteDto>> NoteStore::listNotes() const {
    QSqlQuery query(database_);
    if (!query.exec(QStringLiteral("SELECT id, title, body, created_at, updated_at "
                                   "FROM notes ORDER BY created_at"))) {
        return kernel::Result<QVector<contracts::NoteDto>>(kernel::ErrorCode::Storage,
                                                            query.lastError().text());
    }

    QVector<contracts::NoteDto> notes;
    while (query.next()) {
        contracts::NoteDto note;
        note.id = kernel::NoteId(query.value(0).toString());
        note.title = query.value(1).toString();
        note.body = query.value(2).toString();
        note.createdAt = kernel::Timestamp::fromIso8601(query.value(3).toString());
        note.updatedAt = kernel::Timestamp::fromIso8601(query.value(4).toString());
        notes.append(note);
    }
    return kernel::Result<QVector<contracts::NoteDto>>(notes);
}

kernel::Result<contracts::NoteDto> NoteStore::upsertNote(const contracts::NoteDto& note) {
    if (!note.id.isValid()) {
        return kernel::Result<contracts::NoteDto>(kernel::ErrorCode::InvalidArgument,
                                                  QStringLiteral("the note has no id"));
    }

    QSqlQuery query(database_);
    query.prepare(QStringLiteral(
        "INSERT INTO notes (id, title, body, created_at, updated_at) VALUES (?, ?, ?, ?, ?) "
        "ON CONFLICT(id) DO UPDATE SET title = excluded.title, body = excluded.body, "
        "updated_at = excluded.updated_at"));
    query.addBindValue(note.id.value());
    query.addBindValue(note.title);
    query.addBindValue(note.body);
    query.addBindValue(note.createdAt.toIso8601());
    query.addBindValue(note.updatedAt.toIso8601());
    if (!query.exec()) {
        return kernel::Result<contracts::NoteDto>(kernel::ErrorCode::Storage,
                                                  query.lastError().text());
    }
    return kernel::Result<contracts::NoteDto>(note);
}

kernel::Result<void> NoteStore::removeNote(const QString& noteId) {
    QSqlQuery query(database_);
    query.prepare(QStringLiteral("DELETE FROM notes WHERE id = ?"));
    query.addBindValue(noteId);
    if (!query.exec()) {
        return kernel::Result<void>(kernel::ErrorCode::Storage, query.lastError().text());
    }
    return kernel::Result<void>();
}

}  // namespace desktop::infrastructure::storage
