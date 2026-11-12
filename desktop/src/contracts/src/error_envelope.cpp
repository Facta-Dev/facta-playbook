#include "desktop/contracts/error_envelope.h"

namespace desktop::contracts {

QJsonObject toJson(const ErrorEnvelope& envelope) {
    QJsonObject object;
    object.insert(QStringLiteral("code"), kernel::errorCodeName(envelope.code));
    object.insert(QStringLiteral("detail"), envelope.message);
    if (!envelope.requestId.isEmpty()) {
        object.insert(QStringLiteral("requestId"), envelope.requestId);
    }
    if (!envelope.requiredPermission.isEmpty()) {
        object.insert(QStringLiteral("requiredPermission"), envelope.requiredPermission);
    }
    return object;
}

ErrorEnvelope errorEnvelopeFromJson(const QJsonObject& object, bool* ok) {
    ErrorEnvelope envelope;
    envelope.code = kernel::errorCodeFromName(object.value(QStringLiteral("code")).toString());
    envelope.message = object.value(QStringLiteral("detail")).toString();
    envelope.requestId = object.value(QStringLiteral("requestId")).toString();
    envelope.requiredPermission =
        object.value(QStringLiteral("requiredPermission")).toString();

    if (ok != nullptr) {
        *ok = envelope.code != kernel::ErrorCode::Unknown;
    }
    return envelope;
}

}  // namespace desktop::contracts
