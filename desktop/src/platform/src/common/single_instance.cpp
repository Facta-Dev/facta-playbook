#include "desktop/platform/single_instance.h"

#include "desktop/platform/app_paths.h"

#include <QDir>
#include <QLocalSocket>
#include <chrono>
#include <utility>

namespace desktop::platform {

SingleInstanceGuard::SingleInstanceGuard(QString key)
    : key_(std::move(key)), lock_(AppPaths::lockFile()) {
    QDir().mkpath(AppPaths::runtimeDir());
    primary_ = lock_.tryLock(std::chrono::milliseconds::zero());
}

SingleInstanceGuard::~SingleInstanceGuard() {
    if (server_ != nullptr) {
        server_->close();
        delete server_;
        server_ = nullptr;
    }
    if (primary_) {
        lock_.unlock();
    }
}

QString SingleInstanceGuard::socketPath() const {
    return AppPaths::runtimeDir() + QLatin1Char('/') + key_ + QStringLiteral(".socket");
}

bool SingleInstanceGuard::notifyPrimary(const QByteArray& message) const {
    if (primary_) {
        return false;
    }

    QLocalSocket socket;
    socket.connectToServer(socketPath());
    // A second instance runs before its event loop starts, so a short
    // blocking wait is the simplest way to hand the message over.
    if (!socket.waitForConnected(500)) {
        return false;
    }
    socket.write(message);
    socket.flush();
    socket.waitForBytesWritten(500);
    socket.disconnectFromServer();
    if (socket.state() != QLocalSocket::UnconnectedState) {
        socket.waitForDisconnected(500);
    }
    return true;
}

void SingleInstanceGuard::listen(MessageHandler handler) {
    if (!primary_ || server_ != nullptr) {
        return;
    }

    const QString path = socketPath();
    // A crashed instance leaves the socket file behind. The lock says this
    // process is the primary instance, so the stale socket can go.
    QLocalServer::removeServer(path);

    server_ = new QLocalServer();
    // Only the user who owns the socket may talk to it.
    server_->setSocketOptions(QLocalServer::UserAccessOption);
    if (!server_->listen(path)) {
        delete server_;
        server_ = nullptr;
        return;
    }

    QObject::connect(server_, &QLocalServer::newConnection, server_, [this, handler]() {
        while (QLocalSocket* socket = server_->nextPendingConnection()) {
            QObject::connect(socket, &QLocalSocket::readyRead, socket, [handler, socket]() {
                handler(socket->readAll());
            });
            QObject::connect(socket, &QLocalSocket::disconnected, socket,
                             &QLocalSocket::deleteLater);
        }
    });
}

}  // namespace desktop::platform
