#include "desktop/contracts/realtime_envelope.h"

namespace desktop::contracts {

QJsonObject toJson(const RealtimeEnvelope& envelope) {
    QJsonObject object;
    object.insert(QStringLiteral("id"), envelope.id);
    object.insert(QStringLiteral("type"), envelope.type);
    object.insert(QStringLiteral("tenantId"), envelope.tenantId);
    object.insert(QStringLiteral("topic"), envelope.topic);
    object.insert(QStringLiteral("correlationId"), envelope.correlationId);
    object.insert(QStringLiteral("v"), envelope.version);
    object.insert(QStringLiteral("ts"), envelope.timestamp.toIso8601());
    object.insert(QStringLiteral("payload"), envelope.payload);
    return object;
}

RealtimeEnvelope realtimeEnvelopeFromJson(const QJsonObject& object, bool* ok) {
    RealtimeEnvelope envelope;
    envelope.id = object.value(QStringLiteral("id")).toString();
    envelope.type = object.value(QStringLiteral("type")).toString();
    envelope.tenantId = object.value(QStringLiteral("tenantId")).toString();
    envelope.topic = object.value(QStringLiteral("topic")).toString();
    envelope.correlationId = object.value(QStringLiteral("correlationId")).toString();
    envelope.version = object.value(QStringLiteral("v")).toInt(1);
    envelope.timestamp =
        kernel::Timestamp::fromIso8601(object.value(QStringLiteral("ts")).toString());
    envelope.payload = object.value(QStringLiteral("payload")).toObject();

    if (ok != nullptr) {
        *ok = !envelope.id.isEmpty() && !envelope.type.isEmpty();
    }
    return envelope;
}

}  // namespace desktop::contracts
