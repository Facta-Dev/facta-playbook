#pragma once

#include "desktop/contracts/note_dto.h"
#include "desktop/kernel/result.h"

#include <QFuture>
#include <QNetworkAccessManager>
#include <QNetworkRequest>
#include <QObject>
#include <QString>
#include <QUrl>
#include <QVector>

namespace desktop::sdk {

// The typed client of the notes module (playbook 5.1). The routes are built
// here and nowhere else, the client has no GUI dependency and no knowledge
// of the domain: it speaks the wire shapes of the contracts.
class NotesClient : public QObject {
    Q_OBJECT

public:
    explicit NotesClient(QUrl baseUrl, QObject* parent = nullptr);

    // The bearer token of the session; an empty token means "not signed in".
    void setAccessToken(const QString& token);

    // GET /api/v1/app/notes
    [[nodiscard]] QFuture<kernel::Result<QVector<contracts::NoteDto>>> listNotes();

    // POST /api/v1/app/notes
    [[nodiscard]] QFuture<kernel::Result<contracts::NoteDto>> createNote(const QString& title,
                                                                         const QString& body);

    // GET /api/v1/app/notes/{noteId}
    [[nodiscard]] QFuture<kernel::Result<contracts::NoteDto>> getNote(const QString& noteId);

    // The routes of the module, without the base URL.
    [[nodiscard]] static QString notesPath();
    [[nodiscard]] static QString notePath(const QString& noteId);

private:
    [[nodiscard]] QNetworkRequest request(const QString& path) const;

    QUrl baseUrl_;
    QString accessToken_;
    QNetworkAccessManager manager_;
};

}  // namespace desktop::sdk
