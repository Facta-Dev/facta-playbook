# Auth & Platform Features

Onboarding, sign-in, registration and password recovery are **the same
feature on every client** of the platform: the backend contract, the UX
rules and the flows are shared, and only the implementation differs. This
chapter is about the iOS implementation of that shared contract, and about
what the template ships.

> **Known gap — do not copy a fiction.** The template **does not implement**
> authentication. There is no network client, no SDK, no token store, no
> Keychain wrapper and no `AuthenticationServices` integration in the tree.
> The app declares **no entitlement** and **no `Info.plist` usage key**. This
> chapter specifies the contract a project must implement, the seams it must
> open, and the order in which to build it.

## 1. The shared contract

The backend contract is the one the other clients implement. The iOS app
speaks the same protocol; the iOS playbook fixes the group
(`/api/v1/public/auth/*`) and the device-registration endpoint, and the
per-flow paths are the platform contract that every client shares.

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
| Assets are static images | A looping heavy animation is a startup cost on a Low-tier device |
| Reduce Motion is honoured | The user's reduce-motion setting is respected |
| Dynamic Type is supported up to the accessibility sizes | Accessibility is a floor, not a feature |

Onboarding is a **sequence of routes**, not a special mode: each step is a
case of a route enum in a slice's `Contracts` package, and the host renders
them in a sequence. That keeps the onboarding flow on the same navigation
machinery as everything else
([06](06-navigation-routes-and-deep-links.md)).

## 3. Registration

| Rule | Reason |
| --- | --- |
| Minimal fields | Every extra field costs completions and creates a PII liability |
| Password-policy feedback is inline | The rule is visible before the submit |
| AutoFill is integrated (`textContentType(.username)`, `.newPassword`, `.emailAddress`) | The platform's AutoFill and the Strong Password suggestion are the best password manager there is |
| Consent is captured **per version** | A consent change is a new version, not a new field |
| Email/phone verification uses a **Universal Link** or an OTP | — |
| The OTP field uses `textContentType(.oneTimeCode)` | The SMS AutoFill delivers the code without the app reading the message |

There is **no SMS permission on iOS** — the platform has none, and the app
never reads the Messages database.

### Abuse controls

| Control | Where |
| --- | --- |
| Rate limiting | Server-side; the client shows a generic message and honours `Retry-After` |
| **App Attest** assertion (+ **DeviceCheck**) | A **server-evaluated** risk signal; never a client-side block |
| CAPTCHA | Only on high risk, never on the first attempt |

App Attest is the iOS analogue of Play Integrity: the device produces an
assertion, the **server** verifies it against Apple's service and decides
the risk. A client that blocks on its own verdict is a client an attacker
patches.

## 4. Sign-in

### AuthenticationServices

`AuthenticationServices` is the single entry point for the credentials the
user has: **Sign in with Apple**, **passkeys**, and — through
`ASWebAuthenticationSession` — the enterprise SSO flow. The first-party form
is the fallback when the platform has nothing to offer.

| Capability | API |
| --- | --- |
| Sign in with Apple | `ASAuthorizationAppleIDProvider` / `SignInWithAppleButton` |
| Passkeys | `ASAuthorizationPlatformPublicKeyCredentialProvider` (sign-in and registration) |
| Enterprise / social SSO | `ASWebAuthenticationSession` with OIDC Authorization Code + PKCE |
| Biometric gate | `LocalAuthentication` (`LAContext`) in front of a Keychain item |

The rule that matters: **AuthenticationServices returns a credential, never
a session**. The app exchanges the credential with the backend over the same
HTTPS transport as every other call. It is not an authentication service.

> **Known gap.** None of these are in the tree. `grep` for
> `AuthenticationServices`, `ASWebAuthenticationSession` and `LocalAuthentication`
> across `Packages/` and `Apps/` returns nothing, and no `Info.plist` usage
> key (including `NSFaceIDUsageDescription`) is declared.

**Sign in with Apple is a policy requirement**, not a preference: if the app
offers other third-party social logins, it must offer Sign in with Apple or
an equivalent privacy-preserving option.

### Enterprise SSO

