# Admin Guide

How an operator or an enterprise administrator configures a deployed mobile
app: the push credentials, the store configuration, the managed config keys
and how to verify that all of it arrived.

## 1 · What is configurable, and by whom

| Surface             | Set by                      | Where it lands                                          | Read by                             |
| ------------------- | --------------------------- | ------------------------------------------------------- | ----------------------------------- |
| The API base URL    | The build                   | `extra.apiBaseUrl` (adoption step)                      | The HTTP adapter                    |
| The variant         | The build                   | `extra.variant` (adoption step)                         | The app and the server registration |
| Push credentials    | The operator                | The secret store and the EAS project                    | The backend that sends              |
| The managed config  | The administrator           | The MDM (Android Enterprise / Apple Managed App Config) | The config reader                   |
| The remote config   | The operator                | The remote config service                               | The app at runtime                  |
| The quality profile | The user, then the operator | The device, then the remote config                      | The governor                        |
| The kill switch     | The operator                | The remote config service                               | The app and the backend             |

Three of these — the managed config, the remote config and the kill switch —
are **not shipped by the template**. They are adoption steps, described here
so that adopting them is a decision with a shape, not an improvisation.

The template ships a static `app/app.json`: `expo.name` (`<APP>`), `expo.slug`
and `expo.scheme` (`<app>`), `expo.ios.bundleIdentifier` and
`expo.android.package` (`com.<scope>.<app>`). It ships **no** `extra` block,
no `app/app.config.ts` and no EAS project.

> **Known gap.** The API base URL and the variant have no home in the
> template. The adopted shape is a dynamic `app/app.config.ts` that reads the
> environment and adds an `extra` block, with the static `app/app.json` kept
> as the base it extends
> ([06 — Variants & Monorepo](06-variants-and-monorepo.md)).

## 2 · The push credentials

### FCM v1 (Android)

The backend needs a service account key with the **Firebase Cloud Messaging
API** enabled. The key is a JSON file with a `private_key` and a
`client_email`.

| Item           | Where it goes                                              |
| -------------- | ---------------------------------------------------------- |
| The key file   | The backend's secret store; **never** in the app bundle    |
| The project id | The backend's configuration; it identifies the FCM project |
| The sender id  | The FCM sender id, used to verify a registration           |

> **The app never holds the FCM key.** The app registers a _device token_;
> the backend holds the credential and sends. A key in the bundle is a key
> in the attacker's hands.

### APNs (iOS)

The backend needs an APNs auth key (`.p8`), its key id and the Apple team
id. The app needs the capability enabled on its app record, and the token
it receives is an APNs device token.

| Item           | Where it goes                                           |
| -------------- | ------------------------------------------------------- |
| The `.p8` key  | The backend's secret store; **never** in the app bundle |
| The key id     | The backend's configuration                             |
| The team id    | The backend's configuration                             |
| The capability | Enabled on the app record in the Apple Developer portal |

### The registration

The app registers the device **after the login**, with the metadata the
server needs to route a message:

```json
{
  "platform": "android",
  "provider": "fcm",
  "token": "<the native device token>",
  "appVariant": "market",
  "appVersion": "1.4.0",
  "locale": "es-AR",
  "tz": "America/Argentina/Buenos_Aires"
}
```

The server stores the provider **with** the token, because an Expo push
token and a native token are different types, and it needs the variant to
route a message to the right app.

### The EAS credentials

The build credentials are **per variant**: a market build and a game build
are different apps with different bundle ids, so they have different
credentials.

| Item                                                  | Where it is stored           |
| ----------------------------------------------------- | ---------------------------- |
| The upload certificate / the distribution certificate | The EAS project, per variant |
| The push key                                          | The EAS project, per variant |
| The signing identity                                  | The EAS project, per variant |

> **Known gap.** The template ships no `eas.json` and no credential
> workflow. Credentials are an operator concern
> ([06 — Variants & Monorepo](06-variants-and-monorepo.md)).

## 3 · The store configuration

### iOS

