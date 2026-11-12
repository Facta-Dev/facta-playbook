# Admin Guide

How an enterprise configures the desktop client: the policy keys, where the
policy comes from on each operating system, and how to publish and verify
it.

> **Known gap.** The template ships the **policy schema**
> (`src/contracts/include/desktop/contracts/policy.h`) and its parser, but
> **no reader**: nothing loads a policy from the operating system yet. The
> sources below are the contract the reader must implement. Until it exists,
> the app always runs with `Policy::defaults()`.

## 1 · The policy keys

The policy surface is eleven keys, declared in one struct and parsed from
one JSON shape. The same key names are used by the three sources, so a
policy written for one platform reads the same on the others.

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `TenantUrl` | string | empty | The base URL of the backend the app talks to |
| `SsoMode` | `oidc` \| `local` | `oidc` | The sign-in mode |
| `UpdatesEnabled` | boolean | `true` | Whether the app may check for and install updates |
| `UpdateChannel` | `stable` \| `beta` \| `canary` | `stable` | The release stream the app follows |
| `RealtimeMode` | `auto` \| `websocket` \| `sse` \| `polling` \| `off` | `auto` | The realtime transport |
| `TelemetryLevel` | `off` \| `errors` \| `full` | `errors` | How much telemetry may leave the device |
| `CompatibilityMode` | `auto` \| `force` \| `off` | `auto` | `force` starts the app in software rendering ([03](03-no-gpu-fallback.md)) |
| `KillSwitch` | boolean | `false` | The app refuses to start |
| `FeatureFlags` | object of booleans | empty | Per-capability switches |
| `DataRetentionDays` | integer | `30` | How long local data is kept |
| `AllowUserSettings` | boolean | `true` | Whether the user may change the settings the policy controls |

Three parsing rules, enforced by `policyFromJson()`:

- **An unknown key fails the parse.** The app keeps the defaults; it never
  runs with a half-understood policy.
- **An unknown enum value fails the parse.**
- **A value of the wrong type fails the parse.**

The last two are deliberate: a typo in a managed policy must not silently
change the security posture.

## 2 · The source per operating system

The app reads the policy from the **first** source that provides it, in the
order below. A source that does not exist is not an error.

| Priority | Platform | Source | Notes |
| --- | --- | --- | --- |
| 1 | Windows | The registry: `HKLM\Software\<vendor>\<product>` | Written by Group Policy; needs an ADMX template |
| 2 | macOS | `/Library/Managed Preferences/<bundle-id>.plist` | Managed by an MDM server (Jamf, Intune) |
| 3 | Linux | `/etc/<product>/policy.json` | A plain JSON file, written by configuration management |
| 4 | Any | The user's own settings | Only when `AllowUserSettings` is true |

### Windows — the registry and the ADMX

The policy lives in the registry, under the product key:

```
HKLM\Software\<vendor>\<product>
    TenantUrl          REG_SZ
    SsoMode            REG_SZ
    UpdatesEnabled     REG_DWORD
    UpdateChannel      REG_SZ
    RealtimeMode       REG_SZ
    TelemetryLevel     REG_SZ
    CompatibilityMode  REG_SZ
    KillSwitch         REG_DWORD
    DataRetentionDays  REG_DWORD
    AllowUserSettings  REG_DWORD
    FeatureFlags       REG_SZ   (a JSON object, serialized)
```

The registry values are the projection of a Group Policy setting, so an
administrator never edits them by hand: the **ADMX template** projects them
into the Group Policy editor, where they live next to the other policies of
the machine.

**What the enterprise does:**

1. Deploys the ADMX template to the domain (`SYSVOL\…\PolicyDefinitions`).
2. Creates a Group Policy Object that targets the computers.
3. Configures the settings under the product's node.
4. Runs `gpupdate /force`, or waits for the refresh interval.

The application reads the registry on startup, and **re-reads it** when the
policy says so — a change must not require the user to restart the app. The
polling interval is a product decision; a reasonable default is once a
minute, and it must be cheap (a registry read).

### macOS — the managed preferences

The policy lives in a plist owned by root:

```
/Library/Managed Preferences/<bundle-id>.plist
```

The file is written by the MDM server. It is **not** writable by the user,
which is what makes it managed: a user cannot edit it, and the app must
treat it as read-only.

```xml
<dict>
    <key>TenantUrl</key><string>https://app.example.com</string>
    <key>SsoMode</key><string>oidc</string>
    <key>UpdatesEnabled</key><false/>
    <key>UpdateChannel</key><string>stable</string>
    <key>RealtimeMode</key><string>auto</string>
    <key>TelemetryLevel</key><string>errors</string>
    <key>CompatibilityMode</key><string>auto</string>
    <key>KillSwitch</key><false/>
    <key>DataRetentionDays</key><integer>30</integer>
    <key>AllowUserSettings</key><true/>
    <key>FeatureFlags</key><dict><key>betaEditor</key><true/></dict>
</dict>
```

**What the enterprise does:**

1. Creates a custom payload in the MDM console for the `<bundle-id>`
   application.
