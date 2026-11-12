#pragma once

#include <QByteArray>
#include <QLocalServer>
#include <QLockFile>
#include <QString>
#include <functional>
#include <utility>

namespace desktop::platform {

// One instance per user: the lock file decides which process is the primary,
// the local socket carries the message of a second instance (playbook 10).
class SingleInstanceGuard {
public:
    using MessageHandler = std::function<void(const QByteArray&)>;

    explicit SingleInstanceGuard(QString key);
    ~SingleInstanceGuard();

    SingleInstanceGuard(const SingleInstanceGuard&) = delete;
    SingleInstanceGuard& operator=(const SingleInstanceGuard&) = delete;

    // True when this process owns the lock and is the primary instance.
    [[nodiscard]] bool isPrimary() const { return primary_; }

    // Sends a message to the running primary instance. False when this
    // process is the primary one or when no instance listens.
    [[nodiscard]] bool notifyPrimary(const QByteArray& message) const;

    // Starts the local server of the primary instance. Does nothing when
    // this process is not the primary one.
    void listen(MessageHandler handler);

private:
    [[nodiscard]] QString socketPath() const;

    QString key_;
    QLockFile lock_;
    QLocalServer* server_ = nullptr;
    bool primary_ = false;
};

}  // namespace desktop::platform
