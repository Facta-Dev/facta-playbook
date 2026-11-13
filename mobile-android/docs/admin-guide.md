# Admin Guide

How an administrator configures and verifies a native Android deployment:
the managed configuration, the remote configuration, the Play Console
setup, the signing material, and the checks that prove a deployment is what
it claims to be.

> **Known gap — do not copy a fiction.** The template ships **no
> configuration reader, no Play integration, no signing configuration and no
> diagnostics UI**. It ships the seams: a `:core:kernel` that can carry a
> typed configuration value, a `:app` composition root that builds the
> graph, and
> the chapters that specify what a project must add. Every section below
> states what is missing and what it takes to close the gap.

## 1 · The three levers

An administrator has three independent controls. They are not
interchangeable, and the order of precedence is not a matter of taste.

| Lever | Who owns it | What it reaches | Latency |
| --- | --- | --- | --- |
| **Managed configuration** | The enterprise (Android Enterprise / Managed Google Play) | A device or a device group, on the device, without a server | Minutes |
| **Remote configuration** | The backend | Every installation, signed and validated | Minutes to hours |
| **The Play Console** | The publisher | The distribution: the listing, the data safety form, the target API level, the rollout | Days |

The precedence is fixed:

| Order | Source | Why |
| --- | --- | --- |
| 1 | The managed configuration | An enterprise's policy must not be overridable by a server the enterprise does not control |
| 2 | The remote configuration | The publisher's policy for everyone else |
| 3 | The bundled defaults | The app must work before the first fetch, and with no network at all |

A configuration value is **never** a permission. A managed configuration can
point the app at a different backend, but the **server authorizes every
request**; the URL never authorizes.

## 2 · Managed configuration

### The delivery channel

| Channel | Who can use it | How the value arrives |
| --- | --- | --- |
| **Managed Google Play, private app** | An enterprise that owns the devices | The Play Console declares the configuration; the platform delivers it to the device |
| **Android Management API, device owner** | An enterprise with a device-management console | The console sets the application restrictions; the platform applies them |

Both channels end the same way: the app's managed configuration is a bundle
of key/value pairs applied by the platform, readable by the app itself.

### The reader

The app reads its own managed configuration through the platform:

| API | What it gives |
| --- | --- |
| `RestrictionsManager.getApplicationRestrictions(context)` | The bundle the device owner applied to this application. The standard read, and the one the template's reader should use |
| `android.app.admin.ManagedConfiguration` (API 30+) | The typed, per-app managed configuration, for apps that need more than a flat bundle |
| `DevicePolicyManager` | Only for apps that are themselves device owners; not the template's case |

The read is **cheap and local** — it does not touch the network — so it
belongs on the startup path, on the main thread, before the first screen
renders.

### The keys

The managed configuration is a **closed set**. An unknown key fails the
whole parse; the app runs with the bundled defaults, not with a
half-understood policy.

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `TenantUrl` | string | empty | The base URL of the backend the app talks to |
| `TelemetryLevel` | `off` \| `errors` \| `full` | `errors` | How much telemetry may leave the device |
| `KillSwitch` | boolean | `false` | The app refuses to start |
| `MinVersion` | integer | 0 | The lowest `versionCode` the app accepts; below it, the update screen |
| `DeviceTierOverride` | `low` \| `mid` \| `high` | unset | Forces the quality tier, overriding the detection |
| `FeatureFlags` | object of booleans | empty | Per-capability switches |
| `DiagnosticsEnabled` | boolean | `true` | Whether the diagnostics bundle may be produced |
| `AllowedHosts` | array of strings | empty | The only hosts the app may talk to; empty means the normal set |
| `DataRetentionDays` | integer | `30` | How long local data is kept |
| `AllowUserSettings` | boolean | `true` | Whether the user may change the settings the policy controls |

Three parsing rules, enforced by the reader:

- **An unknown key fails the parse.** The app keeps the defaults; it never
  runs with a half-understood policy.
- **An unknown enum value fails the parse.**
- **A value of the wrong type fails the parse.**

The last two are deliberate: a typo in a managed configuration must not
silently change the security posture.

### What the administrator changes

