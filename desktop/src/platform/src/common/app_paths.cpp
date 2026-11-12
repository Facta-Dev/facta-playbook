#include "desktop/platform/app_paths.h"

#include <QStandardPaths>

namespace desktop::platform {
namespace AppPaths {
namespace {

QString withFileName(const QString& directory, const QString& fileName) {
    return directory + QLatin1Char('/') + fileName;
}

}  // namespace

QString configDir() {
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
}

QString dataDir() {
    return QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
}

QString cacheDir() {
    return QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
}

QString logDir() {
    return withFileName(dataDir(), QStringLiteral("logs"));
}

QString runtimeDir() {
    const QString runtime = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    if (!runtime.isEmpty()) {
        return runtime;
    }
    return QStandardPaths::writableLocation(QStandardPaths::TempLocation);
}

QString databaseFile() {
    return withFileName(dataDir(), QStringLiteral("notes.sqlite"));
}

QString secretsFile() {
    return withFileName(dataDir(), QStringLiteral("secrets.json"));
}

QString lockFile() {
    return withFileName(runtimeDir(), QStringLiteral("instance.lock"));
}

}  // namespace AppPaths
}  // namespace desktop::platform
