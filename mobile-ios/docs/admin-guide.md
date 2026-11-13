# Admin Guide

How an administrator configures and verifies a native iOS deployment: the
prerequisites, the instantiation of the template, the managed configuration,
the remote configuration, the App Store Connect setup, the signing material,
and the checks that prove a deployment is what it claims to be.

> **Known gap — do not copy a fiction.** The template ships **no
> configuration reader, no App Store Connect integration, no signing
> configuration, no capability, no entitlement and no privacy manifest**. It
> ships the seams: a `Kernel` that can carry a typed configuration value, an
> `AppEnvironment` that builds the graph by hand
> ([ADR 0003](adr/0003-explicit-app-environment.md)), and the chapters that
> specify what a project must add. Every section below states what is missing
> and what it takes to close the gap.
>
> The `notes` slice persists nothing, talks to no server and pushes no
> notification. The flows this guide describes beyond the first two are the
> ones a consumer builds.

## 1 · Prerequisites

The kit has two tiers. The portable tier runs anywhere; the native tier needs
a Mac.

| Requirement | Why | How it is checked |
| --- | --- | --- |
| **macOS** | The native tier compiles with the Apple toolchain. Off macOS the native tier prints a visible `SKIP` and exits 0 — the kit stays valid, its native proof moves to a Mac or a CI runner. | `uname -s` returns `Darwin` |
| **Xcode 16+** | `swift test` per package and `xcodebuild` for the app target. The packages declare `swift-tools-version: 6.0`, so the toolchain must be a Swift 6 one. | `xcodebuild -version` |
| **XcodeGen** | The project file is **generated** from `project.yml` and is git-ignored ([ADR 0001](adr/0001-xcodegen-generated-project.md)). | `xcodegen --version`; `scripts/native-gates.sh` prints a visible `SKIP` when it is missing |
| **Node 24** | The `.claude` contract evals only (`node --test evals/contracts.test.mjs`). The app never runs Node. | `node --version` |
| **bash** | Every gate is a shell script under `scripts/`. | `bash --version` |

Two version numbers are the template's own contract and are enforced, not
guessed:

| Setting | Value | Where |
| --- | --- | --- |
| Swift language mode | `6.0` | `SWIFT_VERSION: "6.0"` in `project.yml`; `swiftLanguageModes: [.v6]` in all six `Package.swift` files |
| Strict concurrency | `complete` | `SWIFT_STRICT_CONCURRENCY: complete` in `project.yml` |
| Deployment target | iOS `18.0` | `deploymentTarget` in `project.yml`; `platforms: [.iOS(.v18), .macOS(.v15)]` in the packages |

The App Store's own submission floor moves independently of the kit and must
be re-checked in Apple's documentation before a release — see
[Playbooks](13-playbooks.md).

## 2 · Instantiate the kit

The template ships **literal placeholders**: `<app>`, `<APP>` and `<scope>`.
They are not valid target names or bundle-identifier segments, so the template
root does not build as shipped.

### The rename step

```sh
bash scripts/rename.sh <new-app-slug> <new-org-scope>
```

| Argument | Replaces | Constraint |
| --- | --- | --- |
| First | `<app>` — and `<APP>`, uppercased | `^[a-z][a-z0-9]*$` |
| Second | `<scope>` | `^[a-z][a-z0-9]*$` |

The script substitutes in the order `<scope>` → `<APP>` → `<app>`, renames
the `Apps/<APP>` directory, and **excludes** `scripts/**` (the template
machinery keeps its literal patterns so the gates keep working after the
rename) and `evals/fixtures/**` (frozen test data whose residual tokens
exercise the leak detector). A slug that fails the pattern is rejected
*before* the tree is touched, so a half-renamed copy is impossible.

The result is visible in `project.yml`:

