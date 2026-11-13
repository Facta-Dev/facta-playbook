# Security & Keychain

The threat model of a mobile client: the device is **not trusted**, the
server is authoritative, and everything the app holds can be extracted from
a compromised device. The controls below are the baseline; each one has an
Apple mechanism behind it.

> **Known gap — do not copy a fiction.** The template **does not ship** a
> Keychain wrapper, an encrypted store, an App Transport Security
> configuration, a privacy manifest or an integrity check. The `notes` slice
> stores nothing sensitive and talks to no server, so there is nothing to
> protect yet. The template declares **no entitlement** and **no
> `Info.plist` usage key** — `project.yml` sets `GENERATE_INFOPLIST_FILE:
> YES` and three generated keys, none of which asks the user for anything.
> This chapter specifies the contract and the adoption path.

## 1. The baseline

| Area | Control | iOS mechanism |
| --- | --- | --- |
| Storage of secrets | Secrets are stored in the Keychain, where the app can use them but cannot read the key material | **Keychain** + **CryptoKit** (AES-GCM), optionally a **Secure Enclave** key |
| Backups and transfer | The token stores are device-only and non-synchronizable | `kSecAttrAccessible…ThisDeviceOnly` + `kSecAttrSynchronizable: false` |
| Network | TLS only, no blanket exception, no arbitrary loads | **App Transport Security** — on by default, tightened by the absence of any exception |
| Components | Verified Universal Links, no custom URL scheme, no unnecessary extension | The `Info.plist` and the entitlements file |
| UI | A privacy overlay in the app switcher, `UIScreen.isCaptured` handling on sensitive screens | `scenePhase` and `UIScreen.isCaptured` |
| Binary | Release builds stripped, no API key in the bundle | The release build settings in `project.yml` |
| Platform trust | **App Attest** (with DeviceCheck) as a **server-evaluated** risk signal | The App Attest API |
| Privacy | A privacy manifest that declares what the app actually does | `PrivacyInfo.xcprivacy` |
| Logs | Redaction on by default; nothing sensitive is logged | The logger ([11](11-telemetry.md)) — specified, not shipped |

## 2. The Keychain

The **Keychain** is where a secret belongs. An item added with a
`kSecAttrAccessible` class is protected by the device's passcode, by the
data-protection class behind it and — for the `ThisDeviceOnly` variants — by
the device itself: the item never leaves the device, not through an iCloud
Keychain sync and not through a backup.

### The accessibility classes

The choice is between *when* the item is readable and *where* it may live:

| Class | Readable | Migrates to a new device | Use it for |
| --- | --- | --- | --- |
| `kSecAttrAccessibleWhenUnlocked` | Only while the device is unlocked | Yes | A secret the app only ever needs in the foreground |
| `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` | Only while unlocked | **No** | A refresh token, in a stricter app |
| `kSecAttrAccessibleAfterFirstUnlock` | From the first unlock after boot until the next restart | Yes | A background task that must read the secret while the screen is locked |
| `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` | From the first unlock after boot | **No** | The **default** for a token: the background sync still works, and the secret never leaves the device |
| `kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly` | Only while unlocked, and only if a passcode is set | **No** | The strictest posture: the item is inaccessible on a device without a passcode |
| `kSecAttrAccessibleBiometryCurrentSet` | Only with a fresh biometric match | **No** | A key gated behind Face ID / Touch ID (`SecAccessControl`) |

The trade-off in one line: **`AfterFirstUnlock` costs you the lock-screen
protection but buys you the background job; `WhenUnlocked` protects the
screen but breaks the background job.** A refresh token that the outbox drain
needs is an `AfterFirstUnlock…ThisDeviceOnly` item
([08](08-offline-outbox-and-sync.md)); a key that unlocks an encrypted
database is `WhenUnlocked…ThisDeviceOnly`.

The rule that decides it: **`ThisDeviceOnly` unless there is a reason not
to.** A synchronizable item is a copy of the secret in Apple's servers and
in every device's Keychain.

### The port

```swift
// The shape of the secure store. The plaintext never reaches the disk, and
// the key never leaves the Keychain.
public protocol SecureStorePort: Sendable {
    func get(_ key: String) async throws -> Data?
    func set(_ value: Data, for key: String) async throws
    func remove(_ key: String) async throws
}
```

