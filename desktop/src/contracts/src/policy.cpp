#include "desktop/contracts/policy.h"

#include <QJsonValue>
#include <QSet>
#include <array>
#include <utility>

namespace desktop::contracts {
namespace {

template <class Enum, size_t N>
QString enumName(Enum value, const std::array<std::pair<Enum, const char*>, N>& table) {
    for (const auto& entry : table) {
        if (entry.first == value) {
            return QString::fromLatin1(entry.second);
        }
    }
    return QString();
}

template <class Enum, size_t N>
bool enumFromName(const QString& text, const std::array<std::pair<Enum, const char*>, N>& table,
                  Enum* out) {
    for (const auto& entry : table) {
        if (text == QLatin1String(entry.second)) {
            *out = entry.first;
            return true;
        }
    }
    return false;
}

constexpr std::array<std::pair<SsoMode, const char*>, 2> kSsoModes = {
    {{SsoMode::Oidc, "oidc"}, {SsoMode::Local, "local"}}};

constexpr std::array<std::pair<UpdateChannel, const char*>, 3> kUpdateChannels = {
    {{UpdateChannel::Stable, "stable"}, {UpdateChannel::Beta, "beta"},
     {UpdateChannel::Canary, "canary"}}};

constexpr std::array<std::pair<RealtimeMode, const char*>, 5> kRealtimeModes = {
    {{RealtimeMode::Auto, "auto"}, {RealtimeMode::WebSocket, "websocket"},
     {RealtimeMode::Sse, "sse"}, {RealtimeMode::Polling, "polling"}, {RealtimeMode::Off, "off"}}};

constexpr std::array<std::pair<TelemetryLevel, const char*>, 3> kTelemetryLevels = {
    {{TelemetryLevel::Off, "off"}, {TelemetryLevel::Errors, "errors"},
     {TelemetryLevel::Full, "full"}}};

constexpr std::array<std::pair<CompatibilityMode, const char*>, 3> kCompatibilityModes = {
    {{CompatibilityMode::Auto, "auto"}, {CompatibilityMode::Force, "force"},
     {CompatibilityMode::Off, "off"}}};

const QSet<QString>& knownPolicyKeys() {
    static const QSet<QString> kKeys = {
        QStringLiteral("TenantUrl"),     QStringLiteral("SsoMode"),
        QStringLiteral("UpdatesEnabled"), QStringLiteral("UpdateChannel"),
        QStringLiteral("RealtimeMode"), QStringLiteral("TelemetryLevel"),
        QStringLiteral("CompatibilityMode"), QStringLiteral("KillSwitch"),
        QStringLiteral("FeatureFlags"), QStringLiteral("DataRetentionDays"),
        QStringLiteral("AllowUserSettings")};
    return kKeys;
}

}  // namespace

Policy Policy::defaults() {
    return Policy{};
}

QJsonObject toJson(const Policy& policy) {
    QJsonObject flags;
    for (auto it = policy.featureFlags.constBegin(); it != policy.featureFlags.constEnd();
         ++it) {
        flags.insert(it.key(), it.value());
    }

    QJsonObject object;
    object.insert(QStringLiteral("TenantUrl"), policy.tenantUrl);
    object.insert(QStringLiteral("SsoMode"), enumName(policy.ssoMode, kSsoModes));
    object.insert(QStringLiteral("UpdatesEnabled"), policy.updatesEnabled);
    object.insert(QStringLiteral("UpdateChannel"), enumName(policy.updateChannel, kUpdateChannels));
    object.insert(QStringLiteral("RealtimeMode"), enumName(policy.realtimeMode, kRealtimeModes));
    object.insert(QStringLiteral("TelemetryLevel"),
                  enumName(policy.telemetryLevel, kTelemetryLevels));
    object.insert(QStringLiteral("CompatibilityMode"),
                  enumName(policy.compatibilityMode, kCompatibilityModes));
    object.insert(QStringLiteral("KillSwitch"), policy.killSwitch);
    object.insert(QStringLiteral("FeatureFlags"), flags);
    object.insert(QStringLiteral("DataRetentionDays"), policy.dataRetentionDays);
    object.insert(QStringLiteral("AllowUserSettings"), policy.allowUserSettings);
    return object;
}

Policy policyFromJson(const QJsonObject& object, bool* ok) {
    Policy policy = Policy::defaults();
    bool valid = true;

    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        if (!knownPolicyKeys().contains(it.key())) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("TenantUrl"))) {
        if (!object.value(QStringLiteral("TenantUrl")).isString()) {
            valid = false;
        }
        policy.tenantUrl = object.value(QStringLiteral("TenantUrl")).toString();
    }

    if (object.contains(QStringLiteral("SsoMode"))) {
        const QString name = object.value(QStringLiteral("SsoMode")).toString();
        if (!enumFromName(name, kSsoModes, &policy.ssoMode)) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("UpdatesEnabled"))) {
        const QJsonValue value = object.value(QStringLiteral("UpdatesEnabled"));
        if (!value.isBool()) {
            valid = false;
        }
        policy.updatesEnabled = value.toBool();
    }

    if (object.contains(QStringLiteral("UpdateChannel"))) {
        const QString name = object.value(QStringLiteral("UpdateChannel")).toString();
        if (!enumFromName(name, kUpdateChannels, &policy.updateChannel)) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("RealtimeMode"))) {
        const QString name = object.value(QStringLiteral("RealtimeMode")).toString();
        if (!enumFromName(name, kRealtimeModes, &policy.realtimeMode)) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("TelemetryLevel"))) {
        const QString name = object.value(QStringLiteral("TelemetryLevel")).toString();
        if (!enumFromName(name, kTelemetryLevels, &policy.telemetryLevel)) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("CompatibilityMode"))) {
        const QString name = object.value(QStringLiteral("CompatibilityMode")).toString();
        if (!enumFromName(name, kCompatibilityModes, &policy.compatibilityMode)) {
            valid = false;
        }
    }

    if (object.contains(QStringLiteral("KillSwitch"))) {
        const QJsonValue value = object.value(QStringLiteral("KillSwitch"));
        if (!value.isBool()) {
            valid = false;
        }
        policy.killSwitch = value.toBool();
    }

    if (object.contains(QStringLiteral("FeatureFlags"))) {
        const QJsonObject flags = object.value(QStringLiteral("FeatureFlags")).toObject();
        for (auto it = flags.constBegin(); it != flags.constEnd(); ++it) {
            if (!it.value().isBool()) {
                valid = false;
                continue;
            }
            policy.featureFlags.insert(it.key(), it.value().toBool());
        }
    }

    if (object.contains(QStringLiteral("DataRetentionDays"))) {
        const QJsonValue value = object.value(QStringLiteral("DataRetentionDays"));
        const double raw = value.toDouble();
        const int days = static_cast<int>(raw);
        if (!value.isDouble() || days < 0 || static_cast<double>(days) != raw) {
            valid = false;
        } else {
            policy.dataRetentionDays = days;
        }
    }

    if (object.contains(QStringLiteral("AllowUserSettings"))) {
        const QJsonValue value = object.value(QStringLiteral("AllowUserSettings"));
        if (!value.isBool()) {
            valid = false;
        }
        policy.allowUserSettings = value.toBool();
    }

    if (ok != nullptr) {
        *ok = valid;
    }
    return policy;
}

}  // namespace desktop::contracts