| Placeholder | Becomes | Example |
| --- | --- | --- |
| `name: <APP>` | The Xcode project and scheme name | `Acme` → `Acme.xcodeproj`, scheme `Acme` |
| `bundleIdPrefix: com.<scope>` | The bundle-id prefix | `com.acme` |
| `PRODUCT_BUNDLE_IDENTIFIER: com.<scope>.<app>` | The full bundle id | `com.acme.mycompany` |
| `INFOPLIST_KEY_CFBundleDisplayName: <APP>` | The on-device display name | `Mycompany` |

Run the rename **on a copy**, not in the kit you intend to publish:
`scripts/verify-template.sh` stages a throwaway copy in a temp directory,
renames it with the example values `demo` / `demoorg`, runs every gate on the
copy and discards it — the working tree is never mutated.

### Generating the project

```sh
xcodegen generate     # or: make generate
```

The `.xcodeproj` appears at the root and is git-ignored
(`*.xcodeproj/` in `.gitignore`). `scripts/check-architecture.sh` **fails** if
one is ever tracked: the manifest is the source of truth, the project is a
build artifact. `make clean` removes it; regenerating is one command.

## 3 · The gates

| Command | What it proves | Where it runs |
| --- | --- | --- |
| `bash scripts/check-architecture.sh` | The package graph: no forbidden edge, no SwiftUI/UIKit import in a pure package, no visual-builder artifact, no tracked `.xcodeproj`, Swift 6 mode in all six manifests | anywhere |
| `node --test evals/contracts.test.mjs` | The `.claude` layer: agents, skills, commands, the manifest posture | anywhere with Node |
| `bash scripts/native-gates.sh test` | `swift test` per package — Kernel, UICore, NotesContracts, NotesDomain, NotesData, NotesUI | macOS + Xcode |
| `bash scripts/native-gates.sh build` | `xcodegen generate` then `xcodebuild` for the app target | macOS + Xcode |
| `bash scripts/native-gates.sh` | Both tiers | macOS + Xcode |
| `make lint` | The architecture check; after rename, the three `.claude` linters and the instruction budget | anywhere |
| `make ci` | `lint` + the contract evals + the native tier, in one go | anywhere (the native part SKIPs) |
| `make verify` | A renamed copy passes every gate | anywhere |

Two facts an administrator must not misread:

- **A `SKIP` is not a pass.** Off macOS the native tier prints
  `native-gates: SKIP — macOS + Xcode are required (xcodegen, swift, xcodebuild).`
  and exits 0 so that `make ci` stays green on Linux CI. Report the `SKIP`,
  never claim a build.
- **The native build does not sign.** `scripts/native-gates.sh` builds with
  `CODE_SIGNING_ALLOWED=NO`. The native tier proves the code compiles; it
  proves nothing about distribution.

## 4 · The three levers

An administrator has three independent controls. They are not
interchangeable, and the order of precedence is not a matter of taste.

| Lever | Who owns it | What it reaches | Latency |
| --- | --- | --- | --- |
| **Managed configuration** (MDM) | The enterprise (an MDM server) | A device or a device group, on the device, without a server | Minutes |
| **Remote configuration** | The backend | Every installation, signed and validated | Minutes to hours |
| **App Store Connect** | The publisher | The distribution: the listing, the privacy labels, the build, the rollout | Days |

The precedence is fixed:

| Order | Source | Why |
| --- | --- | --- |
| 1 | The managed configuration | An enterprise's policy must not be overridable by a server the enterprise does not control |
| 2 | The remote configuration | The publisher's policy for everyone else |
| 3 | The bundled defaults | The app must work before the first fetch, and with no network at all |

A configuration value is **never** a permission. A managed configuration can
point the app at a different backend, but the **server authorizes every
request**; the URL never authorizes.

## 5 · Managed configuration

### The delivery channel

On iOS the managed configuration is delivered by a **Mobile Device Management
(MDM) server** that pushes an *app configuration* to the device. The
enterprise owns the devices and the console; the platform applies the
configuration to the app.

