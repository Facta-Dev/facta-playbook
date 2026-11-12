#pragma once

#include "desktop/kernel/time.h"

#include <QJsonObject>
#include <QString>

namespace desktop::contracts {

// The realtime message envelope (playbook 8.8): the header identifies the
// event, the payload stays minimal.
struct RealtimeEnvelope {
    QString id;
    QString type;
    QString tenantId;
    QString topic;
    QString correlationId;
    int version = 1;
    kernel::Timestamp timestamp;
    QJsonObject payload;
};

[[nodiscard]] QJsonObject toJson(const RealtimeEnvelope& envelope);

[[nodiscard]] RealtimeEnvelope realtimeEnvelopeFromJson(const QJsonObject& object,
                                                        bool* ok = nullptr);

}  // namespace desktop::contracts