| Rule | Why |
| --- | --- |
| **A small, fixed set of items** — one master item per purpose | One item per token does not scale, and each item is another thing to migrate |
| The port is in the domain, the implementation in `Packages/Core/Security` | The domain never sees a Keychain query, so it stays testable on a Mac with no device |
| `kSecAttrSynchronizable` is `false` | A synchronizable token is a token on every device |
| An access group only when an extension needs it | A shared group is a shared blast radius |
| A biometric gate uses `SecAccessControl`, not a password check | The gate belongs to the key, not to the code around it |

Two things that are **not** acceptable:

| Not acceptable | Why |
| --- | --- |
| A secret in `UserDefaults` or `@AppStorage` | The plist is readable from a backup, from a jailbroken device and from a device backup tool; it is not protected by the passcode |
| A secret in the source tree, in `project.yml` or in the `Info.plist` | The bundle is public and the repository is not a secret store |

## 3. No secrets in the bundle

The app bundle is **downloadable**. Anything inside it — an `Info.plist`
value, a build setting, a resource file, a string constant — is public to
whoever downloads the app.

| Never in the bundle or the tree | Where it belongs instead |
| --- | --- |
| An API key | The server, or a **restricted** key the provider binds to the bundle id and the signing identity |
| A client secret | The server. It is not a client concept |
| A refresh token | The Keychain ([07](07-auth-and-platform-features.md)) |
| The APNs `.p8` key | The CI secret store; it signs the **server's** requests to APNs, it never ships in the app |
| The signing certificate and the provisioning profile | The CI secret store and the developer portal ([Admin Guide](admin-guide.md) §8) |
| A password | Nowhere |

What belongs in `project.yml` is only the **public** configuration: the API
base URL, a build-time feature flag, a public client id.

| Rule | Why |
| --- | --- |
| The base URL comes from a build setting or a runtime configuration value | It is public |
| A secret comes from the **server** at runtime, or from a Keychain item | The server decides |
| The CI secret store holds the signing material | The key never touches a developer machine |
| `scripts/leak-guard.sh` scans the tree for secret-shaped strings | A guard rail, not a full SAST — but it catches the accident |
| `evals/contracts.test.mjs` asserts `project.yml` declares no credential | The manifest is a build fact, not a review comment |

The contract eval is specific: the manifest is scanned for a key whose
**name** looks like a credential (`api_key`, `secret`, `password`, `token`,
`private_key`, `credential`, a bare `key`) **and** for a **value** whose
shape looks like one (an AWS access key id, a private key header, a GitHub
or Slack token, a JWT, a `.p12` or a `.mobileprovision` suffix). Both sides
fail the build. A manifest that passes is not proof of safety, but a manifest
that fails is a defect.

`.env.example` exists and is a template, not a store: `.env` is gitignored,
and the file itself says so. An iOS app cannot read a `.env` at runtime
anyway — the values become an `xcconfig` entry or a runtime configuration
value.

## 4. App Transport Security

**ATS is on by default.** The template adds no exception and therefore has
no transport posture of its own to declare — which is the strongest posture
there is.

| Rule | Why |
| --- | --- |
| No `NSAllowsArbitraryLoads` | A blanket grant turns TLS off for every host the app reaches |
| An exception, if ever needed, is scoped to a **named domain** with a written reason | An exception is a hole with a shape; a blanket grant is a hole |
| TLS 1.2 minimum | The floor, per `PLAYBOOK_SECURITY.md` §8 |
| Certificate pinning, only with a backup pin and a rotation plan | A pinning mistake bricks the app on the pinned devices |

`evals/contracts.test.mjs` fails the build if `project.yml` contains
`NSAllowsArbitraryLoads: true`. The check is deliberately narrow — it
catches the blanket grant, the mistake that matters, and leaves the
scoped-exception decision to a human with the reason in front of them.

Pinning, done carefully: pin the **intermediate** or the leaf, keep a
**backup pin**, and have a remote kill switch for the pin set. A pin set
that cannot be rotated remotely is an outage waiting for a certificate
rotation.

## 5. Capabilities and the `Info.plist`