| Key | What the administrator sees |
| --- | --- |
| `TenantUrl` | The app talks to a different backend |
| `TelemetryLevel` | The amount of telemetry the app may send |
| `KillSwitch` | The app refuses to start |
| `MinVersion` | Older installs are pushed to the update screen |
| `DeviceTierOverride` | The quality profile is forced, whatever the device says |
| `FeatureFlags` | Individual capabilities appear or disappear |
| `DiagnosticsEnabled` | The diagnostics bundle is produced or not |
| `AllowedHosts` | The app can reach fewer hosts |
| `DataRetentionDays` | Local data older than the limit is deleted |
| `AllowUserSettings` | The user may or may not change the settings |

### The gap, and what it takes to close it

| Missing | Where it belongs |
| --- | --- |
| The configuration value type | `:core:kernel` — a pure JVM data class, so it is testable without a device |
| The reader | `:core:platform` (a new Android library module) — the only place that touches `RestrictionsManager` |
| The merge by precedence | `:app` — the composition root resolves managed → remote → bundled |
| The diagnostics view | The diagnostics screen in `:app` ([11](11-telemetry.md)) |

The reader returns a **document**; the composition root decides what to do
with it. The platform layer never decides policy.

## 3 · Remote configuration

The remote configuration is the publisher's lever over every installation
that is not under an enterprise's control.

### The contract

| Rule | Why |
| --- | --- |
| **Data only** | No code, no code path, no URL to fetch. The document can only change values |
| **Signed** | The signature is verified against a key in the app; an unsigned document is ignored |
| **Versioned** | The document carries a version; an older one is discarded |
| **Schema-validated** | An unknown key fails the parse, exactly like the managed configuration |
| **Cached with a TTL** | A fetch failure must not change the app's behaviour |
| **Bundled fallback** | The app works before the first fetch and with no network |
| **Never a secret** | A value in a configuration document is readable by whoever reads the document |

### The remote-configuration keys

| Key | Meaning |
| --- | --- |
| `TenantUrl` | The base URL of the backend |
| `MinVersion` | The lowest accepted `versionCode` |
| `FeatureFlags` | Per-capability switches |
| `OnboardingContent` | The skippable screens and their copy |
| `QualityDefaults` | The default quality profile, per tier |
| `PushCategories` | The notification categories the app may use |
| `KillSwitches` | Per-capability kill switches (the 3D scene, the camera, the background location) |
| `TelemetryLevel` | The amount of telemetry the app may send |

### The remote-configuration gap, and what it takes to close it

| Missing | Where it belongs |
| --- | --- |
| The document schema | `:core:kernel` or `:feature:config:contracts` — a pure JVM schema |
| The signature verification | `:core:platform` — the Keystore-backed key ([09](09-security-and-keystore.md)) |
| The fetch | A `WorkManager` job with a network constraint ([08](08-offline-outbox-and-sync.md)) |
| The cache | The local store, with the TTL |
| The kill switch check | The startup path, before the first screen |

A remote configuration is **not** fetched on the main thread and **not**
fetched on every launch: it is a deferred job, and the cached copy is read
at startup.

## 4 · The Play Console setup

The tasks below are the publisher's checklist. The template implements none
of them; they are the console-side work a project must do once.

