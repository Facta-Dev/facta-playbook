#include "desktop/infrastructure/http/http_transport.h"

#include "desktop/kernel/error_codes.h"

#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>
#include <memory>
#include <utility>

namespace desktop::infrastructure::http {
namespace {

QMap<QString, QString> collectHeaders(QNetworkReply* reply) {
    QMap<QString, QString> headers;
    const QList<std::pair<QByteArray, QByteArray>> pairs = reply->rawHeaderPairs();
    for (const auto& pair : pairs) {
        headers.insert(QString::fromLatin1(pair.first).toLower(),
                       QString::fromLatin1(pair.second));
    }
    return headers;
}

kernel::ErrorCode transportErrorCode(QNetworkReply::NetworkError error, bool timedOut) {
    if (timedOut) {
        return kernel::ErrorCode::Timeout;
    }
    switch (error) {
    case QNetworkReply::NoError:
        return kernel::ErrorCode::Unknown;
    case QNetworkReply::TimeoutError:
        return kernel::ErrorCode::Timeout;
    case QNetworkReply::OperationCanceledError:
        return kernel::ErrorCode::Cancelled;
    case QNetworkReply::ContentNotFoundError:
        return kernel::ErrorCode::NotFound;
    case QNetworkReply::AuthenticationRequiredError:
        return kernel::ErrorCode::Unauthenticated;
    case QNetworkReply::ContentAccessDenied:
    case QNetworkReply::ContentReSendError:
        return kernel::ErrorCode::PermissionDenied;
    default:
        return kernel::ErrorCode::Network;
    }
}

}  // namespace

HttpTransport::HttpTransport(HttpTransportConfig config, QObject* parent)
    : QObject(parent), config_(std::move(config)) {}

void HttpTransport::send(const HttpRequest& request, Callback callback) {
    sendAttempt(request, 0, std::move(callback));
}

void HttpTransport::scheduleRetry(const HttpRequest& request, int attempt, Callback callback) {
    const int delay = config_.retryDelayMs * (1 << attempt);
    QTimer::singleShot(delay, this, [this, request, attempt, callback]() {
        sendAttempt(request, attempt + 1, callback);
    });
}

void HttpTransport::sendAttempt(const HttpRequest& request, int attempt, Callback callback) {
    QNetworkRequest networkRequest(request.url);
#if QT_VERSION >= QT_VERSION_CHECK(6, 7, 0)
    networkRequest.setTransferTimeout(config_.timeoutMs);
#endif
    for (auto it = request.headers.constBegin(); it != request.headers.constEnd(); ++it) {
        networkRequest.setRawHeader(it.key().toUtf8(), it.value().toUtf8());
    }

    QNetworkReply* reply = manager_.sendCustomRequest(networkRequest, request.method, request.body);

    // The transfer timeout does not cover a server that accepts the request
    // and never answers, so the reply gets its own deadline.
    auto timedOut = std::make_shared<bool>(false);
    auto* deadline = new QTimer(reply);
    deadline->setSingleShot(true);
    QObject::connect(deadline, &QTimer::timeout, reply, [reply, timedOut]() {
        *timedOut = true;
        reply->abort();
    });
    deadline->start(config_.timeoutMs);

    QObject::connect(reply, &QNetworkReply::finished, reply,
                     [this, request, attempt, callback, reply, timedOut]() {
        const QNetworkReply::NetworkError error = reply->error();
        const int status =
            reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();

        if (status > 0) {
            const QByteArray body = reply->readAll();
            if (body.size() > config_.maxResponseBytes) {
                callback(kernel::Result<HttpResponse>(
                    kernel::ErrorCode::Server, QStringLiteral("the response is too large")));
                reply->deleteLater();
                return;
            }
            const HttpResponse response{status, body, collectHeaders(reply)};
            const bool serverSide = status == 429 || status >= 500;
            if (serverSide && attempt < config_.maxRetries) {
                scheduleRetry(request, attempt, callback);
                reply->deleteLater();
                return;
            }
            callback(kernel::Result<HttpResponse>(response));
            reply->deleteLater();
            return;
        }

        // No status code: the request never reached the server.
        const bool cancelled = error == QNetworkReply::OperationCanceledError && !*timedOut;
        if (!cancelled && attempt < config_.maxRetries) {
            scheduleRetry(request, attempt, callback);
            reply->deleteLater();
            return;
        }
        callback(kernel::Result<HttpResponse>(transportErrorCode(error, *timedOut),
                                              reply->errorString()));
        reply->deleteLater();
    });
}

}  // namespace desktop::infrastructure::http