| Channel | Who can use it | How the value arrives |
| --- | --- | --- |
| **MDM server, managed app configuration** | An enterprise that owns the devices | The MDM server sets the payload; the platform delivers it to the device and stores it for the app to read |
| **Apple Business Manager / Apple School Manager, managed apps** | An enterprise that buys apps through the volume purchase | The managed app record carries the same configuration payload |

### The reader

The app reads its own managed configuration from the standard user defaults
domain the platform reserves for it:

| API | What it gives |
| --- | --- |
| `UserDefaults.standard.dictionary(forKey: "com.apple.configuration.managed")` | The payload the MDM server applied to this application. The standard read, and the one the template's reader should use |
| `UserDefaults.standard.object(forKey:)` on a known key | One value, when the payload is flat and the app knows the key |
| `ManagedSettings` / `com.apple.developer.managed-configuration` entitlement | Only for apps that need the read to be **protected** from the user; not the template's case |

The read is **cheap and local** — it does not touch the network — so it
belongs on the startup path, before the first screen renders.

### The keys

The managed configuration is a **closed set**. An unknown key fails the whole
parse; the app runs with the bundled defaults, not with a half-understood
policy. **The template ships no reader and no key set** — the table below is
the shape a project adopts, not shipped code.

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `TenantUrl` | string | empty | The base URL of the backend the app talks to |
| `TelemetryLevel` | `off` \| `errors` \| `full` | `errors` | How much telemetry may leave the device |
| `KillSwitch` | boolean | `false` | The app refuses to start |
| `MinVersion` | string | empty | The lowest `MARKETING_VERSION` the app accepts; below it, the update screen |
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
| The configuration value type | `Kernel` — a pure Swift value, so it is testable without a device |
| The reader | A new platform package (`Packages/Core/Platform`), the only place that touches `UserDefaults` and the MDM key |
| The merge by precedence | `AppEnvironment` — the composition root resolves managed → remote → bundled ([ADR 0003](adr/0003-explicit-app-environment.md)) |
| The diagnostics view | The diagnostics screen in the app target ([Telemetry](11-telemetry.md)) |

The reader returns a **document**; the composition root decides what to do
with it. The platform layer never decides policy.

## 6 · Remote configuration

The remote configuration is the publisher's lever over every installation
that is not under an enterprise's control. It is the same shape as the
Android kit's remote configuration, re-homed on the Apple stack: a signed,
versioned, data-only document fetched by the backend.

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
| `MinVersion` | The lowest accepted `MARKETING_VERSION` |
| `FeatureFlags` | Per-capability switches |
| `OnboardingContent` | The skippable screens and their copy |
| `QualityDefaults` | The default quality profile, per tier |
| `PushCategories` | The notification categories the app may use |
| `KillSwitches` | Per-capability kill switches |
| `TelemetryLevel` | The amount of telemetry the app may send |

### The remote-configuration gap, and what it takes to close it

| Missing | Where it belongs |
| --- | --- |
| The document schema | `Kernel` or the slice's `Contracts` package — a pure Swift schema |
| The signature verification | A new security package, backed by the Keychain ([Security & Keychain](09-security-and-keychain.md)) |
| The fetch | A `URLSession` call behind a domain port, or a `BGAppRefreshTask` job ([Offline, Outbox & Sync](08-offline-outbox-and-sync.md)) |
| The cache | The local store, with the TTL |
| The kill switch check | The startup path, before the first screen |

A remote configuration is **not** fetched on the main thread and **not**
fetched on every launch: it is a deferred job, and the cached copy is read at
startup.

## 7 · App Store Connect setup

The tasks below are the publisher's checklist. **The template implements none
of them**; they are the console-side work a project must do once.

