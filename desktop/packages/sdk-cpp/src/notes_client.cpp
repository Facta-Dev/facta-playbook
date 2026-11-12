#include "desktop/sdk/notes_client.h"

#include "desktop/contracts/error_envelope.h"
#include "desktop/kernel/error_codes.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QNetworkReply>
#include <QPromise>
#include <memory>
#include <utility>

namespace desktop::sdk {
namespace {

// The route table of the notes module (PLAYBOOK_URLS.md): the version, the
// experience namespace and the module. The path is built here and nowhere
// else, and the namespace never authorizes anything.
QString apiRoot() {
    return QStringLiteral("/api/v1/app");
}

kernel::ErrorCode errorCodeForStatus(int status) {
    switch (status) {
    case 401:
        return kernel::ErrorCode::Unauthenticated;
    case 403:
        return kernel::ErrorCode::PermissionDenied;
    case 404:
        return kernel::ErrorCode::NotFound;
    case 409:
        return kernel::ErrorCode::Conflict;
    case 422:
        return kernel::ErrorCode::ValidationFailed;
    case 429:
        return kernel::ErrorCode::RateLimited;
    default:
        break;
    }
    return status >= 500 ? kernel::ErrorCode::Server : kernel::ErrorCode::Unknown;
}

// A failed answer: the envelope of the backend wins over the status code.
template <class T>
kernel::Result<T> failure(const QByteArray& body, int status) {
    const QJsonDocument document = QJsonDocument::fromJson(body);
    if (document.isObject()) {
        bool ok = false;
        const contracts::ErrorEnvelope envelope =
            contracts::errorEnvelopeFromJson(document.object(), &ok);
        if (ok) {
            return kernel::Result<T>(envelope.code, envelope.message);
        }
    }
    return kernel::Result<T>(errorCodeForStatus(status),
                             QStringLiteral("the server answered with the status %1").arg(status));
}

kernel::Result<contracts::NoteDto> readNote(const QByteArray& body, int status) {
    if (status >= 400) {
        return failure<contracts::NoteDto>(body, status);
    }
    const QJsonDocument document = QJsonDocument::fromJson(body);
    if (!document.isObject()) {
        return kernel::Result<contracts::NoteDto>(
            kernel::ErrorCode::Server, QStringLiteral("the answer of the server is not a note"));
    }
    bool ok = false;
    const contracts::NoteDto note = contracts::noteFromJson(document.object(), &ok);
    if (!ok) {
        return kernel::Result<contracts::NoteDto>(
            kernel::ErrorCode::Server, QStringLiteral("the note of the server is malformed"));
    }
    return kernel::Result<contracts::NoteDto>(note);
}

kernel::Result<QVector<contracts::NoteDto>> readNotes(const QByteArray& body, int status) {
    if (status >= 400) {
        return failure<QVector<contracts::NoteDto>>(body, status);
    }
    const QJsonDocument document = QJsonDocument::fromJson(body);
    if (!document.isArray()) {
        return kernel::Result<QVector<contracts::NoteDto>>(
            kernel::ErrorCode::Server, QStringLiteral("the answer of the server is not a list"));
    }

    QVector<contracts::NoteDto> notes;
    const QJsonArray array = document.array();
    notes.reserve(array.size());
    for (const QJsonValue& value : array) {
        bool ok = false;
        const contracts::NoteDto note = contracts::noteFromJson(value.toObject(), &ok);
        if (!ok) {
            return kernel::Result<QVector<contracts::NoteDto>>(
                kernel::ErrorCode::Server, QStringLiteral("a note of the server is malformed"));
        }
        notes.append(note);
    }
    return kernel::Result<QVector<contracts::NoteDto>>(notes);
}

int statusOf(QNetworkReply* reply) {
    return reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
}

}  // namespace

NotesClient::NotesClient(QUrl baseUrl, QObject* parent) : QObject(parent), baseUrl_(baseUrl) {}

void NotesClient::setAccessToken(const QString& token) {
    accessToken_ = token;
}

QString NotesClient::notesPath() {
    return QStringLiteral("%1/notes").arg(apiRoot());
}

QString NotesClient::notePath(const QString& noteId) {
    return QStringLiteral("%1/%2").arg(notesPath(), noteId);
}

QNetworkRequest NotesClient::request(const QString& path) const {
    QNetworkRequest request(baseUrl_.resolved(QUrl(path)));
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    request.setRawHeader(QByteArrayLiteral("Accept"), QByteArrayLiteral("application/json"));
    request.setRawHeader(QByteArrayLiteral("X-Client"), QByteArrayLiteral("desktop"));
    if (!accessToken_.isEmpty()) {
        request.setRawHeader(QByteArrayLiteral("Authorization"),
                             QByteArrayLiteral("Bearer ") + accessToken_.toUtf8());
    }
    return request;
}

QFuture<kernel::Result<QVector<contracts::NoteDto>>> NotesClient::listNotes() {
    auto promise = std::make_shared<QPromise<kernel::Result<QVector<contracts::NoteDto>>>>();
    promise->start();

    QNetworkReply* reply = manager_.get(request(notesPath()));
    connect(reply, &QNetworkReply::finished, this, [reply, promise]() {
        kernel::Result<QVector<contracts::NoteDto>> result =
            readNotes(reply->readAll(), statusOf(reply));
        if (statusOf(reply) == 0) {
            result = kernel::Result<QVector<contracts::NoteDto>>(kernel::ErrorCode::Network,
                                                                 reply->errorString());
        }
        promise->addResult(std::move(result));
        promise->finish();
        reply->deleteLater();
    });
    return promise->future();
}

QFuture<kernel::Result<contracts::NoteDto>> NotesClient::createNote(const QString& title,
                                                                     const QString& body) {
    auto promise = std::make_shared<QPromise<kernel::Result<contracts::NoteDto>>>();
    promise->start();

    QJsonObject payload;
    payload.insert(QStringLiteral("title"), title);
    payload.insert(QStringLiteral("body"), body);
    const QByteArray data = QJsonDocument(payload).toJson(QJsonDocument::Compact);

    QNetworkReply* reply = manager_.post(request(notesPath()), data);
    connect(reply, &QNetworkReply::finished, this, [reply, promise]() {
        kernel::Result<contracts::NoteDto> result = readNote(reply->readAll(), statusOf(reply));
        if (statusOf(reply) == 0) {
            result = kernel::Result<contracts::NoteDto>(kernel::ErrorCode::Network,
                                                         reply->errorString());
        }
        promise->addResult(std::move(result));
        promise->finish();
        reply->deleteLater();
    });
    return promise->future();
}

QFuture<kernel::Result<contracts::NoteDto>> NotesClient::getNote(const QString& noteId) {
    auto promise = std::make_shared<QPromise<kernel::Result<contracts::NoteDto>>>();
    promise->start();

    QNetworkReply* reply = manager_.get(request(notePath(noteId)));
    connect(reply, &QNetworkReply::finished, this, [reply, promise]() {
        kernel::Result<contracts::NoteDto> result = readNote(reply->readAll(), statusOf(reply));
        if (statusOf(reply) == 0) {
            result = kernel::Result<contracts::NoteDto>(kernel::ErrorCode::Network,
                                                         reply->errorString());
        }
        promise->addResult(std::move(result));
        promise->finish();
        reply->deleteLater();
    });
    return promise->future();
}

}  // namespace desktop::sdk
