#include "desktop/platform/keystore.h"

#if defined(Q_OS_LINUX)

#include "../common/keystore_unavailable.h"

#include "desktop/kernel/error_codes.h"
#include "desktop/platform/app_paths.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <memory>
#include <utility>

namespace desktop::platform {
namespace {

// The Linux backend of this template.
//
// ponytail: known ceiling - the token lives in a 0600 file in the app data
// directory, not in the Secret Service. Replace it with libsecret/KWallet
// before shipping; the file backend stays as the headless fallback.
class FileKeystore final : public Keystore {
public:
    explicit FileKeystore(QString path) : path_(std::move(path)) {}

    [[nodiscard]] kernel::Result<QString> read(const QString& key) const override {
        QJsonObject items;
        const kernel::Result<void> loaded = loadItems(&items);
        if (!loaded.ok()) {
            return kernel::Result<QString>(loaded.errorCode(), loaded.errorMessage());
        }
        const QJsonValue value = items.value(key);
        if (!value.isString()) {
            return kernel::Result<QString>(kernel::ErrorCode::NotFound,
                                          QStringLiteral("no secret stored for this key"));
        }
        return kernel::Result<QString>(value.toString());
    }

    [[nodiscard]] kernel::Result<void> write(const QString& key,
                                              const QString& value) override {
        QJsonObject items;
        const kernel::Result<void> loaded = loadItems(&items);
        if (!loaded.ok()) {
            return loaded;
        }
        items.insert(key, value);
        return saveItems(items);
    }

    [[nodiscard]] kernel::Result<void> remove(const QString& key) override {
        QJsonObject items;
        const kernel::Result<void> loaded = loadItems(&items);
        if (!loaded.ok()) {
            return loaded;
        }
        items.remove(key);
        return saveItems(items);
    }

    [[nodiscard]] bool isSecure() const override { return false; }

private:
    [[nodiscard]] kernel::Result<void> loadItems(QJsonObject* items) const {
        QFile file(path_);
        if (!file.exists()) {
            *items = QJsonObject();
            return kernel::Result<void>();
        }
        if (!file.open(QIODevice::ReadOnly)) {
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("cannot read the secret file"));
        }
        const QByteArray raw = file.readAll();
        file.close();

        QJsonParseError parseError{};
        const QJsonDocument document = QJsonDocument::fromJson(raw, &parseError);
        if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("the secret file is corrupted"));
        }
        *items = document.object().value(QStringLiteral("items")).toObject();
        return kernel::Result<void>();
    }

    [[nodiscard]] kernel::Result<void> saveItems(const QJsonObject& items) const {
        const QFileInfo info(path_);
        if (!QDir().mkpath(info.absolutePath())) {
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("cannot create the app data directory"));
        }

        QJsonObject document;
        document.insert(QStringLiteral("items"), items);
        const QByteArray payload = QJsonDocument(document).toJson(QJsonDocument::Compact);

        QSaveFile file(path_);
        if (!file.open(QIODevice::WriteOnly)) {
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("cannot open the secret file for writing"));
        }
        if (file.write(payload) != payload.size()) {
            file.cancelWriting();
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("cannot write the secret file"));
        }
        if (!file.commit()) {
            return kernel::Result<void>(kernel::ErrorCode::Storage,
                                        QStringLiteral("cannot write the secret file"));
        }
        // The secret file is readable and writable by its owner only.
        QFile::setPermissions(path_, QFileDevice::ReadOwner | QFileDevice::WriteOwner);
        return kernel::Result<void>();
    }

    QString path_;
};

}  // namespace

std::unique_ptr<Keystore> createKeystore() {
    return std::make_unique<FileKeystore>(AppPaths::secretsFile());
}

}  // namespace desktop::platform

#endif  // Q_OS_LINUX
