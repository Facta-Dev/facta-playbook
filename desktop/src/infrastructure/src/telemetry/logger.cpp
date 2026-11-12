#include "desktop/infrastructure/telemetry/logger.h"

#include <QLoggingCategory>
#include <QRegularExpression>
#include <QStringList>

namespace desktop::infrastructure::telemetry {
namespace {

Q_LOGGING_CATEGORY(lcTelemetry, "desktop.telemetry")

const QRegularExpression& bearerPattern() {
    static const QRegularExpression kPattern(
        QStringLiteral("(?i)\\bbearer\\s+[A-Za-z0-9._~+/=-]+"));
    return kPattern;
}

}  // namespace

Logger::Logger(int ringCapacity) : capacity_(ringCapacity > 0 ? ringCapacity : 1) {}

QString Logger::categoryName(Category category) {
    switch (category) {
    case Category::App:
        return QStringLiteral("app");
    case Category::Http:
        return QStringLiteral("http");
    case Category::Storage:
        return QStringLiteral("storage");
    case Category::Realtime:
        return QStringLiteral("realtime");
    case Category::Platform:
        return QStringLiteral("platform");
    }
    return QStringLiteral("app");
}

QString Logger::redactText(const QString& text) {
    QString copy = text;
    return copy.replace(bearerPattern(), QStringLiteral("Bearer ***"));
}

bool Logger::isSensitiveKey(const QString& key) {
    const QString lower = key.toLower();
    static const QStringList kSensitive = {
        QStringLiteral("token"),   QStringLiteral("access_token"),
        QStringLiteral("refresh_token"), QStringLiteral("password"),
        QStringLiteral("secret"),  QStringLiteral("authorization"),
        QStringLiteral("api_key"), QStringLiteral("cookie")};
    for (const QString& candidate : kSensitive) {
        if (lower.contains(candidate)) {
            return true;
        }
    }
    return false;
}

void Logger::log(Category category, const QString& message, QMap<QString, QString> fields) {
    LogEntry entry;
    entry.timestamp = kernel::Timestamp::now();
    entry.category = category;
    entry.message = redactionEnabled_ ? redactText(message) : message;

    if (redactionEnabled_) {
        for (auto it = fields.begin(); it != fields.end(); ++it) {
            if (isSensitiveKey(it.key())) {
                it.value() = QStringLiteral("***");
            }
        }
    }
    entry.fields = fields;

    ring_.append(entry);
    if (ring_.size() > capacity_) {
        ring_.remove(0, ring_.size() - capacity_);
    }

    QString line = entry.message;
    for (auto it = entry.fields.constBegin(); it != entry.fields.constEnd(); ++it) {
        line += QStringLiteral(" %1=%2").arg(it.key(), it.value());
    }
    qCInfo(lcTelemetry).noquote() << categoryName(category) << line;
}

}  // namespace desktop::infrastructure::telemetry
