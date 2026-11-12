#pragma once

#include "desktop/kernel/result.h"

#include <QString>
#include <memory>

namespace desktop::platform {

// The secret store that holds the OAuth refresh token (playbook 7).
//
// Known ceiling of this template: the Linux backend keeps the token in a 0600
// file in the app data directory instead of the Secret Service, and the
// Windows and macOS backends are stubs. Replace them with DPAPI, the
// Keychain and libsecret/KWallet before shipping.
class Keystore {
public:
    virtual ~Keystore() = default;

    [[nodiscard]] virtual kernel::Result<QString> read(const QString& key) const = 0;
    [[nodiscard]] virtual kernel::Result<void> write(const QString& key,
                                                      const QString& value) = 0;
    [[nodiscard]] virtual kernel::Result<void> remove(const QString& key) = 0;

    // False when the backend is the plain-file fallback or a stub.
    [[nodiscard]] virtual bool isSecure() const = 0;
};

// The backend of the host operating system.
[[nodiscard]] std::unique_ptr<Keystore> createKeystore();

}  // namespace desktop::platform