| Task | Where in App Store Connect | Why it matters |
| --- | --- | --- |
| Create the app record and register the bundle id | My Apps → + → New App; Identifiers in the developer portal | The identifier must match `PRODUCT_BUNDLE_IDENTIFIER` in `project.yml` exactly |
| The store listing | App Store → App Information | The name, the subtitle, the description, the icon, the screenshots |
| The privacy policy URL | App Store → App Privacy | Required by the store; it is a real document, not a placeholder |
| The App Privacy answers (the privacy labels) | App Store → App Privacy | One questionnaire. The answers must match what the app actually does and the privacy manifest; a mismatch is a rejection and, worse, a false claim |
| The age rating | App Store → Age Rating | Required before any release |
| App review information | App Store → App Review Information | The instructions a reviewer needs, including a test account |
| The build | TestFlight → Builds | The uploaded build; the version and build numbers are the ones in `project.yml` |
| The TestFlight groups | TestFlight | Internal (up to 100 testers, fast review) and external (pilot groups, beta review) |
| The production release | App Store → + → New Version | The metadata, the build, the release type |
| The phased release | Production → Phased Release | The rollout percentage and the pause control |
| The App Store Connect API key | Users and Access → Keys | The CI credential for uploading and for the automated gates |

### The usage-description declaration

The template declares **no usage-description key** and **no entitlement**:
`project.yml` sets `GENERATE_INFOPLIST_FILE: YES` and three generated keys
(`CFBundleDisplayName`, `UILaunchScreen_Generation`,
`UIApplicationSceneManifest_Generation`) — none of which asks the user for
anything. A consumer that touches the camera, the microphone, the photo
library, the location or the contacts must add the matching
`INFOPLIST_KEY_NS…UsageDescription` to `project.yml` and declare the
matching capability.

| Check | How |
| --- | --- |
| The merged `Info.plist` of the built app | `plutil -p DerivedData/.../<App>.app/Info.plist` after a build |
| Every usage description against the register | The permissions register in the security chapter ([09](09-security-and-keychain.md)) |
| Every usage description against the App Privacy answers | A prompt that collects data must be declared in the questionnaire |
| Every capability against the entitlements file | `codesign -d --entitlements :- <App>.app` on the built product |

A usage description that is not in the register does not ship.

## 8 · Signing

### The two identities

| Identity | Who holds it | Where it lives | Can it be rotated |
| --- | --- | --- | --- |
| **The distribution certificate** (the identity the build is signed with) | The publisher | The developer's keychain, and the CI secret store for a release | Yes: revoke it in the developer portal and reissue |
| **The provisioning profile** | The publisher | Downloaded from the developer portal or the CI secret store; it binds the certificate, the bundle id and the devices | Yes: the profile is a generated artefact and is re-downloaded |

The **Apple Developer Program membership** owns both. On iOS there is no
"upload key held by the store" split of the kind the Android kit has: the
binary is signed locally with the distribution certificate, and the profile
decides which capabilities the binary may use.

### Automatic signing

The template declares **no signing settings at all** — no `DEVELOPMENT_TEAM`,
no `CODE_SIGN_STYLE`, no profile, no entitlements file. A consumer therefore
adds them to `project.yml`, under the `<APP>` target:

| Setting | Value | Why |
| --- | --- | --- |
| `DEVELOPMENT_TEAM` | The team's identifier | Without it, `xcodebuild` fails with "requires a development team" |
| `CODE_SIGN_STYLE` | `Automatic` | Xcode manages the certificate and the profile for the selected team |
| `CODE_SIGN_ENTITLEMENTS` | The path to the entitlements file — **only if** the app needs a capability | A file the consumer creates and commits; the project stays generated, the capability files do not ([ADR 0001](adr/0001-xcodegen-generated-project.md)) |

With automatic signing, the developer account in Xcode holds the
certificates; CI uses an App Store Connect API key or an imported
certificate, both taken from the CI secret store.

### The rules

