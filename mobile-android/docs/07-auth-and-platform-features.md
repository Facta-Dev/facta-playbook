# Auth & Platform Features

Onboarding, sign-in, registration and password recovery are **the same
feature on every client** of the platform: the backend contract, the UX
rules and the flows are shared, and only the implementation differs. This
chapter is about the Android implementation of that shared contract, and
about what the template ships.

> **Known gap — do not copy a fiction.** The template **does not implement**
> authentication. There is no network client, no SDK, no token store and no
> `Credential Manager` integration in the tree. This chapter specifies the
> contract a project must implement, the seams it must open, and the order in
> which to build it.

## 1. The shared contract

The backend contract is the one the other clients implement. The Android app
speaks the same protocol.

| Flow | Backend surface |
| --- | --- |
| Sign-in | `POST /api/v1/public/auth/login` |
| Registration | `POST /api/v1/public/auth/register` |
| Forgot password | `POST /api/v1/public/auth/forgot-password` |
| Reset password | `POST /api/v1/public/auth/reset-password` |
| Device registration | `POST /api/v1/app/devices` |
| Session revocation | `POST /api/v1/app/auth/sessions/revoke` |

Two rules of the contract that the client must not soften:

| Rule | Why |
| --- | --- |
| The forgot-password response is **uniform** | The server must not reveal whether an account exists |
| A reset token is **single-use and short-lived** | A leaked link must be worth nothing |

The **server** owns the session, the entitlements and the token rotation.
The app is untrusted.

## 2. Onboarding

Three to five skippable screens, driven by remote configuration with a
bundled fallback.

| Rule | Reason |
| --- | --- |
| Every screen is skippable | A user who does not finish onboarding still uses the app |
| Value first, permissions later | The permission prompt comes at the point of use, not at first launch |
| The content is remote-configurable with a bundled fallback | The app works offline and before the first fetch |
| Assets are static WebP/AVIF | A looping heavy animation is a startup cost on a Low device |
| `ANIMATOR_DURATION_SCALE` is honoured | The user's reduce-motion setting is respected |
| Large font scale is supported | Accessibility is a floor, not a feature |

Onboarding is a **sequence of routes**, not a special mode: each step is a
`NavKey` in the slice's `contracts` module, and the host renders them in a
sequence. That keeps the onboarding flow on the same navigation machinery
as everything else.

## 3. Registration

| Rule | Reason |
| --- | --- |
| Minimal fields | Every extra field costs completions and creates a PII liability |
| Password policy feedback is inline | The rule is visible before the submit |
| Autofill is integrated (`ContentType.Username` / `Password` / `NewPassword`) | The platform's autofill is the best password manager there is |
| Consent is captured **per version** | A consent change is a new version, not a new field |
| Email/phone verification uses an **App Link** or an OTP | — |
| **Never read SMS with an SMS permission** | Android 17 delays SMS-delivered OTP access for apps that are not the intended recipient |

The OTP rule is a platform change, not a preference. The supported
approaches are the **SMS Retriever API** (the app gets the code without
reading the message) and the **SMS User Consent API**. Reading the SMS with
`RECEIVE_SMS` is blocked on modern Android.

### Abuse controls

| Control | Where |
| --- | --- |
| Rate limiting | Server-side; the client shows a generic message and honours `Retry-After` |
| Play Integrity verdict | A **server-evaluated** risk signal; never a client-side block |
| CAPTCHA | Only on high risk, never on the first attempt |

## 4. Sign-in

### Credential Manager

`androidx.credentials` is the single entry point for the credentials the
user has: passkeys, saved passwords and **Sign in with Google**
(`GetGoogleIdOption`). The first-party form is the fallback when the
platform has nothing to offer.

```kotlin
// The shape of a Credential Manager sign-in. The request carries no secret:
// the platform returns the credential to the app, and the app exchanges it
// with the server.
val request = GetSignInWithGoogleOption.GoogleIdOption(
    filterByAuthorizedAccounts = false,
    serverClientId = /* the OAuth client id from the server */,
)
```

The rule that matters: **Credential Manager returns a credential, never a
session**. The app exchanges the credential with the backend over the same
HTTPS transport as every other call. Credential Manager is not an
authentication service.

### Enterprise SSO

OIDC Authorization Code with **PKCE**, through **AppAuth-Android** and
**Custom Tabs**. There is no embedded WebView login: a WebView login is a
phishing surface and it breaks the system password manager.

### Step-up and biometrics

| Action | Requirement |
| --- | --- |
| Payment, password change, account deletion | Step-up authentication |
| Sensitive local data | A `BiometricPrompt` gate on a **Keystore-bound** key (`setUserAuthenticationRequired`) |

**Biometrics never replace server authentication.** They gate a local key,
and the server still decides.

### Throttling

Server-side. The client shows a generic error and honours `Retry-After`; it
never reveals whether the account exists.

## 5. Session and token storage

| Token | Where it lives | Why |
| --- | --- | --- |
| Access token | **In memory only** | A short-lived token in memory is not at rest |
| Refresh token | **Encrypted**, with the key in the Android Keystore | A stolen file is useless without the device |
| User settings | `DataStore` | Not a secret |

The encryption is **Tink** with an **AES-GCM** key that lives in the
**Android Keystore**. The DataStore or file holds only ciphertext.