The template declares **no entitlement and no `Info.plist` usage key**. The
only `Info.plist` keys that exist are the three generated ones in
`project.yml` — `INFOPLIST_KEY_CFBundleDisplayName`,
`INFOPLIST_KEY_UILaunchScreen_Generation` and
`INFOPLIST_KEY_UIApplicationSceneManifest_Generation` — and none of them
asks the user for anything.

| Rule | Why |
| --- | --- |
| A capability is declared **when the feature ships**, not before | A declared-but-unused capability is a rejection risk; an undeclared one is a crash |
| Every usage description is specific about the data | A vague string is a rejection, and the user's only clue |
| Every entitlement is justified in a review note | The note is what an App Reviewer reads |
| The capabilities are declared in `project.yml` and in an entitlements file, never in Xcode's UI | The project file is generated and git-ignored ([ADR 0001](adr/0001-xcodegen-generated-project.md)) |
| The release bundle is the artefact that is reviewed | `codesign -d --entitlements :-` on the built product is the evidence |

### The capability register

Every capability the app declares is listed here, with the feature that
needs it and the prompt that explains it. **A capability that is not in this
table does not ship.** The table is the check list for the release: the
merged `Info.plist` and the entitlements of the built app are compared
against it, and a capability that appears in the binary but not in this
table is a finding ([Admin Guide](admin-guide.md) §7).

| Capability | Needed for | Prompt | Status |
| --- | --- | --- | --- |
| Push notifications (`aps-environment`) | The notification pipeline ([10](10-push-notifications.md)) | The notification permission prompt | **Not declared** — no entitlements file, no `aps-environment` |
| Notification permission | The same | `UNUserNotificationCenter.requestAuthorization` | **Not declared** — no notification code |
| Background modes (`processing`, `remote-notification`) | The outbox drain and the silent pushes ([08](08-offline-outbox-and-sync.md)) | None — the system decides | **Not declared** — no `UIBackgroundModes` key |
| Associated domains | Universal Links ([06](06-navigation-routes-and-deep-links.md)) | None | **Not declared** |
| Keychain access group | A shared secret with an extension | None | **Not declared** |
| Sign in with Apple | The third-party sign-in ([07](07-auth-and-platform-features.md)) | The system sheet | **Not declared** |
| `NSLocationWhenInUseUsageDescription` | A location feature | At the point of use | **Not declared** — no location feature |
| `NSCameraUsageDescription` | A capture feature | At the point of use | **Not declared** — no capture feature |
| `NSMicrophoneUsageDescription` | A voice feature | At the point of use, with an in-app cue | **Not declared** — no voice feature |
| `NSFaceIDUsageDescription` | A biometric lock | At the point of use | **Not declared** |
| `NSUserTrackingUsageDescription` | Cross-app tracking | At the point of use | **Not declared** — the app does not track |

## 6. Integrity

**App Attest** tells the server whether the app and the device look
genuine. **DeviceCheck** answers the same question with a coarser signal —
whether the device is honest, not whether this app instance is honest.

| Rule | Why |
| --- | --- |
| The verdict is evaluated **server-side** | A client-side check can be patched out |
| It is a **risk signal**, not a block | Blocking on a false positive costs a legitimate user their access |
| Jailbreak signals are **advisory** | A jailbroken device is not automatically an attacker |
| The server exchanges the client assertion and decides | The client never makes the access decision |
| The App Attest key is stored in the Keychain | It is a secret, and it is per-installation |

The client sends the assertion; the server exchanges it and decides. The
client never makes the access decision.

## 7. App Store policy and the privacy manifest

The checks that bite at submission time:

| Topic | Requirement |
| --- | --- |
| Account deletion | **Both** an in-app path and a web path ([07](07-auth-and-platform-features.md)) |
| Permissions | Specific usage strings; background modes limited to what is used; no unnecessary sensitive permission |
| Privacy | The privacy manifest and the App Privacy answers must match the actual behaviour |
| Tracking | The ATT prompt only if the app actually tracks across apps and sites |
| Sign in with Apple | Offered when other third-party social logins exist (guideline 4.8) |
| Export compliance | `ITSAppUsesNonExemptEncryption` answered honestly |
| UGC and AI | Reporting, blocking, moderation, disclosure |
| Content rating | Must reflect chat, voice, purchases and location |
| Age rating | The questionnaire reflects the variant's audience |