| Task | Where in the console | Why it matters |
| --- | --- | --- |
| Create the app and set the application id | Play Console → Create app | The id must match the `applicationId` in `:app` exactly |
| Set the namespace and the store listing | App content → Main store listing | The name, the description, the icon, the screenshots |
| The privacy policy URL | App content → Privacy policy | Required by Play; it is a real document, not a placeholder |
| The data safety form | App content → Data safety | It must match what the app actually does; a mismatch is a rejection and, worse, a false claim |
| The content rating | App content → Content ratings | Required before any release |
| App access | App content → App access | The instructions a reviewer needs, including a test account |
| The target API level | Release → Setup → App requirements | The store's floor; the app must meet it in the upload |
| The 16 KB page-size requirement | Release → Setup | Every native library must be aligned for it |
| Play App Signing | App integrity → App signing | The store holds the app signing key ([§5](#5--signing)) |
| The app integrity API | App integrity → App integrity API | The Play Integrity verdict, evaluated server-side ([09](09-security-and-keystore.md)) |
| App Links | App links → App links | The `assetlinks.json` is served from the domain, with the app's fingerprints ([06](06-navigation-routes-and-deep-links.md)) |
| The push credentials | Project settings → Cloud Messaging | The server needs the service account; the app needs the `google-services.json` |
| The tracks | Release → Testing → Internal testing | The promotion path: internal → closed → production |
| The staged rollout | Production → Staged rollout | The rollout percentage and the halt button |
| The pre-launch report | Release → Pre-launch report | The last automated gate before production |

### The permission declaration

The set of permissions the app requests is declared in the manifest and
must match the data safety form. The review:

| Check | How |
| --- | --- |
| The merged manifest of the release variant | `./gradlew :app:assembleRelease` and read the merged manifest |
| Every permission against the register | The register in the security chapter ([09](09-security-and-keystore.md)) |
| Every permission against the data safety form | A permission that collects data must be declared there |
| Every permission against the user-facing prompts | A permission that is never requested must not be declared |

A permission that is not in the register does not ship.

## 5 · Signing

### The two keys

| Key | Who holds it | Where it lives | Can it be rotated |
| --- | --- | --- | --- |
| **The app signing key** | Google Play | The store, never on a machine | Yes, through Play Console → App integrity → App signing → Request key rotation |
| **The upload key** | The publisher's CI | A KMS/HSM, in an isolated CI job | Yes, and it must be rotated on the store's schedule |

The app signing key never exists outside the store. The upload key is the
only key a build ever has, and it lives in the CI secret store — **never on
a developer machine**, never in the repository, never in a build log.

### The CI secret names

Each secret is prefixed with the app's uppercase name, so one CI project can
carry several apps:

| Secret | Content |
| --- | --- |
| `<APP>_UPLOAD_KEYSTORE_B64` | The upload keystore, base64-encoded |
| `<APP>_UPLOAD_KEYSTORE_PASSWORD` | Its password |
| `<APP>_UPLOAD_KEY_ALIAS` | The key alias inside the keystore |
| `<APP>_UPLOAD_KEY_PASSWORD` | The key password |
| `<APP>_PLAY_SERVICE_ACCOUNT_JSON` | The service account with the **release** permission only |

The debug keystore is the one exception: it is generated per machine by the
Android SDK and is not a secret worth protecting — but it is also not used
for a release.

### The rules

| Rule | Why |
| --- | --- |
| The release job is **isolated** and requires an approval | A release is not a side effect of a merge |
| The signing configuration reads the keystore from **properties or the environment**, never from a file in the tree | A keystore in the repository is a compromised key |
| The keystore file is **deleted** after the build | A CI artifact must not carry a signing key |
| The password is never printed, never echoed, never in the log | The log is a copy that outlives the job |
| The **app signing** key is never uploaded anywhere | Play holds it; a copy is a copy nobody can revoke |
| The signing config is declared in the root build file, not in a module | One place to review it |
| A key rotation is a planned release, not an incident response | The store's rotation flow has its own steps |

### The release signing configuration

The shape a project adopts in the root build file — **not** present in the
template:

```kotlin
// The shape, not the shipped code. The values come from the environment.
android {
    signingConfigs {
        create("release") {
            val keystorePath = providers.environmentVariable("<APP>_UPLOAD_KEYSTORE_PATH")
            storeFile = keystorePath.orNull?.let { file(it) }
            storePassword = providers.environmentVariable("<APP>_UPLOAD_KEYSTORE_PASSWORD").orNull
            keyAlias = providers.environmentVariable("<APP>_UPLOAD_KEY_ALIAS").orNull
            keyPassword = providers.environmentVariable("<APP>_UPLOAD_KEY_PASSWORD").orNull
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = true      // R8 full mode
            isShrinkResources = true
            isDebuggable = false
            signingConfig = signingConfigs.getByName("release")
        }
    }
}
```

The four `environmentVariable` reads are the whole integration. The values
come from the CI secret store, the local developer gets a **debug** build
with the local debug keystore, and no keystore file is ever committed.

### The verification metadata

The dependency verification file (`gradle/verification-metadata.xml`) is
committed: it pins the exact bytes of every artifact the build resolves. A
dependency that changes its bytes without changing its version is a
supply-chain incident, and the verification file is what catches it.

## 6 · Verifying a deployment

An administrator needs to answer four questions: is the policy applied,
what does the app think it is, what did the app actually do, and is the
installed build the one that was approved.

| Question | The check |
| --- | --- |
| Is the managed configuration applied? | The platform read succeeds and returns the expected keys; the diagnostics bundle shows the **effective** configuration, redacted |
| What does the app believe? | The effective configuration appears in the diagnostics bundle, with the source of each value named (managed, remote, bundled) |
| Did the app act on it? | The log line of the configuration load names the source and the document version |
| Is the installed build the approved one? | The `versionCode`, the `versionName` and the **signing certificate fingerprint** of the installed package match the release record |
| Is the device genuine? | The server's Play Integrity verdict ([09](09-security-and-keystore.md)) |
| Is the release healthy? | The Play Console vitals for the release, against the thresholds that gate the promotion ([12](12-testing-and-ci.md)) |

The last three rows are why the diagnostics bundle exists
([11](11-telemetry.md)): an administrator who cannot see what the app
believes cannot debug a deployment.

### The install check

| Step | Command | What it proves |
| --- | --- | --- |
| List the installed packages | `adb shell pm list packages \| grep <app>` | The app is installed under the expected id |
| Read the installed version | `adb shell dumpsys package <app> \| grep versionName` | The installed build is the one that was approved |
| Read the certificate | `apksigner verify --print-certs <path-to-apk>` | The APK is signed with the expected certificate |
| Compare the fingerprints | the release record vs. the printed digest | The build is the approved one |

## 7 · The rollout and the kill switch

| Stage | Where | The gate |
| --- | --- | --- |
| Internal testing | Play Console → Internal testing | The pre-launch report, the smoke test on the Low pool |
| Closed testing | Play Console → Closed testing | The vitals thresholds, the E2E suite on the pool |
| Production | Play Console → Production | The staged rollout starts at a small percentage |
| The staged rollout | Production → Staged rollout | The percentage increases only while the vitals stay under the thresholds |
| The halt | The rollout screen | The rollout stops; the installed builds keep working |
| The kill switch | The managed configuration, or the remote configuration | The app refuses to start or disables the capability |

A staged rollout is the last reversible step; after it, the fix is a new
release. That is why the rollout percentage starts small and why the halt
button must be reachable by someone who is not on call for the app.

## 8 · The admin surface the template does not have

| Missing | Chapter that specifies it |
| --- | --- |
| The managed configuration reader | [§2](#2--managed-configuration) above, [07](07-auth-and-platform-features.md) |
| The remote configuration client | [§3](#3--remote-configuration) above |
| The diagnostics screen and the bundle | [11](11-telemetry.md) |
| The signing configuration | [§5](#5--signing) above, [09](09-security-and-keystore.md) |
| The Play integrity integration | [09](09-security-and-keystore.md) |
| The release pipeline | [13 — Playbooks](13-playbooks.md) §6 |
| The permission register | [09](09-security-and-keystore.md) |
| The version / build metadata | [11](11-telemetry.md) |

## 9 · Security rules

| Rule | Why |
| --- | --- |
| A configuration value never carries a secret | A configuration document is readable by whoever reads the document |
| A malformed configuration never becomes a partial configuration | The defaults are used as a whole |
| The `KillSwitch` needs no network | It must work on a device that cannot reach the network |
| The managed configuration wins over the remote one | An enterprise's policy is not overridable by a server |
| A policy value is never logged verbatim | Some values are sensitive; the effective configuration is redacted in the bundle |
| The upload key never leaves the CI secret store | A key that exists in two places exists in the wrong one |
| The configuration never authorizes | The server authorizes every request |

## See also

- [Security & Keystore](09-security-and-keystore.md) — the Keystore, the
  integrity verdict, the Play policy checks.
- [Telemetry](11-telemetry.md) — the diagnostics bundle that proves what the
  app believes.
- [Playbooks](13-playbooks.md) — the release recipe.
- [Support Matrix](support-matrix.md) — the devices the deployment is
  verified on.
