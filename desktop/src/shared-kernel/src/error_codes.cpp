#include "desktop/kernel/error_codes.h"

#include <QHash>

namespace desktop::kernel {
namespace {

struct CodeName {
    ErrorCode code;
    const char* name;
};

// The single mapping between the enum and the wire taxonomy.
constexpr CodeName kCodeNames[] = {
    {ErrorCode::Unknown, "unknown"},
    {ErrorCode::InvalidArgument, "invalid-argument"},
    {ErrorCode::Unauthenticated, "unauthenticated"},
    {ErrorCode::PermissionDenied, "forbidden"},
    {ErrorCode::NotFound, "not-found"},
    {ErrorCode::Conflict, "conflict"},
    {ErrorCode::ValidationFailed, "validation"},
    {ErrorCode::RateLimited, "rate-limited"},
    {ErrorCode::Network, "network"},
    {ErrorCode::Server, "server"},
    {ErrorCode::Storage, "storage"},
    {ErrorCode::Timeout, "timeout"},
    {ErrorCode::Cancelled, "cancelled"},
    {ErrorCode::Unsupported, "unsupported"},
    {ErrorCode::UpgradeRequired, "upgrade-required"},
};

}  // namespace

QString errorCodeName(ErrorCode code) {
    for (const CodeName& entry : kCodeNames) {
        if (entry.code == code) {
            return QString::fromLatin1(entry.name);
        }
    }
    return QStringLiteral("unknown");
}

ErrorCode errorCodeFromName(const QString& name) {
    static const QHash<QString, ErrorCode> kByName = [] {
        QHash<QString, ErrorCode> map;
        for (const CodeName& entry : kCodeNames) {
            map.insert(QString::fromLatin1(entry.name), entry.code);
        }
        return map;
    }();

    return kByName.value(name, ErrorCode::Unknown);
}

}  // namespace desktop::kernel
