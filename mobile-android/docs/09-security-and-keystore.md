# Security & Keystore

The threat model of a mobile client: the device is **not trusted**, the
server is authoritative, and everything the app holds can be extracted from
a compromised device. The controls below are the baseline; each one has an
Android mechanism behind it.

> **Known gap — do not copy a fiction.** The template **does not ship** a
> Keystore integration, an encrypted store, a network security config or an
> integrity check. The `notes` slice stores nothing sensitive and talks to
> no server, so there is nothing to protect yet. This chapter specifies the
> contract and the adoption path.

## 1. The baseline

| Area | Control | Android mechanism |
| --- | --- | --- |
| Storage of secrets | Secrets are encrypted with a key the app cannot export | Android Keystore + **Tink** (AEAD, AES-GCM) |
| Backups | The token stores are excluded from backup and device transfer | `android:allowBackup="false"` — the rules files are an adoption step (§8) |
| Network | No cleartext, no user-installed CAs in release, TLS 1.2+ | `network_security_config.xml` |
| Components | `android:exported` is explicit; `PendingIntent`s are immutable | The manifest |
| UI | `FLAG_SECURE` on payment and auth screens; `filterTouchesWhenObscured` against tapjacking | The window flags |
| Binary | R8 full mode, `debuggable` off, no API key in the APK | The release build type |
| Platform trust | Play Integrity as a **server-evaluated** risk signal | The Play Integrity API |
| Logs | Redaction on by default; nothing sensitive is logged | The logger ([11](11-telemetry.md)) — specified, not shipped |

## 2. The Keystore and Tink

The **Android Keystore** holds a key that the app can use but **cannot
read**. The key material lives in the hardware-backed keystore when the
device has one, and the app only ever sees a handle.

**Tink** provides the AEAD encryption on top: the app asks Tink to encrypt
a value, Tink uses the Keystore-backed key, and the ciphertext is what
reaches the disk.

```kotlin
// The shape of the secure store. The plaintext never reaches the disk, and
// the key never leaves the Keystore.
interface SecureStorePort {
    suspend fun get(key: String): ByteArray?
    suspend fun put(key: String, value: ByteArray)
    suspend fun remove(key: String)
}
```

| Rule | Why |
| --- | --- |
| **A small, fixed set of keys** — one master key per purpose | Android 17 caps the number of Keystore keys per app; a key per token does not scale |
| StrongBox when available | Hardware-backed, but not every device has it — fall back |
| The key set is created lazily and reused | A key created per operation exhausts the cap |
| The port is in the domain, the implementation in `:core:security` | The domain never sees a Keystore, so it stays testable on the JVM |

Two things that are **not** acceptable:

| Not acceptable | Why |
| --- | --- |
| A secret in `SharedPreferences` or DataStore in plaintext | The file is readable from a backup, from a rooted device, or from an `adb backup` |
| `androidx.security:security-crypto` | Deprecated; use Tink directly with the Keystore |

## 3. No secrets in `BuildConfig`

`BuildConfig` is **compiled into the APK**. Anything in it is in the binary,
and the binary is downloadable.

| Never in `BuildConfig` | Never in the source tree |
| --- | --- |
| An API key | A signing key |
| A client secret | A service-account JSON |
| A refresh token | A `.jks` or `.keystore` file |
| A password | A production URL that is not public |

What belongs there is only the **public** configuration: the API base URL, a
build-time feature flag, a public OAuth client id.

| Rule | Why |
| --- | --- |
| The base URL comes from a build-time property or the manifest | It is public |
| A secret comes from the **server** at runtime, or from a Keystore-backed local entry | The server decides |
| The CI secret store holds the signing key | The key never touches a developer machine |
| `scripts/leak-guard.sh` scans the tree for secret-shaped strings | A guard rail, not a full SAST — but it catches the accident |

A common pattern that is legitimate: a **restricted** API key (an Android
key restricted by package name and signing certificate, or a Google Maps key
restricted by app id and API). A restricted key in `BuildConfig` is a
mitigated risk, not a secret — and the restriction is what makes it safe.

## 4. The network security config

`res/xml/network_security_config.xml` is the declarative posture of the
app's transport.

```xml
<!-- The release posture: no cleartext, no user-installed CAs, TLS 1.2+. -->
<network-security-config>
    <base-config cleartextTrafficPermitted="false">
        <trust-anchors>
            <certificates src="system" />
        </trust-anchors>
    </base-config>
</network-security-config>
```

| Rule | Why |
| --- | --- |
| `cleartextTrafficPermitted="false"` | A plaintext request is a plaintext secret |
| Only the system CAs in release | A user-installed CA is an interception proxy — legitimate for debugging, not for release |
| `debug-overrides` for the debug build | The developer needs the proxy; the user does not |
| Certificate pinning only with a backup pin and a rotation plan | A pinning mistake bricks the app on the pinned devices |

Pinning, done carefully: pin the **intermediate** or the leaf, keep a
**backup pin**, and have a remote kill switch for the pin set. A pin set
that cannot be rotated remotely is an outage waiting for a certificate
rotation.

## 5. The manifest

| Rule | Why |
| --- | --- |
| `android:exported` is explicit on every component that has an intent filter | The default is `false` for a filter-less component and `true` for one with a filter — the explicit value removes the guess |
| `PendingIntent`s are `FLAG_IMMUTABLE` | A mutable `PendingIntent` lets another app rewrite the intent it carries |
| No `PendingIntent` trampolines | Android 12+ forbids routing a notification tap through a service or receiver; the `PendingIntent` goes to an Activity directly ([10](10-push-notifications.md)) |
| The App Links intent filter is `android:autoVerify="true"` | An unverified link can be claimed by any app ([06](06-navigation-routes-and-deep-links.md)) |
| Permissions are declared only for what the app uses, and justified in the Data safety form | Play reviews the permission set |