```kotlin
// The shape of the secure store. The key never leaves the Keystore, and the
// plaintext never reaches the disk.
interface SecureStorePort {
    suspend fun get(key: String): ByteArray?
    suspend fun put(key: String, value: ByteArray)
    suspend fun remove(key: String)
}
```

The port is the seam: the domain and the UI never see a Keystore, and the
implementation is replaceable — a fake in the unit tests, the real Keystore
in the app.

Three more rules:

| Rule | Why |
| --- | --- |
| `android:allowBackup="false"` and a `dataExtractionRules` file that excludes the token stores | A backup must not carry the refresh token to another device |
| A **small, fixed set of Keystore keys** — one master key per purpose | Android 17 caps the number of Keystore keys per app; a key per token does not scale |
| Single-flight refresh (a `Mutex` in the authenticator) | Five concurrent 401s must not produce five refreshes |

On sign-out, on account deletion and on suspension, the tokens are wiped
immediately — not when the process dies.

## 6. Device registration

The app registers the device with the backend so the backend can send push
to it.

| Field | Meaning |
| --- | --- |
| `platform` | `"android"` |
| `token` | The FCM registration token ([10](10-push-notifications.md)) |
| `provider` | `"fcm"` |
| `appVariant` | Which product this build is |
| `versionCode` | The build |
| `locale` | For notification language |
| `tz` | For scheduling |
| `notificationsEnabled` | The user's choice |
| `channelStates` | The per-category channel states |

Registration happens **after login**, on every app start, and again on every
`onNewToken` callback. On logout or account deletion, the device is
unregistered.

## 7. Account deletion

Two paths, both required by Play's data-deletion policy:

| Path | What it does |
| --- | --- |
| **In-app** | Initiate → re-authenticate → grace period → confirmation |
| **Web** | A page linked from the Play listing, for the case where the user has lost the device |

The server cascades the deletion to the devices, the push tokens, the
cached assets, the location history and the server-side caches. The app
wipes the local projection and the tokens in the same flow.

## 8. The permission primers

Android's runtime permissions are requested **at the point of use**, never
all at first launch.

| Permission | Mechanism | Notes |
| --- | --- | --- |
| Notifications (API 33+) | `POST_NOTIFICATIONS` via `ActivityResultContracts.RequestPermission` | Show a primer first; after repeated denials the system stops prompting, so deep-link to the app's notification settings |
| Location | Foreground first (`ACCESS_COARSE_LOCATION` / `ACCESS_FINE_LOCATION`); background (`ACCESS_BACKGROUND_LOCATION`) only after the foreground grant, and only if it is genuinely needed | Two-step flow; Play declaration and justification |
| Camera | `CAMERA` | Prefer the **Photo Picker**, which needs no storage or camera permission |
| Microphone | `RECORD_AUDIO` | Show a live-microphone indicator in the app |

Every denial has a **degraded path**. A denied location permission means a
map without the user's position, not a broken screen.

## 9. What the template ships, and what it does not

| Piece | State |
| --- | --- |
| The route types for the session-less screens (`/login`, `/register`, `/onboarding`, …) | **Not implemented** — `contracts` ships only the `notes` routes |
| A network client and an SDK | **Not implemented** — there is none in the tree |
| A `SecureStorePort` | **Not implemented** — the seam is described in §5 |
| The `Credential Manager` integration | **Not implemented** — the adoption path is in §4 |
| The onboarding screens | **Not implemented** — the flow shape is in §2 |
| The device registration call | **Not implemented** — the payload is in §6 |

The template ships the **route pattern** — a typed `NavKey` route per
screen, in a pure JVM `contracts` module — because that is the decision a
project would otherwise have to reverse. It does not ship the session
routes themselves. The rest is the adoption path, in the order of §10.

## 10. The adoption path

1. **Add the SDK.** A Kotlin client over OkHttp with kotlinx.serialization,
   in a `:core:sdk` module. The error envelope wins over the status code.
2. **Add `:core:security`.** The `SecureStorePort` and its Keystore-backed
   implementation, plus the Tink AEAD.
3. **Add the session slice.** `feature/session/{contracts,domain,data,ui}`
   with the sign-in, registration and recovery screens.
4. **Open the port in `:app`.** The composition root builds the SDK, the
   secure store and the repository.
5. **Add Credential Manager**, behind the `SessionPort`, so the fallback
   form stays available.
6. **Add the device registration**, driven by the push token
   ([10](10-push-notifications.md)).

At every step the module graph stays legal, and the architecture check
stays green.

## Do / don't

| Do | Don't |
| --- | --- |
| Exchange the credential with the server | Treat Credential Manager as a session |
| Keep the access token in memory | Write it to DataStore |
| Use one Keystore key per purpose | Create a key per token |
| Request a permission at the point of use | Ask for everything at first launch |
| Use the SMS Retriever or an email OTP | Read the SMS with an SMS permission |
| Delete the account in-app **and** on the web | Ship only one of the two paths |

## See also

- [Security & Keystore](09-security-and-keystore.md) — the encryption
  details.
- [Push Notifications](10-push-notifications.md) — the device registration
  and the token.
- [Offline, Outbox & Sync](08-offline-outbox-and-sync.md) — the mutation
  path that must carry an idempotency key.
- [Playbook Native Android Kotlin §7](../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md)
  — the shared flows.
