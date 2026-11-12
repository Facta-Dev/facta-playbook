// The runtime check of the desktop template: kernel, contracts, platform and
// infrastructure. The formal test homes are tests/<slice>/desktop/; this
// suite lives with the layers it covers because the layer agent owns them.

#include <QtTest>

#include "desktop/contracts/error_envelope.h"
#include "desktop/contracts/note_dto.h"
#include "desktop/contracts/policy.h"
#include "desktop/contracts/realtime_envelope.h"
#include "desktop/infrastructure/storage/note_store.h"
#include "desktop/infrastructure/telemetry/logger.h"
#include "desktop/kernel/error_codes.h"
#include "desktop/kernel/result.h"
#include "desktop/kernel/time.h"
#include "desktop/platform/app_paths.h"
#include "desktop/platform/keystore.h"
#include "desktop/platform/single_instance.h"

#include <QFileInfo>
#include <QJsonObject>
#include <QStandardPaths>
#include <QTemporaryDir>

using desktop::contracts::ErrorEnvelope;
using desktop::contracts::NoteDto;
using desktop::contracts::Policy;
using desktop::contracts::RealtimeEnvelope;
using desktop::infrastructure::storage::NoteStore;
using desktop::infrastructure::telemetry::Category;
using desktop::infrastructure::telemetry::Logger;
using desktop::kernel::ErrorCode;
using desktop::kernel::Result;
using desktop::kernel::Timestamp;

class RuntimeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();

    void errorCodeNamesRoundTrip();
    void resultCarriesErrors();
    void noteDtoRoundTrip();
    void errorEnvelopeRoundTrip();
    void policyRejectsUnknownValues();
    void realtimeEnvelopeRoundTrip();
    void noteStoreMigratesAndStores();
    void loggerRedactsAndBounds();
    void keystoreStoresTokenPrivately();
    void singleInstanceGuardsTheApp();
};

void RuntimeTest::initTestCase() {
    // The app paths must not point at the real user data.
    QStandardPaths::setTestModeEnabled(true);
}

void RuntimeTest::errorCodeNamesRoundTrip() {
    QCOMPARE(desktop::kernel::errorCodeName(ErrorCode::PermissionDenied),
             QStringLiteral("forbidden"));
    QCOMPARE(desktop::kernel::errorCodeFromName(QStringLiteral("not-found")),
             ErrorCode::NotFound);
    QCOMPARE(desktop::kernel::errorCodeFromName(QStringLiteral("nonsense")),
             ErrorCode::Unknown);
}

void RuntimeTest::resultCarriesErrors() {
    const Result<int> ok(7);
    QVERIFY(ok.ok());
    QCOMPARE(ok.value(), 7);
    QCOMPARE(ok.errorCode(), ErrorCode::Unknown);

    const Result<int> failed(ErrorCode::Storage, QStringLiteral("disk full"));
    QVERIFY(!failed.ok());
    QCOMPARE(failed.errorCode(), ErrorCode::Storage);
    QCOMPARE(failed.errorMessage(), QStringLiteral("disk full"));
    QCOMPARE(failed.valueOr(3), 3);

    const Result<void> empty;
    QVERIFY(empty.ok());
    const Result<void> voidFailure(ErrorCode::NotFound, QStringLiteral("gone"));
    QVERIFY(!voidFailure.ok());
    QCOMPARE(voidFailure.errorCode(), ErrorCode::NotFound);
}

void RuntimeTest::noteDtoRoundTrip() {
    NoteDto note;
    note.id = desktop::kernel::NoteId(QStringLiteral("n-1"));
    note.title = QStringLiteral("Title");
    note.body = QStringLiteral("Body");
    note.createdAt = Timestamp::now();
    note.updatedAt = note.createdAt;

    bool ok = false;
    const NoteDto parsed = desktop::contracts::noteFromJson(desktop::contracts::toJson(note), &ok);
    QVERIFY(ok);
    QCOMPARE(parsed.id.value(), note.id.value());
    QCOMPARE(parsed.title, note.title);
    QCOMPARE(parsed.body, note.body);
    QCOMPARE(parsed.createdAt.toIso8601(), note.createdAt.toIso8601());
    QCOMPARE(parsed.updatedAt.toIso8601(), note.updatedAt.toIso8601());
}

void RuntimeTest::errorEnvelopeRoundTrip() {
    ErrorEnvelope envelope;
    envelope.code = ErrorCode::PermissionDenied;
    envelope.message = QStringLiteral("missing notes.write");
    envelope.requiredPermission = QStringLiteral("notes.write");

    bool ok = false;
    const ErrorEnvelope parsed =
        desktop::contracts::errorEnvelopeFromJson(desktop::contracts::toJson(envelope), &ok);
    QVERIFY(ok);
    QCOMPARE(parsed.code, ErrorCode::PermissionDenied);
    QCOMPARE(parsed.message, envelope.message);
    QCOMPARE(parsed.requiredPermission, envelope.requiredPermission);
}

void RuntimeTest::policyRejectsUnknownValues() {
    Policy policy = Policy::defaults();
    policy.tenantUrl = QStringLiteral("https://example.test");
    policy.featureFlags.insert(QStringLiteral("beta"), true);

    bool ok = false;
    const Policy parsed =
        desktop::contracts::policyFromJson(desktop::contracts::toJson(policy), &ok);
    QVERIFY(ok);
    QCOMPARE(parsed.tenantUrl, policy.tenantUrl);
    QCOMPARE(parsed.featureFlags.value(QStringLiteral("beta")), true);
    QCOMPARE(parsed.realtimeMode, policy.realtimeMode);

    QJsonObject invalid = desktop::contracts::toJson(policy);
    invalid.insert(QStringLiteral("Unknown"), 1);
    ok = true;
    const Policy rejected = desktop::contracts::policyFromJson(invalid, &ok);
    QVERIFY(!ok);
    QCOMPARE(rejected.tenantUrl, policy.tenantUrl);
}