2. Sets the keys above in the payload.
3. Scopes the payload to the devices or groups that need it.

The plist is not read on every poll by choice: it is cached and re-read on a
timer, because the MDM server rewrites the file atomically.

### Linux — the policy file

The policy lives in a JSON file, owned by root:

```
/etc/<product>/policy.json
```

```json
{
    "TenantUrl": "https://app.example.com",
    "SsoMode": "oidc",
    "UpdatesEnabled": false,
    "UpdateChannel": "stable",
    "RealtimeMode": "auto",
    "TelemetryLevel": "errors",
    "CompatibilityMode": "auto",
    "KillSwitch": false,
    "DataRetentionDays": 30,
    "AllowUserSettings": true,
    "FeatureFlags": { "betaEditor": true }
}
```

**What the enterprise does:**

1. Writes the file from the configuration management system (Ansible, Puppet,
   Chef, or a plain package).
2. Sets the ownership to `root:root` and the mode to `0644`.
3. Restarts the app, or lets it re-read on its timer.

The file must be world-readable and **not** user-writable: a policy a user
can edit is not a policy.

### The user's own settings

When no managed source provides a value, the app falls back to the user's
own settings — **but only for the keys that `AllowUserSettings` covers**. A
managed key is never overridden by the user, whatever that key says.

## 3 · Precedence

| Rule | Effect |
| --- | --- |
| A managed source always wins over the user's settings | The user's setting is ignored for a managed key |
| The first source that provides a value wins | Windows → macOS → Linux → the user's settings |
| The strictest value wins for `CompatibilityMode` and `TelemetryLevel` | `force` beats `auto`; `off` beats `errors` |
| `KillSwitch` is absolute | Nothing overrides it; the app does not start |
| `TenantUrl` is not a permission | The server still authorizes every request |

The last row matters: a policy can point the app at a different backend, but
**the URL never authorizes**. The server decides what the session may do.

## 4 · Verifying a deployment

An administrator needs to answer three questions: is the policy applied,
what does the app think it is, and what did the app actually do.

| Check | How |
| --- | --- |
| The file or the registry value exists | The platform's own tooling: `reg query`, `plutil -p`, `cat` |
| The app reads it | The app shows the **effective policy** in its diagnostics screen, with the secret values redacted |
| The app acted on it | The log line of the policy load names the source and the version of the document |
| The behaviour changed | The corresponding feature is visibly in the new state |

The last two are the reason the effective policy belongs in the diagnostics
bundle ([10](10-telemetry.md)): an administrator who cannot see what the app
believes cannot debug a deployment.

## 5 · What the administrator can change

| Key | Effect the administrator sees |
| --- | --- |
| `TenantUrl` | The app talks to a different backend |
| `SsoMode` | The sign-in screen changes |
| `UpdatesEnabled` | The app stops checking for updates entirely |
| `UpdateChannel` | The app follows a different release stream |
| `RealtimeMode` | The realtime transport changes, or is turned off |
| `TelemetryLevel` | The amount of telemetry the app may send |
| `CompatibilityMode` | The app starts in software rendering |
| `KillSwitch` | The app refuses to start |
| `FeatureFlags` | Individual capabilities appear or disappear |
| `DataRetentionDays` | Local data older than the limit is deleted |
| `AllowUserSettings` | The user may or may not change the settings |

## 6 · The gap, and what it takes to close it

| Missing | File that must change |
| --- | --- |
| The reader for the Windows registry | A new source in `src/platform/src/windows/` |
| The reader for the macOS plist | A new source in `src/platform/src/macos/` |
| The reader for the Linux file | A new source in `src/platform/src/linux/` |
| The composition of the sources by priority | A new file in `src/infrastructure/` — the platform layer returns the raw document, the infrastructure layer merges them |
| The re-read timer | The infrastructure layer |
| The effective-policy screen | `src/app/` |

The seam is already in place: `desktop::contracts::Policy` is the
destination of the parse, and the platform layer is where an OS-specific
reader belongs. The design rule that applies is the one from
[04](04-hexagonal-layers-and-mvvm.md): the platform layer returns a
document, and the infrastructure layer decides what to do with it.

## 7 · Security rules

| Rule | Why |
| --- | --- |
| A managed policy file is owned by root and is not user-writable | A policy the user can edit is not a policy |
| The token never appears in a policy | The token belongs in the keystore ([07](07-threading-and-the-gui-thread.md)) |
| A policy value is never logged verbatim | Some values are sensitive; the effective policy is redacted in the bundle |
| A malformed policy never becomes a partial policy | The defaults are used as a whole |
| `KillSwitch` needs no network | It must work on a machine that cannot reach the network |

## See also

- [The No-GPU Fallback](03-no-gpu-fallback.md) — the `CompatibilityMode`
  key in action.
- [Packaging & Updates](11-packaging-and-updates.md) — the update channel the
  policy selects.
- [Telemetry](10-telemetry.md) — the `TelemetryLevel` key.
- [`src/contracts/include/desktop/contracts/policy.h`](../src/contracts/include/desktop/contracts/policy.h)
