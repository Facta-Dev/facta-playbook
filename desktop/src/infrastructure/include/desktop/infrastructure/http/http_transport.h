#pragma once

#include "desktop/kernel/result.h"

#include <QByteArray>
#include <QMap>
#include <QNetworkAccessManager>
#include <QObject>
#include <QString>
#include <QUrl>
#include <functional>
#include <utility>

namespace desktop::infrastructure::http {

struct HttpTransportConfig {
    int timeoutMs = 15000;
    int maxRetries = 2;
    int retryDelayMs = 250;
    int maxResponseBytes = 4 * 1024 * 1024;
};

struct HttpRequest {
    QUrl url;
    QByteArray method = QByteArrayLiteral("GET");
    QByteArray body;
    QMap<QString, QString> headers;
};

struct HttpResponse {
    int statusCode = 0;
    QByteArray body;
    QMap<QString, QString> headers;

    bool operator==(const HttpResponse&) const = default;
};

// The outbound HTTP transport: one QNetworkAccessManager, explicit timeouts
// and a bounded retry with backoff. It knows nothing about the UI.
//
// A response with a status code is a success even for 4xx and 5xx: the
// status decides what the caller does with it. Only a transport failure (no
// status at all) becomes an error.
class HttpTransport : public QObject {
    Q_OBJECT

public:
    using Callback = std::function<void(kernel::Result<HttpResponse>)>;

    explicit HttpTransport(HttpTransportConfig config = {}, QObject* parent = nullptr);
    ~HttpTransport() override = default;

    HttpTransport(const HttpTransport&) = delete;
    HttpTransport& operator=(const HttpTransport&) = delete;

    // Sends the request and calls the callback with the outcome. The
    // callback runs on the thread of the transport.
    void send(const HttpRequest& request, Callback callback);

private:
    void sendAttempt(const HttpRequest& request, int attempt, Callback callback);
    void scheduleRetry(const HttpRequest& request, int attempt, Callback callback);

    HttpTransportConfig config_;
    QNetworkAccessManager manager_;
};

}  // namespace desktop::infrastructure::http
