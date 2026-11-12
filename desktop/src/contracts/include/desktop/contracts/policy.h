#pragma once

#include <QJsonObject>
#include <QMap>
#include <QString>

namespace desktop::contracts {

// The managed policy surface (playbook 12.1 and appendix F). The JSON form is
// the schema of /etc/<product>/policy.json; the same keys are used by the
// ADMX and plist sources.
enum class SsoMode { Oidc, Local };
enum class UpdateChannel { Stable, Beta, Canary };
enum class RealtimeMode { Auto, WebSocket, Sse, Polling, Off };
enum class TelemetryLevel { Off, Errors, Full };
enum class CompatibilityMode { Auto, Force, Off };

struct Policy {
    QString tenantUrl;
    SsoMode ssoMode = SsoMode::Oidc;
    bool updatesEnabled = true;
    UpdateChannel updateChannel = UpdateChannel::Stable;
    RealtimeMode realtimeMode = RealtimeMode::Auto;
    TelemetryLevel telemetryLevel = TelemetryLevel::Errors;
    CompatibilityMode compatibilityMode = CompatibilityMode::Auto;
    bool killSwitch = false;
    QMap<QString, bool> featureFlags;
    int dataRetentionDays = 30;
    bool allowUserSettings = true;

    // The built-in safe defaults, used when no policy source provides a value.
    [[nodiscard]] static Policy defaults();
};

[[nodiscard]] QJsonObject toJson(const Policy& policy);

// Reads a policy document. An unknown key, an unknown enum value or a value
// of the wrong type sets *ok to false and keeps the defaults.
[[nodiscard]] Policy policyFromJson(const QJsonObject& object, bool* ok = nullptr);

}  // namespace desktop::contracts
