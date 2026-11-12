#pragma once

#include "desktop/platform/keystore.h"

#include <memory>
#include <utility>

namespace desktop::platform {

// The stub backend used where this template has no OS keystore yet.
//
// ponytail: a stub, not an implementation. Windows (DPAPI) and macOS
// (Keychain) replace it before shipping.
class UnavailableKeystore final : public Keystore {
public:
    [[nodiscard]] kernel::Result<QString> read(const QString&) const override {
        return kernel::Result<QString>(
            kernel::ErrorCode::Unsupported,
            QStringLiteral("no OS keystore backend in this template build"));
    }

    [[nodiscard]] kernel::Result<void> write(const QString&, const QString&) override {
        return kernel::Result<void>(kernel::ErrorCode::Unsupported,
                                    QStringLiteral("no OS keystore backend in this template build"));
    }

    [[nodiscard]] kernel::Result<void> remove(const QString&) override {
        return kernel::Result<void>(kernel::ErrorCode::Unsupported,
                                    QStringLiteral("no OS keystore backend in this template build"));
    }

    [[nodiscard]] bool isSecure() const override { return false; }
};

inline std::unique_ptr<Keystore> createUnavailableKeystore() {
    return std::make_unique<UnavailableKeystore>();
}

}  // namespace desktop::platform