OIDC Authorization Code with **PKCE**, through **`ASWebAuthenticationSession`**.
There is no embedded `WKWebView` login: a WebView login is a phishing
surface and it breaks the system credential managers.

### Step-up and biometrics

| Action | Requirement |
| --- | --- |
| Payment, password change, account deletion | Step-up authentication |
| Sensitive local data | A `LocalAuthentication` gate on a **Keychain item** protected by `SecAccessControl` (`.biometryCurrentSet` or `.userPresence`) |

**Biometrics never replace server authentication.** They gate a local key,
and the server still decides.

### Throttling

Server-side. The client shows a generic error and honours `Retry-After`; it
never reveals whether the account exists.

## 5. Session and token storage

| Token | Where it lives | Why |
| --- | --- | --- |
| Access token | **In memory only** | A short-lived token in memory is not at rest |
| Refresh token | **Keychain**, `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` (or `WhenUnlockedThisDeviceOnly` for stricter apps) | A stolen file is useless without the device |
| User settings | `UserDefaults` / `@AppStorage` | Not a secret |

The refresh token is **non-synchronizable**, and it uses an app-specific
access group only if an extension needs it.

```swift
// The shape of the secure store — the seam the domain and the UI see.
// The domain and the UI never see a Keychain query, and the implementation
// is replaceable: a fake in the unit tests, the real Keychain in the app.
public protocol SecureStorePort: Sendable {
    func get(_ key: String) async throws -> Data?
    func set(_ value: Data, for key: String) async throws
    func remove(_ key: String) async throws
}
```

Three more rules:

| Rule | Why |
| --- | --- |
| Sensitive files use a Data Protection class (`complete` / `completeUnlessOpen`) | A file is only as protected as its class |
| **A small, fixed set of Keychain items** — one master item per purpose | One key per token does not scale and each item is another thing to migrate |
| Single-flight refresh (an actor) | Five concurrent 401s must not produce five refreshes |

On sign-out, on account deletion and on suspension, the tokens are wiped
immediately — not when the process dies.

> **Known gap.** There is **no Keychain wrapper, no `SecureStorePort` and
> no `Security` import** in the tree. The encryption details are in
> [Security & Keychain](09-security-and-keychain.md).

## 6. Device registration

The app registers the device with the backend so the backend can send push
to it.

| Field | Meaning |
| --- | --- |
| `platform` | `"ios"` |
| `token` | The APNs device token ([10](10-push-notifications.md)) |
| `provider` | `"apns"` |
| `appVariant` | Which product this build is |
| `version` | The build |
| `locale` | For notification language |
| `tz` | For scheduling |
| `notificationsEnabled` | The user's choice |

Registration happens **after login**, on every app start, and again on every
token refresh. On logout or account deletion, the device is unregistered.

> **Known gap.** No APNs registration, no `aps-environment` entitlement and
> no registration call exist in the tree.

## 7. Account deletion

Two paths, both required by the App Store's data-deletion policy:

| Path | What it does |
| --- | --- |
| **In-app** | Initiate → re-authenticate → grace period → confirmation |
| **Web** | A page linked from the App Store listing, for the case where the user has lost the device |

The server cascades the deletion to the devices, the push tokens, the cached
assets, the location history and the server-side caches. The app wipes the
local projection and the tokens in the same flow. **Sign in with Apple
tokens are revoked** on deletion, as Apple's REST API requires.

## 8. The permission primers

iOS permissions are requested **at the point of use**, never all at first
launch. Each one needs a usage string in the `Info.plist`, and the template
declares none.

| Permission | Mechanism | Usage string | Notes |
| --- | --- | --- | --- |
| Notifications | `UNUserNotificationCenter.requestAuthorization` (alert/badge/sound; provisional optional) | — | Show a primer first; the system prompt appears once, afterwards deep-link to Settings |
| Location | `CLLocationManager` **When In Use** first; **Always** only when needed | `NSLocationWhenInUseUsageDescription`, `NSLocationAlwaysAndWhenInUseUsageDescription` | Two-step flow; the strings must be specific |
| Camera | `AVCaptureDevice` / ARKit | `NSCameraUsageDescription` | Prefer the **PhotosPicker**, which needs no permission |
| Microphone | `AVAudioApplication.requestRecordPermission` | `NSMicrophoneUsageDescription` | Show an in-app live-mic cue |
| Face ID | `LocalAuthentication` | `NSFaceIDUsageDescription` | For the biometric lock |
| Tracking | `ATTrackingManager` | `NSUserTrackingUsageDescription` | Only if you actually track across apps/sites — otherwise omit it |