| Item                                 | Where                        | Why                                         |
| ------------------------------------ | ---------------------------- | ------------------------------------------- |
| The bundle id                        | `expo.ios.bundleIdentifier`  | Must match the app record and the push key  |
| The associated domain                | `expo.ios.associatedDomains` | `applinks:<domain>` for the universal links |
| The push entitlement                 | The app record               | Remote push                                 |
| The privacy manifest                 | The app bundle               | The store's privacy nutrition label         |
| The `NSUserTrackingUsageDescription` | `expo.ios.infoPlist`         | Only if the app tracks — otherwise omit it  |

### Android

| Item                     | Where                        | Why                                        |
| ------------------------ | ---------------------------- | ------------------------------------------ |
| The package name         | `expo.android.package`       | Must match the app record and the push key |
| The push permission      | `POST_NOTIFICATIONS`         | Android 13+; without it, nothing is shown  |
| The `intent-filter`      | `expo.android.intentFilters` | The App Links, with `autoVerify`           |
| The 16 KB page alignment | The native libraries         | A Play requirement for recent Android      |
| The data safety form     | The Play Console             | Must match the actual SDK behaviour        |

### The verification files

The two files that make a link open the app instead of the browser:

| Platform | File                         | Served at                                                 |
| -------- | ---------------------------- | --------------------------------------------------------- |
| Android  | `assetlinks.json`            | `https://<domain>/.well-known/assetlinks.json`            |
| iOS      | `apple-app-site-association` | `https://<domain>/.well-known/apple-app-site-association` |

Each lists the app id and the signing fingerprint. A mismatch means the link
opens the browser — every time, silently.

> **Known gap.** The template ships neither file. The domain and the signing
> fingerprints are a deployment step.

## 4 · The managed config keys

The managed config is the administrator's channel: it is published by the
MDM (Android Enterprise, or Apple Managed App Config) and it is **read-only**
from the app's point of view.

The keys below are the ones this kit's shape implies. The schema and the
reader are **not shipped**; they are an adoption step.

| Key                   | Type                            | Meaning                                                               | The default when absent |
| --------------------- | ------------------------------- | --------------------------------------------------------------------- | ----------------------- |
| `ApiBaseUrl`          | string (URL)                    | The API the app talks to                                              | The build-time URL      |
| `AllowedHosts`        | array of hosts                  | The hosts the app may talk to                                         | The API host only       |
| `BlockedHosts`        | array of hosts                  | The hosts the app must never talk to                                  | Empty                   |
| `FeatureFlags`        | object of booleans              | The features the administrator turns on or off                        | The code defaults       |
| `KillSwitch`          | boolean                         | Switches the app off without uninstalling it                          | `false`                 |
| `AllowUserSettings`   | boolean                         | Whether the user may change the settings                              | `true`                  |
| `PerformanceMode`     | `auto \| battery-saver \| high` | The default quality profile                                           | `auto`                  |
| `TierOverrides`       | map of model → tier             | Forces the device tier for a model                                    | None                    |
| `MinSupportedVersion` | string                          | The minimum version the app accepts; below it, a force-upgrade screen | None                    |
| `TelemetryEnabled`    | boolean                         | Whether the telemetry sink is installed                               | `false`                 |

Three rules, the same as everywhere else in this repository:

1. **The administrator wins.** A managed value beats the user's setting.
2. **An invalid value is ignored, and the default is kept.** The app never
   runs with a half-parsed config.
3. **An unknown key is dropped**, not an error. A newer administrator
   config must not break an older app.

### Where the config is read

| Platform | The source                                                |
| -------- | --------------------------------------------------------- |
| Android  | Android Enterprise managed configuration, through the MDM |
| iOS      | Apple Managed App Configuration, through the MDM          |

The reader is a port, like every other device capability: an adapter reads
the MDM value and hands a plain object to the app. The app never parses an
MDM payload itself.

## 5 · The remote config and the kill switch

The remote config is the operator's runtime channel: it changes without a
new build.

