#pragma once

#include <QString>

namespace desktop::kernel {

// The error taxonomy shared by every layer. The wire names follow the backend
// taxonomy (playbook 5.1): unauthenticated | forbidden | not-found | conflict
// | validation | rate-limited | network | server | upgrade-required.
//
// The enum lives in the kernel because every layer needs it; contracts only
// maps it to and from the JSON envelope.
enum class ErrorCode {
    Unknown = 0,
    InvalidArgument,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    Conflict,
    ValidationFailed,
    RateLimited,
    Network,
    Server,
    Storage,
    Timeout,
    Cancelled,
    Unsupported,
    UpgradeRequired,
};

// Wire name of a code, used in error envelopes and in logs.
[[nodiscard]] QString errorCodeName(ErrorCode code);

// Inverse of errorCodeName(); an unknown name maps to ErrorCode::Unknown.
[[nodiscard]] ErrorCode errorCodeFromName(const QString& name);

}  // namespace desktop::kernel
