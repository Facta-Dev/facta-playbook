#pragma once

#include <QString>

namespace desktop::platform {

// Every path the app may write. The paths come from QStandardPaths; nothing
// is ever written next to the executable (playbook 9).
namespace AppPaths {

[[nodiscard]] QString configDir();
[[nodiscard]] QString dataDir();
[[nodiscard]] QString cacheDir();
[[nodiscard]] QString logDir();

// Per-user runtime directory, used for the single-instance lock and socket.
[[nodiscard]] QString runtimeDir();

[[nodiscard]] QString databaseFile();

// The file the Linux keystore fallback writes the token to.
[[nodiscard]] QString secretsFile();

[[nodiscard]] QString lockFile();

}  // namespace AppPaths
}  // namespace desktop::platform
