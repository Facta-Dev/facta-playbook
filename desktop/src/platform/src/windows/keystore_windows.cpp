#include "desktop/platform/keystore.h"

#if defined(Q_OS_WINDOWS)

#include "../common/keystore_unavailable.h"

#include <memory>

namespace desktop::platform {

// ponytail: a stub. Replace it with the Windows Credential Manager (DPAPI)
// before shipping; the file fallback is not available here.
std::unique_ptr<Keystore> createKeystore() {
    return createUnavailableKeystore();
}

}  // namespace desktop::platform

#endif  // Q_OS_WINDOWS
