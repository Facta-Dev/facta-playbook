// The contracts: the JSON shapes that cross the wire and the storage boundary.

#include "desktop/contracts/error_envelope.h"
#include "desktop/contracts/note_dto.h"
#include "desktop/contracts/policy.h"
#include "desktop/contracts/realtime_envelope.h"
#include "desktop/kernel/error_codes.h"
#include "desktop/kernel/ids.h"
#include "desktop/kernel/time.h"

#include <QJsonObject>
#include <QTest>

namespace {

using desktop::contracts::ErrorEnvelope;
using desktop::contracts::NoteDto;
using desktop::contracts::Policy;
using desktop::contracts::RealtimeEnvelope;
using desktop::kernel::ErrorCode;
using desktop::kernel::NoteId;
using desktop::kernel::Timestamp;

NoteDto makeNote() {
    NoteDto note;
    note.id = NoteId(QStringLiteral("n-1"));
    note.title = QStringLiteral("Title");
    note.body = QStringLiteral("Body");
    note.createdAt = Timestamp::now();
    note.updatedAt = note.createdAt;
    return note;
}

}  // namespace

class ContractsTests : public QObject {
    Q_OBJECT

private slots:
    void noteRoundTripsThroughJson();
    void noteWithoutAnIdentifierIsRejected();
    void noteWithABrokenTimestampIsRejected();
    void errorEnvelopeRoundTripsThroughJson();
    void errorEnvelopeWithAnUnknownCodeIsRejected();
    void realtimeEnvelopeRoundTripsThroughJson();
    void realtimeEnvelopeWithoutATypeIsRejected();
    void policyRoundTripsThroughJson();
    void policyWithAnUnknownKeyIsRejected();
};

void ContractsTests::noteRoundTripsThroughJson() {
    const NoteDto note = makeNote();

    bool ok = false;
    const NoteDto parsed = desktop::contracts::noteFromJson(desktop::contracts::toJson(note), &ok);

    QVERIFY(ok);
    QCOMPARE(parsed.id.value(), note.id.value());
    QCOMPARE(parsed.title, note.title);
    QCOMPARE(parsed.body, note.body);
    QCOMPARE(parsed.createdAt.toIso8601(), note.createdAt.toIso8601());
    QCOMPARE(parsed.updatedAt.toIso8601(), note.updatedAt.toIso8601());
}

void ContractsTests::noteWithoutAnIdentifierIsRejected() {
    QJsonObject object = desktop::contracts::toJson(makeNote());
    object.remove(QStringLiteral("id"));

    bool ok = true;
    const NoteDto parsed = desktop::contracts::noteFromJson(object, &ok);

    QVERIFY(!ok);
    QVERIFY(!parsed.id.isValid());
}

void ContractsTests::noteWithABrokenTimestampIsRejected() {
    QJsonObject object = desktop::contracts::toJson(makeNote());
    object.insert(QStringLiteral("createdAt"), QStringLiteral("not-a-timestamp"));

    bool ok = true;
    const NoteDto parsed = desktop::contracts::noteFromJson(object, &ok);

    QVERIFY(!ok);
    QVERIFY(!parsed.createdAt.isValid());
}

void ContractsTests::errorEnvelopeRoundTripsThroughJson() {
    ErrorEnvelope envelope;
    envelope.code = ErrorCode::PermissionDenied;
    envelope.message = QStringLiteral("missing notes.write");
    envelope.requestId = QStringLiteral("r-1");
    envelope.requiredPermission = QStringLiteral("notes.write");

    bool ok = false;
    const ErrorEnvelope parsed = desktop::contracts::errorEnvelopeFromJson(
        desktop::contracts::toJson(envelope), &ok);

    QVERIFY(ok);
    QCOMPARE(parsed.code, ErrorCode::PermissionDenied);
    QCOMPARE(parsed.message, envelope.message);
    QCOMPARE(parsed.requestId, envelope.requestId);
    QCOMPARE(parsed.requiredPermission, envelope.requiredPermission);
}

void ContractsTests::errorEnvelopeWithAnUnknownCodeIsRejected() {
    QJsonObject object;
    object.insert(QStringLiteral("code"), QStringLiteral("not-a-code"));
    object.insert(QStringLiteral("detail"), QStringLiteral("broken"));

    bool ok = true;
    const ErrorEnvelope parsed = desktop::contracts::errorEnvelopeFromJson(object, &ok);

    QVERIFY(!ok);
    QCOMPARE(parsed.code, ErrorCode::Unknown);
}

void ContractsTests::realtimeEnvelopeRoundTripsThroughJson() {
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

void ContractsTests::realtimeEnvelopeWithoutATypeIsRejected() {
    QJsonObject object = desktop::contracts::toJson([] {
        RealtimeEnvelope envelope;
        envelope.id = QStringLiteral("e-1");
        envelope.timestamp = Timestamp::now();
        return envelope;
    }());
    object.remove(QStringLiteral("type"));

    bool ok = true;
    const RealtimeEnvelope parsed = desktop::contracts::realtimeEnvelopeFromJson(object, &ok);

    QVERIFY(!ok);
    QVERIFY(parsed.type.isEmpty());
}

void ContractsTests::policyRoundTripsThroughJson() {
    Policy policy = Policy::defaults();
    policy.tenantUrl = QStringLiteral("https://example.test");
    policy.ssoMode = desktop::contracts::SsoMode::Local;
    policy.updateChannel = desktop::contracts::UpdateChannel::Beta;
    policy.realtimeMode = desktop::contracts::RealtimeMode::WebSocket;
    policy.telemetryLevel = desktop::contracts::TelemetryLevel::Full;
    policy.compatibilityMode = desktop::contracts::CompatibilityMode::Off;
    policy.killSwitch = true;
    policy.dataRetentionDays = 90;
    policy.allowUserSettings = false;
    policy.featureFlags.insert(QStringLiteral("beta"), true);

    bool ok = false;
    const Policy parsed =
        desktop::contracts::policyFromJson(desktop::contracts::toJson(policy), &ok);

    QVERIFY(ok);
    QCOMPARE(parsed.tenantUrl, policy.tenantUrl);
    QCOMPARE(parsed.ssoMode, policy.ssoMode);
    QCOMPARE(parsed.updateChannel, policy.updateChannel);
    QCOMPARE(parsed.realtimeMode, policy.realtimeMode);
    QCOMPARE(parsed.telemetryLevel, policy.telemetryLevel);
    QCOMPARE(parsed.compatibilityMode, policy.compatibilityMode);
    QCOMPARE(parsed.killSwitch, policy.killSwitch);
    QCOMPARE(parsed.dataRetentionDays, policy.dataRetentionDays);
    QCOMPARE(parsed.allowUserSettings, policy.allowUserSettings);
    QCOMPARE(parsed.featureFlags.value(QStringLiteral("beta")), true);
}

void ContractsTests::policyWithAnUnknownKeyIsRejected() {
    QJsonObject object = desktop::contracts::toJson(Policy::defaults());
    object.insert(QStringLiteral("Unknown"), 1);

    bool ok = true;
    const Policy parsed = desktop::contracts::policyFromJson(object, &ok);

    QVERIFY(!ok);
    QCOMPARE(parsed.tenantUrl, Policy::defaults().tenantUrl);
}

QTEST_MAIN(ContractsTests)
#include "tst_contracts.moc"
