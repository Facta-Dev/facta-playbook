#pragma once

#include "desktop/contracts/note_dto.h"
#include "desktop/kernel/result.h"

#include <QSqlDatabase>
#include <QString>
#include <QVector>

namespace desktop::infrastructure::storage {

// One migration step. The version is the PRAGMA user_version of the database;
// the migrations are forward-only and applied in order.
struct Migration {
    int version;
    const char* sql;
};

// The local projection cache of the notes (playbook 9). The server holds the
// truth: this database is never authoritative, it only makes the app usable
// offline and fast on the next start.
class NoteStore {
public:
    // Opens (and creates) the database and applies the pending migrations.
    // Fails fast when the SQLite driver is missing.
    static kernel::Result<NoteStore> open(const QString& databasePath);

    [[nodiscard]] kernel::Result<int> schemaVersion() const;
    [[nodiscard]] kernel::Result<QVector<contracts::NoteDto>> listNotes() const;
    [[nodiscard]] kernel::Result<contracts::NoteDto> upsertNote(const contracts::NoteDto& note);
    [[nodiscard]] kernel::Result<void> removeNote(const QString& noteId);

    // The migration list, in order.
    [[nodiscard]] static QVector<Migration> migrations();

private:
    explicit NoteStore(QSqlDatabase database);

    kernel::Result<void> migrate();

    QSqlDatabase database_;
};

}  // namespace desktop::infrastructure::storage