### The permission register

Every permission the app declares is listed here, with the feature that
needs it and the prompt that explains it. **A permission that is not in
this table does not ship.**

| Permission | Needed for | Prompt | Status |
| --- | --- | --- | --- |
| `INTERNET` | The network client | None — a normal permission | **Not declared** — the template has no network client ([07](07-auth-and-platform-features.md)) |
| `ACCESS_NETWORK_STATE` | The connectivity check before a request | None | Not declared — same reason |
| `POST_NOTIFICATIONS` | The notifications (API 33+) | At the point of use, after a primer | Not declared — the template has no push pipeline ([10](10-push-notifications.md)) |
| `RECEIVE_BOOT_COMPLETED` | A scheduled sync after a reboot | None | Not declared — no scheduled work ([08](08-offline-outbox-and-sync.md)) |
| `CAMERA` | A capture feature | At the point of use | Not declared — no capture feature |
| `ACCESS_FINE_LOCATION` | A location feature | At the point of use, with a reason | Not declared — no location feature |

The table is the check list for the release: the merged manifest of the
release variant is compared against it, and a permission that appears in
the manifest but not in this table is a finding
([Admin Guide](admin-guide.md) §4).

## 6. Integrity

**Play Integrity** tells the server whether the app and the device look
genuine.

| Rule | Why |
| --- | --- |
| The verdict is evaluated **server-side** | A client-side check can be patched out |
| It is a **risk signal**, not a block | Blocking on a false positive costs a legitimate user their access |
| Root signals are **advisory** | A rooted device is not automatically an attacker |
| The standard request type is enough for most apps | The higher tiers are for apps with a real fraud problem |

The client sends the request token; the server exchanges it and decides.
The client never makes the access decision.

## 7. Play policy

The checks that bite at submission time:

| Topic | Requirement |
| --- | --- |
| Target API | New apps and updates must target the current floor |
| Native code | 16 KB page-size support |
| Billing | Play Billing at the currently required major, for digital goods |
| Account deletion | **Both** an in-app path and a web path ([07](07-auth-and-platform-features.md)) |
| Permissions | Background location, foreground-service types and notifications justified; no unnecessary sensitive permissions |
| Data safety | The form must match the actual SDK and data behaviour |
| UGC and AI | Reporting, blocking, moderation, disclosure |
| Content rating | Must reflect chat, voice, purchases and location |
| Edge-to-edge and predictive back | Verified at the current target API |
| Loot boxes | Odds disclosure and the regional restrictions, where applicable |

The **Data safety form is generated from what the app actually does**, not
from what it intends to do. A dependency that collects data changes the
form; that is why the dependency register
([Dependency Register](dependency-register.md)) exists.

## 8. The template's posture today

| Control | State in the template |
| --- | --- |
| `network_security_config.xml` | **Not present** — the app makes no network call |
| The Keystore and Tink | **Not present** — no secret is stored |
| `dataExtractionRules` | **Not present** — nothing to exclude |
| `FLAG_SECURE` | **Not present** — no sensitive screen |
| R8 in the release build | **To be configured** when a release build is added |
| Play Integrity | **Not present** |
| The redacting logger | **Not present** — the template writes no log at all ([11](11-telemetry.md)) |
| The `notes` data | An id and a title, in memory, never persisted |

The template's security posture is honest: **it holds nothing sensitive and
reaches nothing**. Every control in this chapter becomes a real
requirement the moment the app gains a session, a cache or a network call —
which is exactly when §9 has to be executed, not before.

## 9. The adoption path

1. **Add `:core:security`.** An Android library: the `SecureStorePort`
   implementation over Tink + Keystore, the network security config, and
   the integrity client.
2. **Add the network security config** to the app module and to the
   manifest. Start with the release posture above.
3. **Add the secret store** when the app gains a session
   ([07](07-auth-and-platform-features.md)): the refresh token is the first
   secret, and it goes in the secure store, never in DataStore.
4. **Add `dataExtractionRules`** excluding the token stores, and set
   `allowBackup="false"`.
5. **Add the R8 release configuration**: full mode, resource shrinking,
   `isDebuggable = false`, the keep rules per library.
6. **Add Play Integrity** as a server-evaluated signal when the fraud model
   justifies it.
7. **Write the Play data safety form** from the dependency register.

## Do / don't

| Do | Don't |
| --- | --- |
| Encrypt the refresh token with a Keystore-backed key | Store a token in DataStore |
| Use a small, fixed key set | Create a key per secret |
| Keep the base URL and a restricted client id in `BuildConfig` | Put an API key in `BuildConfig` |
| Disable cleartext in release | Ship a cleartext fallback "for debugging" |
| Make every `PendingIntent` immutable | Use a mutable `PendingIntent` |
| Evaluate the integrity verdict on the server | Block the app on a client-side check |

## See also

- [Auth & Platform Features](07-auth-and-platform-features.md) — where the
  tokens live.
- [Telemetry](11-telemetry.md) — the redaction that keeps secrets out of
  the logs.
- [Push Notifications](10-push-notifications.md) — the `PendingIntent` rules
  for a notification tap.
- [Admin Guide](admin-guide.md) — the managed policy that can forbid a
  feature.
- [Playbook Native Android Kotlin §15](../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md)
  — the security baseline.