void RuntimeTest::realtimeEnvelopeRoundTrip() {
    RealtimeEnvelope envelope;
    envelope.id = QStringLiteral("e-1");
    envelope.type = QStringLiteral("notes.updated");
    envelope.tenantId = QStringLiteral("t-1");
    envelope.topic = QStringLiteral("notes");
    envelope.correlationId = QStringLiteral("c-1");
    envelope.timestamp = Timestamp::now();
    envelope.payload.insert(QStringLiteral("id"), QStringLiteral("n-1"));

    bool ok = false;
    const RealtimeEnvelope parsed = desktop::contracts::realtimeEnvelopeFromJson(
        desktop::contracts::toJson(envelope), &ok);
    QVERIFY(ok);
    QCOMPARE(parsed.id, envelope.id);
    QCOMPARE(parsed.type, envelope.type);
    QCOMPARE(parsed.tenantId, envelope.tenantId);
    QCOMPARE(parsed.topic, envelope.topic);
    QCOMPARE(parsed.correlationId, envelope.correlationId);
    QCOMPARE(parsed.version, 1);
    QCOMPARE(parsed.payload.value(QStringLiteral("id")).toString(), QStringLiteral("n-1"));
}

void RuntimeTest::noteStoreMigratesAndStores() {
    QTemporaryDir directory;
    QVERIFY(directory.isValid());

    Result<NoteStore> opened = NoteStore::open(directory.filePath(QStringLiteral("notes.sqlite")));
    QVERIFY(opened.ok());

    const Result<int> version = opened.value().schemaVersion();
    QVERIFY(version.ok());
    QCOMPARE(version.value(), 1);

    NoteDto note;
    note.id = desktop::kernel::NoteId(QStringLiteral("n-1"));
    note.title = QStringLiteral("Title");
    note.body = QStringLiteral("Body");
    note.createdAt = Timestamp::now();
    note.updatedAt = note.createdAt;

    const Result<NoteDto> stored = opened.value().upsertNote(note);
    QVERIFY(stored.ok());

    const Result<QVector<NoteDto>> notes = opened.value().listNotes();
    QVERIFY(notes.ok());
    QCOMPARE(static_cast<int>(notes.value().size()), 1);
    QCOMPARE(notes.value().first().title, note.title);

    const Result<void> removed = opened.value().removeNote(note.id.value());
    QVERIFY(removed.ok());
    QCOMPARE(static_cast<int>(opened.value().listNotes().value().size()), 0);
}

void RuntimeTest::loggerRedactsAndBounds() {
    Logger logger(2);
    logger.log(Category::Http, QStringLiteral("Authorization Bearer abc.def-123"));
    logger.log(Category::App, QStringLiteral("note saved"),
               QMap<QString, QString>{{QStringLiteral("refresh_token"), QStringLiteral("secret")}});

    QCOMPARE(static_cast<int>(logger.entries().size()), 2);
    QVERIFY(!logger.entries().first().message.contains(QStringLiteral("abc.def-123")));
    QCOMPARE(logger.entries().at(1).fields.value(QStringLiteral("refresh_token")),
             QStringLiteral("***"));

    logger.log(Category::App, QStringLiteral("third"));
    QCOMPARE(static_cast<int>(logger.entries().size()), 2);
    QCOMPARE(logger.entries().first().message, QStringLiteral("note saved"));
}

void RuntimeTest::keystoreStoresTokenPrivately() {
    std::unique_ptr<desktop::platform::Keystore> keystore = desktop::platform::createKeystore();
    QVERIFY(keystore != nullptr);

    const Result<void> written =
        keystore->write(QStringLiteral("refresh_token"), QStringLiteral("secret-value"));
    if (!written.ok() && written.errorCode() == ErrorCode::Unsupported) {
        QSKIP("no OS keystore backend on this platform");
    }
    QVERIFY(written.ok());

    const Result<QString> read = keystore->read(QStringLiteral("refresh_token"));
    QVERIFY(read.ok());
    QCOMPARE(read.value(), QStringLiteral("secret-value"));

#if defined(Q_OS_LINUX)
    const QFileInfo info(desktop::platform::AppPaths::secretsFile());
    QVERIFY(info.exists());
    const QFileDevice::Permissions permissions = info.permissions();
    QVERIFY(!(permissions & QFileDevice::ReadGroup));
    QVERIFY(!(permissions & QFileDevice::ReadOther));
    QVERIFY(!(permissions & QFileDevice::WriteGroup));
    QVERIFY(!(permissions & QFileDevice::WriteOther));
#endif

    const Result<void> removed = keystore->remove(QStringLiteral("refresh_token"));
    QVERIFY(removed.ok());
    QVERIFY(!keystore->read(QStringLiteral("refresh_token")).ok());
}

void RuntimeTest::singleInstanceGuardsTheApp() {
    desktop::platform::SingleInstanceGuard primary(QStringLiteral("desktop-runtime-test"));
    QVERIFY(primary.isPrimary());

    bool received = false;
    primary.listen([&received](const QByteArray& message) {
        if (message == QByteArrayLiteral("ping")) {
            received = true;
        }
    });

    desktop::platform::SingleInstanceGuard second(QStringLiteral("desktop-runtime-test"));
    QVERIFY(!second.isPrimary());
    QVERIFY(second.notifyPrimary(QByteArrayLiteral("ping")));
    QTRY_VERIFY_WITH_TIMEOUT(received, 2000);
}

QTEST_MAIN(RuntimeTest)
#include "tst_runtime.moc"