Every denial has a **degraded path**. A denied location permission means a
map without the user's position, not a broken screen.

> **Known gap.** No usage string, no entitlement and no permission request
> exists in the tree. `project.yml` sets `GENERATE_INFOPLIST_FILE: YES`, so
> the strings would be added as `INFOPLIST_KEY_*` settings or through an
> `Info.plist` file — the second choice is the one that scales past a
> handful of keys.

## 9. What the template ships, and what it does not

| Piece | State |
| --- | --- |
| The route cases for the session-less screens (`/login`, `/register`, `/onboarding`, …) | **Not implemented** — `NotesContracts` ships only `NotesRoute` (`list`, `detail`) |
| A network client and an SDK | **Not implemented** — there is no `URLSession` call in the tree |
| A `SecureStorePort` and a Keychain wrapper | **Not implemented** — the seam is described in §5 |
| The `AuthenticationServices` integration (Sign in with Apple, passkeys) | **Not implemented** — the adoption path is in §4 |
| The `ASWebAuthenticationSession` OIDC flow | **Not implemented** |
| The onboarding screens | **Not implemented** — the flow shape is in §2 |
| The device registration call | **Not implemented** — the payload is in §6 |
| The App Attest / DeviceCheck assertion | **Not implemented** — the risk signal is described in §3 |
| An entitlements file | **Not implemented** — `project.yml` references none |
| The `Info.plist` usage strings | **Not implemented** — the keys are listed in §8 |

The template ships the **route pattern** — a typed route case per screen,
in a pure Swift `Contracts` package — because that is the decision a
project would otherwise have to reverse. It does not ship the session
routes themselves. The rest is the adoption path, in the order of §10.

## 10. The adoption path

1. **Add the SDK.** A `URLSession` client over the shared error envelope,
   in a `Packages/Core/SDK` package. The error envelope wins over the status
   code.
2. **Add `Packages/Core/Security`.** The `SecureStorePort` and its
   Keychain-backed implementation, with `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`.
3. **Add the session slice.** `Packages/Feature/Session/{Contracts,Domain,Data,UI}`
   with the sign-in, registration and recovery screens, and the route cases
   for them.
4. **Open the port in `AppEnvironment`.** The composition root builds the
   SDK, the secure store and the repository
   ([ADR 0003](adr/0003-explicit-app-environment.md)).
5. **Add `AuthenticationServices`**, behind the session port, so the
   fallback form stays available.
6. **Add the entitlements and the usage strings** — Sign in with Apple,
   Associated Domains, Push — through the manifest, never by hand-editing
   the generated project.
7. **Add the device registration**, driven by the APNs token
   ([10](10-push-notifications.md)).

At every step the package graph stays legal, and the architecture check
stays green.

## Do / don't

| Do | Don't |
| --- | --- |
| Exchange the credential with the server | Treat `AuthenticationServices` as a session |
| Keep the access token in memory | Write it to `UserDefaults` |
| Use one Keychain item per purpose | Create an item per token |
| Request a permission at the point of use | Ask for everything at first launch |
| Let the server evaluate the App Attest assertion | Block on a client-side verdict |
| Delete the account in-app **and** on the web | Ship only one of the two paths |
| Use `ASWebAuthenticationSession` for SSO | Embed a `WKWebView` login |

## See also

- [Security & Keychain](09-security-and-keychain.md) — the encryption
  details.
- [Push Notifications](10-push-notifications.md) — the device registration
  and the token.
- [Offline, Outbox & Sync](08-offline-outbox-and-sync.md) — the mutation
  path that must carry an idempotency key.
- [Navigation Routes & Deep Links](06-navigation-routes-and-deep-links.md) —
  the session-less routes and the guards.
- [Playbook Native iOS Swift §7](../PLAYBOOK_MOBILE_IOS_SWIFT.md) — the
  shared flows.