The **privacy manifest** (`PrivacyInfo.xcprivacy`) is code that ships with
the binary, and it is reviewed like code:

| Item | Who provides it | Note |
| --- | --- | --- |
| The app's own `PrivacyInfo.xcprivacy` | The consumer | Declares the tracking domains and the **required-reason API** categories the app itself uses |
| The privacy manifest of every third-party SDK | The SDK's vendor | Every binary SDK must ship one; the template ships **zero third-party packages**, so today there is nothing to collect |
| The App Privacy answers | The consumer, in App Store Connect | They must agree with the manifest and with the app's actual behaviour |

Because the template declares no entitlement, no usage description and no
third-party package, its privacy surface is empty — which is exactly the
state a consumer must re-derive the moment a capability is added.

## 8. The template's posture today

| Control | State in the template |
| --- | --- |
| An entitlements file | **Not present** — `project.yml` sets no `CODE_SIGN_ENTITLEMENTS` |
| The `Info.plist` usage keys | **Not present** — the three generated keys ask the user for nothing |
| The ATS configuration | **Not present, and not needed** — no exception, no arbitrary loads |
| The Keychain wrapper | **Not present** — no secret is stored |
| `PrivacyInfo.xcprivacy` | **Not present** |
| The release build configuration | **To be configured** when a release build is added |
| App Attest | **Not present** |
| The redacting logger | **Not present** — the template writes no log at all ([11](11-telemetry.md)) |
| The `notes` data | An id and a title, in memory, never persisted |

The template's security posture is honest: **it holds nothing sensitive and
reaches nothing**. Every control in this chapter becomes a real requirement
the moment the app gains a session, a cache or a network call — which is
exactly when §9 has to be executed, not before.

## 9. The adoption path

1. **Add `Packages/Core/Security`.** The `SecureStorePort` implementation
   over the Keychain, with `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`
   and `kSecAttrSynchronizable: false`.
2. **Add the secure store** when the app gains a session
   ([07](07-auth-and-platform-features.md)): the refresh token is the first
   secret, and it goes in the Keychain, never in `UserDefaults`.
3. **Add the entitlements file** and wire it in `project.yml`. Push,
   Associated Domains and Sign in with Apple arrive with their features, one
   at a time, each with a review note.
4. **Add the usage strings** as `INFOPLIST_KEY_*` settings, or move to a
   committed `Info.plist` when the key count makes the generated form
   unreadable.
5. **Add `PrivacyInfo.xcprivacy`** to the app target, with the required-reason
   API declarations that match the code.
6. **Add the ATS exceptions**, if any, scoped to a named domain with a
   written reason — never as a blanket grant.
7. **Add App Attest** as a server-evaluated signal when the fraud model
   justifies it.
8. **Write the App Privacy answers** from the dependency register, so the
   questionnaire matches what the app actually does.

## Do / don't

| Do | Don't |
| --- | --- |
| Store the refresh token in the Keychain, device-only | Store a token in `UserDefaults` |
| Use one Keychain item per purpose | Create an item per token |
| Pick `AfterFirstUnlock…ThisDeviceOnly` for a token a background job needs | Pick `WhenUnlocked` and break the background job |
| Keep the base URL and a restricted client id in `project.yml` | Put an API key in `project.yml` |
| Leave ATS on | Ship an `NSAllowsArbitraryLoads` "for debugging" |
| Declare a capability when the feature ships | Declare a capability "for later" |
| Evaluate the App Attest assertion on the server | Block the app on a client-side verdict |
| Keep the privacy manifest in step with the code | Write it once and forget it |

## See also

- [Auth & Platform Features](07-auth-and-platform-features.md) — where the
  tokens live.
- [Telemetry](11-telemetry.md) — the redaction that keeps secrets out of
  the logs.
- [Push Notifications](10-push-notifications.md) — the `aps-environment`
  entitlement and the permission prompt.
- [Offline, Outbox & Sync](08-offline-outbox-and-sync.md) — the background
  job that decides the Keychain accessibility class.
- [Admin Guide](admin-guide.md) — the signing material and the App Store
  Connect setup.
- [Playbook Native iOS Swift §15](../PLAYBOOK_MOBILE_IOS_SWIFT.md) — the
  security and privacy baseline.
