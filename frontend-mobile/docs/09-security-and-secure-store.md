# Security & Secure Store

A mobile app ships its code to the device. The bundle is readable, the
filesystem is readable on a compromised device, and a rooted phone is a real
device in the field. The posture is therefore: **assume the client is
untrusted, keep the secrets out of reach, and let the server decide.**

## 1. The rules

| #   | Rule                                                            | Where it is enforced           |
| --- | --------------------------------------------------------------- | ------------------------------ |
| 1   | The access token lives in the OS keystore, never in the bundle  | `SecureStorePort`              |
| 2   | No secret is ever a build-time constant                         | The secret scan in CI          |
| 3   | The refresh token never reaches the JavaScript bundle in clear  | The secure store adapter       |
| 4   | TLS everywhere; certificate pinning where the risk justifies it | The network adapter            |
| 5   | Device integrity is a **signal**, never a decision              | The server evaluates it        |
| 6   | A deep link is untrusted input                                  | The route parser               |
| 7   | The URL never authorizes                                        | The server                     |
| 8   | Nothing sensitive is logged                                     | The redacting logger (adopted) |

## 2. The secure store

`packages/core/src/ports.ts` declares the contract:

```ts
export interface SecureStorePort {
  get(key: string): Promise<string | null>;
  set(key: string, value: string): Promise<void>;
  remove(key: string): Promise<void>;
}
```

The implementation a product writes is over `expo-secure-store`, which is
the iOS Keychain and the Android Keystore. The template ships the port, not
that adapter. What the implementation buys:

| Property                                      | Why it matters                                       |
| --------------------------------------------- | ---------------------------------------------------- |
| The value is encrypted at rest                | A file dump of the app data does not yield the token |
| The value is outside the app's data directory | An `adb` file pull does not reach it                 |
| The value survives an app restart             | The session survives                                 |
| The value is removed with the app uninstall   | A reinstall starts clean                             |

Two caveats, both real:

- **The keystore is not a vault.** On a rooted or jailbroken device, a
  determined attacker with the right tools can read the Keychain. The
  keystore raises the cost; it does not end the game.
- **Size limits.** Secure-store values have a size ceiling (of the order of
  a few kilobytes). A token fits; a cached list does not. The projection is
  SQLite, not the keystore.

## 3. No secrets in the bundle

The JavaScript bundle is **not** a secret store. It is a file on the device
that anyone can extract and read. Concretely:

| Never in the bundle          | Instead                                           |
| ---------------------------- | ------------------------------------------------- |
| An API key                   | A backend proxy that holds it                     |
| A service-account credential | A backend that signs its own requests             |
| A shared secret              | A per-user token in the keystore                  |
| A private signing key        | The server signs; the app requests                |
| A hard-coded environment URL | The environment is build-time, but never a secret |

Hermes bytecode is not obfuscation. Hermes ships bytecode; it is not a
secret container, and a determined analyst decompiles it.