| Rule | Why |
| --- | --- |
| The release job is **isolated** and requires an approval | A release is not a side effect of a merge |
| The signing material comes from **the environment or the CI secret store**, never from a file in the tree | A certificate in the repository is a compromised identity |
| The password is never printed, never echoed, never in the log | The log is a copy that outlives the job |
| The **distribution certificate** never leaves the CI secret store | A certificate that exists in two places exists in the wrong one |
| The entitlements file is **reviewed like code** | An entitlement is a capability grant |
| A certificate revocation is a planned release, not an incident response | The reissue flow has its own steps |

### The archive

The template's own build never signs and never archives
(`CODE_SIGNING_ALLOWED=NO` in `scripts/native-gates.sh`). A consumer's
release archive is a separate step:

```sh
xcodebuild \
  -project <APP>.xcodeproj \
  -scheme <APP> \
  -configuration Release \
  -destination 'generic/platform=iOS' \
  -archivePath build/<APP>.xcarchive \
  archive
```

The export step needs an `ExportOptions.plist` that **does not exist in the
template**: the consumer writes it (method, team id, the signing style) and
keeps it in the tree next to `project.yml`, so the export is reproducible and
reviewable.

## 9 · The privacy manifest

`PrivacyInfo.xcprivacy` is **not in the tree**. It is a resource the consumer
adds, and it is a release gate, not a formality.

| Item | Who provides it | Note |
| --- | --- | --- |
| The app's own `PrivacyInfo.xcprivacy` | The consumer | Declares the tracking domains and the **required-reason API** categories the app itself uses |
| The privacy manifest of every third-party SDK | The SDK's vendor | Every binary SDK must ship one; the template ships **zero third-party Swift packages**, so today there is nothing to collect |
| The App Privacy answers | The consumer, in App Store Connect | They must agree with the manifest and with the app's actual behaviour |

Because the template declares no entitlement, no usage description and no
third-party package, its privacy surface is empty — which is exactly the
state a consumer must re-derive once it adds a capability.

## 10 · Verifying a deployment

An administrator needs to answer four questions: is the policy applied, what
does the app think it is, what did the app actually do, and is the installed
build the one that was approved.

| Question | The check |
| --- | --- |
| Is the managed configuration applied? | The platform read succeeds and returns the expected keys; the diagnostics bundle shows the **effective** configuration, redacted |
| What does the app believe? | The effective configuration appears in the diagnostics bundle, with the source of each value named (managed, remote, bundled) |
| Did the app act on it? | The log line of the configuration load names the source and the document version |
| Is the installed build the approved one? | The version, the build number and the **signing identity** of the installed app match the release record |
| Is the release healthy? | The App Store Connect metrics for the release, against the thresholds that gate the promotion ([Testing & CI](12-testing-and-ci.md)) |

The last three rows are why the diagnostics bundle is specified
([Telemetry](11-telemetry.md)): an administrator who cannot see what the app
believes cannot debug a deployment.

### The build checks

| Step | Command | What it proves |
| --- | --- | --- |
| The generated project name and scheme | `ls -d *.xcodeproj` → the name comes from `name: <APP>` in `project.yml` | The scheme and the target are the ones the manifest declares |
| The toolchain that built it | `xcodebuild -version` | The Xcode and Swift version behind the binary |
| The version and build numbers | `xcodebuild -showBuildSettings -project <APP>.xcodeproj -scheme <APP>` | `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION` — `0.1.0` and `1` in the template |
| The bundle id | the same `showBuildSettings` output | `PRODUCT_BUNDLE_IDENTIFIER` matches the App Store Connect record |
| The built product's metadata | `plutil -p DerivedData/.../<App>.app/Info.plist` | `CFBundleShortVersionString` and `CFBundleVersion` as they were packaged |
| The signing identity | `codesign -dv --verbose=4 <App>.app` | The authority and the team identifier of the signature |
| The embedded capabilities | `codesign -d --entitlements :- <App>.app` | The entitlements actually shipped, which is the capability register's evidence |

