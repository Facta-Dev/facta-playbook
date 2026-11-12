#include "desktop/kernel/time.h"

namespace desktop::kernel {

Timestamp Timestamp::now() {
    return Timestamp(QDateTime::currentDateTimeUtc());
}

Timestamp Timestamp::fromIso8601(const QString& text, bool* ok) {
    QDateTime parsed = QDateTime::fromString(text, Qt::ISODateWithMs);
    if (!parsed.isValid()) {
        parsed = QDateTime::fromString(text, Qt::ISODate);
    }
    if (ok != nullptr) {
        *ok = parsed.isValid();
    }
    if (!parsed.isValid()) {
        return Timestamp();
    }
    return Timestamp(parsed);
}

QString Timestamp::toIso8601() const {
    if (!value_.isValid()) {
        return QString();
    }
    return value_.toUTC().toString(Qt::ISODateWithMs);
}

}  // namespace desktop::kernel