> **Known gap.** The template ships the `SecureStorePort` declaration and the
> token-handling **contract**, not a production adapter. A product implements
> the port over `expo-secure-store`
> ([Playbooks §3](13-playbooks.md#3-add-a-port-and-its-adapter)).

## 4. Token handling

| Token          | Where it lives                | Lifetime                      | Refresh                          |
| -------------- | ----------------------------- | ----------------------------- | -------------------------------- |
| Access token   | The secure store              | Minutes                       | Refreshed from the refresh token |
| Refresh token  | The secure store              | Days, rotating                | Rotated on every refresh         |
| Device token   | The secure store + the server | Until rotated by the provider | Re-registered on rotation        |
| Correlation id | Memory                        | One user action               | Never persisted                  |

The rules:

1. **The access token goes in a header**, never in a URL, never in a query
   parameter, never in a log.
2. **The refresh token rotates.** A refresh returns a new refresh token and
   invalidates the old one; a reused refresh token revokes the family.
3. **Sign-out clears the store**, unregisters the device for push, and
   clears the projection according to the retention policy.
4. **The token is never logged.** The redacting logger removes it by shape as
   well as by key name ([11](11-telemetry.md)).

## 5. Certificate pinning

TLS 1.2+ is the floor. **Pinning** goes further: the app accepts only the
certificate or public key of the servers it expects.

| Aspect           | Rule                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------- |
| What is pinned   | The public key of the API host, plus one **backup** key                               |
| Rotation         | The backup key is published first; the old key is removed after the fleet has updated |
| The escape hatch | A remote-config kill switch that disables pinning, for an outage                      |
| Not pinned       | Third-party hosts the app does not control (a CDN, a payment provider)                |
| Bypass           | Never in a release build; only in a debug build, behind a flag                        |

> **Known gap.** The template ships **no pinning implementation**. Pinning
> without a rotation plan and a kill switch locks you out of your own API
> on the day the certificate changes. Read the section, adopt it as a
> deliberate step, and test the rotation.

## 6. Device integrity and rooted devices

| Signal                        | Source                            | What it is worth                          |
| ----------------------------- | --------------------------------- | ----------------------------------------- |
| Play Integrity / App Attest   | The platform's attestation API    | A **signal** the server evaluates         |
| A rooted or jailbroken device | A native check or a heuristic     | A **signal**, often noisy and often wrong |
| A debuggable build            | The build configuration           | A hard signal in a release build          |
| A tampered bundle             | A signature check on the artifact | A strong signal                           |

The rules that matter:

1. **Never block on a client-side check alone.** A determined attacker
   patches the check out; a false positive locks out a paying customer.
   The client sends the signal, the server decides.
2. **A rooted device is not a jailbreak sentence.** A developer, a tester
   and a fraudster all have a rooted device. The signal escalates a
   transaction; it does not end the session.
3. **Never log the raw attestation verdict.** Log a boolean or an
   evaluation result.

## 7. The deep link and the notification payload

Both arrive from outside and both are untrusted:

| Input                 | The check                                                                  |
| --------------------- | -------------------------------------------------------------------------- |
| A deep link path      | Parsed against the route table; unknown route → home; parameters validated |
| A notification `type` | Checked against the known types; unknown types are ignored and counted     |
| A notification `data` | Validated by the payload schema before it reaches a view model             |
| A deep link action    | **Never** a state change without an explicit user confirmation             |

A link that says "delete this note" does not delete it. A link that says
"open this note" opens it. The difference is a confirmation dialog.

## 8. The clipboard, the screenshots and the switcher

| Surface          | Control                                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------- |
| The clipboard    | Copy only what the user asked to copy; clear on a timer                                  |
| The app switcher | A privacy overlay on sensitive screens where the platform allows one                     |
| Screenshots      | `FLAG_SECURE`-style protection on the screens that need it, where the platform allows it |
| The keyboard     | Disable autocorrect and secure text entry on a password field                            |

These are platform capabilities, not portable ones: the two platforms
differ, and the template ships the intent, not the implementation.

## 9. What the template ships

| Piece                                                    | Status                                                                     |
| -------------------------------------------------------- | -------------------------------------------------------------------------- |
| `SecureStorePort` declaration                            | **Shipped**, `packages/core/src/ports.ts`                                  |
| The token-handling contract (storage, refresh, sign-out) | **Not shipped** — the port is the contract; the flow is a product decision |
| The deep-link parsing rules                              | **Shipped**, `packages/contracts/src/routes.ts`                            |
| The `expo-secure-store` adapter                          | **Not shipped** — a product decision                                       |
| Certificate pinning                                      | **Not shipped** — needs a rotation plan                                    |
| Integrity attestation                                    | **Not shipped** — the server must evaluate it                              |
| A secret scan in CI                                      | **Not shipped** — see [12](12-testing-and-ci.md)                           |

## Do / Don't

| Do                                          | Don't                                      |
| ------------------------------------------- | ------------------------------------------ |
| Keep the token in the keystore              | Put the token in the bundle or in SQLite   |
| Rotate the refresh token on every refresh   | Reuse a refresh token                      |
| Treat an attestation verdict as a signal    | Block the user on a client-side check      |
| Confirm a destructive action from a link    | Act on a payload without confirmation      |
| Ship a backup pinning key and a kill switch | Pin without a rotation plan                |
| Treat a deep link as untrusted input        | Trust a link because the OS opened the app |

## See also

- [Navigation, Routes & Deep Links](07-navigation-routes-and-deep-links.md)
- [Push Notifications](10-push-notifications.md)
- [Telemetry](11-telemetry.md)