On a **physical device**, the installed build is visible in Xcode's *Window →
Devices and Simulators* panel, and the device can be listed from the command
line with `xcrun devicectl` (the exact subcommand varies between Xcode
versions — verify it against your toolchain before scripting it).

### The simulator pool

The kit has **no device farm and no simulator pool**; it declares one build
destination, `generic/platform=iOS Simulator`, in `scripts/native-gates.sh`.
A project that needs a pool installs the runtimes it wants and drives them
with `xcrun simctl`:

| Command | What it does |
| --- | --- |
| `xcrun simctl list runtimes` | The installed simulator runtimes |
| `xcrun simctl list devices` | The available simulators and their state |
| `xcrun simctl push <device> <bundle-id> <payload.json>` | Injects a push payload on the simulator, the fastest way to exercise a push tap before APNs exists |

The device tiers the pool must cover are in
[Support Matrix](support-matrix.md).

## 11 · The rollout and the kill switch

| Stage | Where | The gate |
| --- | --- | --- |
| Internal TestFlight | App Store Connect → TestFlight → Internal | The pre-launch report, the smoke test on the simulator pool |
| External TestFlight | App Store Connect → TestFlight → External | The metrics thresholds, the beta review |
| Production | App Store Connect → App Store | The phased release starts at a small percentage |
| The phased release | Production → Phased Release | The percentage increases only while the metrics stay under the thresholds |
| The pause | The phased-release controls | The rollout stops; the installed builds keep working |
| The kill switch | The managed configuration, or the remote configuration | The app refuses to start or disables the capability |

A phased release is the last reversible step; after it, the fix is a new
release. That is why the rollout percentage starts small and why the pause
control must be reachable by someone who is not on call for the app.

## 12 · The admin surface the template does not have

| Missing | Chapter that specifies it |
| --- | --- |
| The managed configuration reader | [§5](#5--managed-configuration) above, [Auth & Platform Features](07-auth-and-platform-features.md) |
| The remote configuration client | [§6](#6--remote-configuration) above |
| The diagnostics screen and the bundle | [Telemetry](11-telemetry.md) |
| The signing configuration | [§8](#8--signing) above, [Security & Keychain](09-security-and-keychain.md) |
| The privacy manifest and the App Privacy answers | [§9](#9--the-privacy-manifest) above |
| The APNs pipeline and the token registration | [Push Notifications](10-push-notifications.md) |
| The release pipeline | [Playbooks](13-playbooks.md) |
| The permissions and capability register | [Security & Keychain](09-security-and-keychain.md) |
| The version / build metadata | [Telemetry](11-telemetry.md) |

## 13 · Security rules

| Rule | Why |
| --- | --- |
| A configuration value never carries a secret | A configuration document is readable by whoever reads the document |
| A malformed configuration never becomes a partial configuration | The defaults are used as a whole |
| The `KillSwitch` needs no network | It must work on a device that cannot reach the network |
| The managed configuration wins over the remote one | An enterprise's policy is not overridable by a server |
| A policy value is never logged verbatim | Some values are sensitive; the effective configuration is redacted in the bundle |
| The signing material never leaves the CI secret store | An identity that exists in two places exists in the wrong one |
| The configuration never authorizes | The server authorizes every request |
| A capability needs a register entry, a justification, a test and a security review | An entitlement is a grant that outlives the change that added it |

## See also

- [Security & Keychain](09-security-and-keychain.md) — the Keychain, the
  entitlement review, the App Store policy checks.
- [Telemetry](11-telemetry.md) — the diagnostics bundle that proves what the
  app believes.
- [Playbooks](13-playbooks.md) — the release recipe.
- [Support Matrix](support-matrix.md) — the devices the deployment is
  verified on.
- [ADR 0001 — XcodeGen-generated project](adr/0001-xcodegen-generated-project.md)
  — why the project file is generated and never committed.