| Key                   | Type                | Meaning                           |
| --------------------- | ------------------- | --------------------------------- |
| `FeatureFlags`        | object of booleans  | The features that are on          |
| `KillSwitch`          | boolean             | The emergency off switch          |
| `MinSupportedVersion` | string              | The force-upgrade threshold       |
| `PerformanceMode`     | the quality profile | The default profile for the fleet |
| `TierOverrides`       | map of model → tier | The forced tier, by device model  |
| `PushCategories`      | object of booleans  | The categories that may be sent   |

The remote config is **data, not code**: it is fetched, validated against a
schema, and applied. It never loads a module, never changes a URL that was
not in the allowlist, and never turns a feature on that the binary does not
contain.

The kill switch is checked **before** the feature, not inside it: a switch
that is checked after the work started is a switch that is too late.

## 6 · Verification

### The push credentials

| Check                   | How                                                              |
| ----------------------- | ---------------------------------------------------------------- |
| The FCM key is valid    | The backend's send call returns a provider id, not an auth error |
| The APNs key is valid   | The backend's send call returns `200` from APNs, not `403`       |
| The token is registered | The registration row exists for the device and the variant       |
| The message arrives     | A test message reaches the device and opens the right screen     |

### The store configuration

| Check                            | How                                                                  |
| -------------------------------- | -------------------------------------------------------------------- |
| The bundle id matches            | The build's id is the one in the store console and in the app record |
| The signing fingerprint          | The fingerprint in `assetlinks.json` is the one of the shipped build |
| The universal link opens the app | Tapping the link on a device opens the app, not the browser          |
| The permission is declared       | The installed build's manifest declares `POST_NOTIFICATIONS`         |
| The privacy manifest matches     | The manifest's list matches what the app actually does               |

### The managed config

| Check                        | How                                                                    |
| ---------------------------- | ---------------------------------------------------------------------- |
| The MDM delivered the value  | The MDM console lists the app configuration                            |
| The app read it              | The app's diagnostics screen shows the effective config and its source |
| An invalid value was ignored | The effective config shows the default, and the log carries a warning  |
| The user cannot override it  | The setting is rendered read-only when `AllowUserSettings` is `false`  |

### The kill switch

| Check                            | How                                              |
| -------------------------------- | ------------------------------------------------ |
| The switch flips the feature off | The feature is unavailable, without a new build  |
| The switch is checked early      | The feature does no work before the check        |
| The switch survives a bad fetch  | A failed fetch keeps the last good configuration |

## 7 · Failure modes

| Symptom                              | Cause                                                                      | Fix                                                                                 |
| ------------------------------------ | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Push arrives on one variant only     | The credentials are shared instead of per variant                          | Give each variant its own key ([§2](#2--the-push-credentials))                      |
| Push never arrives, no error         | `POST_NOTIFICATIONS` was never granted on Android 13+                      | Ask for the permission in context ([10](10-push-notifications.md))                  |
| The push token is rejected           | The app registered before the login, or after a sign-out                   | Register after the login; unregister on sign-out                                    |
| The link opens the browser           | The verification file does not match the signing fingerprint               | Re-publish `assetlinks.json` and `apple-app-site-association`                       |
| The app ignores the managed config   | The key is unknown, the value is invalid, or the reader is not implemented | Implement the reader; check the warning in the log                                  |
| The user overrides the administrator | `AllowUserSettings` was not published, or is `true`                        | Publish `AllowUserSettings: false`                                                  |
| The kill switch does nothing         | It is checked after the work started                                       | Move the check before the feature ([§5](#5--the-remote-config-and-the-kill-switch)) |

## 8 · Removing a policy

1. Remove the value from the MDM console (or the remote config service).
2. Force the app to re-read the config, or let it expire at the next fetch.
3. Confirm on the diagnostics screen that the key is gone and the default
   applies.
4. With `AllowedHosts` empty, the app talks to no host: it is the safe state.

## See also

- [Push Notifications](10-push-notifications.md)
- [Security & Secure Store](09-security-and-secure-store.md)
- [Variants & Monorepo](06-variants-and-monorepo.md)
