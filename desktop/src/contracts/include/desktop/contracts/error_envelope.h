#pragma once

#include "desktop/kernel/error_codes.h"

#include <QJsonObject>
#include <QString>

namespace desktop::contracts {

// The error envelope shared with the backend: a code, a human message and
// the correlation id of the failed request.
struct ErrorEnvelope {
    kernel::ErrorCode code = kernel::ErrorCode::Unknown;
    QString message;
    QString requestId;
    QString requiredPermission;
};

[[nodiscard]] QJsonObject toJson(const ErrorEnvelope& envelope);

[[nodiscard]] ErrorEnvelope errorEnvelopeFromJson(const QJsonObject& object,
                                                   bool* ok = nullptr);

}  // namespace desktop::contracts
