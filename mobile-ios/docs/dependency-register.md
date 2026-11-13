# Dependency Register

Every dependency the template declares, with the version, the reason it is
there, and the path that removes it. **A dependency that is not in this
register does not belong in a `Package.swift`, in `project.yml` or in the
`evals/` package.**

The register is the evidence for the least-dependency posture. A dependency
is a permanent maintenance obligation: it needs version bumps, it needs a
security watch, and it needs a removal plan.

The headline: **the runtime dependency count is zero.** The app links nothing
that the operating system does not already provide. Everything below is
either the platform itself or a tool that runs on a build machine.

## 1 · The platform and the toolchain

| Dependency | Version | Why it is there | Removal path |
| --- | --- | --- | --- |
| **Swift** | `6.0` language mode (`SWIFT_VERSION: "6.0"`, `swift-tools-version: 6.0`, `swiftLanguageModes: [.v6]` in all six manifests) | The language. Swift 6 mode with complete strict concurrency makes a data race a compile error, which is what lets the domain be `Sendable` and the models be `@MainActor`. | Not removable. The mode is a *floor*, not a ceiling: it only moves up. |
| **SwiftUI** | The iOS 18 SDK | The whole UI layer: `NavigationStack`, `navigationDestination`, `ContentUnavailableView`, the `.task` modifier that fires the first refresh. | The app would have to be a UIKit application — the `NotesUI` package and every screen in it. |
| **Observation** | The iOS 18 SDK | `@Observable`, the state mechanism behind `NotesModel` and `Router`. It is a separate module from SwiftUI, and a pure package can use it without a view framework. | Replace `@Observable` with `ObservableObject` and `@Published` — the Combine-era approach, with more boilerplate and no observation-tracking benefit. |
| **Swift Testing** | The iOS 18 SDK | The test framework: `@Suite`, `@Test`, `#expect`. The three shipped suites (`KernelTests`, `NotesDomainTests`, `NotesUITests`) all use it. | Migrate to XCTest — the tests keep their structure, but the assertions lose their built-in diffing. |
| **Xcode** | 16 or newer | `swift test` per package and `xcodebuild` for the app target. The toolchain is the compiler and the SDK. | Not removable: it *is* the build. |
| **XcodeGen** | **Not pinned in the tree** — `brew install xcodegen` | The only external build tool. It turns `project.yml` into the `.xcodeproj` ([ADR 0001](adr/0001-xcodegen-generated-project.md)). | Commit the generated `.xcodeproj` and drop the manifest — the conflict and churn that ADR 0001 refuses come back. Or switch to another generator (Tuist) and change `scripts/native-gates.sh` and the `make generate` target. |
| **Node.js** | 24 (documented in `README.md`; `evals/package.json` declares no `engines` field) | The `.claude` contract evals only: `node --test evals/contracts.test.mjs`. The app never runs Node, and the evals have zero npm dependencies. | Port the contract evals to `swift test` or to the native tier — they are pure text checks over the `.claude` tree. The `.claude` runtime itself is still a separate concern. |

`scripts/native-gates.sh` treats a missing `xcodegen`, `swift` or
`xcodebuild` as a **visible `SKIP`**, never as a silent pass — the absence
of a tool is reported, not hidden.

## 2 · The SwiftPM packages

These are **not** dependencies in the third-party sense: all six are local
path packages inside this repository, and every dependency edge between them
is a `.package(path:)`. They are listed here because each one is a permanent
obligation.

| Package | Kind | Depends on | Removal path |
| --- | --- | --- | --- |
| `Kernel` | Pure Swift | the standard library | Not removable. `Outcome`, `AppError`, `ErrorCode` and `NoteID` are the vocabulary every layer speaks. |
| `UICore` | Pure Swift | `Kernel` | Merge into `Kernel` — but the split is what keeps `UiStatus` out of the domain's dependency set. |
| `NotesContracts` | Pure Swift | the standard library | Delete the package with the slice; it is the slice's public door. |
| `NotesDomain` | Pure Swift | `Kernel` | The slice's core; it goes when the slice goes. |
| `NotesData` | Adapter | `NotesDomain`, `Kernel` | Replaced, not removed, when a real adapter arrives — the port and the use case stay unchanged. |
| `NotesUI` | SwiftUI | `NotesDomain`, `NotesContracts`, `Kernel`, `UICore` | Replaced with the real feature's UI; the package boundaries stay. |

The graph is the architecture: a pure package cannot import SwiftUI or UIKit
because the framework is not in its manifest, so the layering is a **compile
error**, not a review comment ([ADR 0002](adr/0002-one-swiftpm-package-per-layer.md)).

## 3 · The runtime dependencies

| Runtime dependency | Count | Why |
| --- | --- | --- |
| Third-party Swift packages (SwiftPM, remote) | **0** | Every `dependencies:` array in the six manifests holds only `.package(path:)` entries. There is no `Package.resolved` to commit — `.gitignore` excludes it. |
| Binary frameworks / XCFrameworks | **0** | The tree contains no `.framework`, no `.xcframework` and no vendored binary. |
| CocoaPods / Carthage | **0** | Neither is installed, referenced or configured. |
| Network clients (Alamofire, Moya, …) | **0** | The app talks to no server. The `notes` slice reads two fixed notes from memory. |
| Analytics / crash reporters | **0** | No logger, no ring buffer, no remote sink, no crash reporter ships ([Telemetry](11-telemetry.md)). |
| Push SDKs | **0** | No APNs wrapper, no Firebase, no notification extension. APNs is reached through the system framework when a project adds it. |
| DI frameworks | **0** | `AppEnvironment` builds the graph by hand ([ADR 0003](adr/0003-explicit-app-environment.md)). |
| Persistence libraries | **0** | `InMemoryNoteRepository` writes nothing. |
| Keychain wrappers | **0** | There is no secret to store. |

**The runtime dependency count is zero.** The shipped binary links only
system frameworks. That is the floor the template defends: every row above
that becomes non-zero is an adoption decision a project makes on purpose,
with a register row in the same change.

## 4 · The platform capabilities

A capability is a dependency on the operating system's trust: a key that
grants the app access to a protected resource, or a hardware/system service.
It is registered here for the same reason a library is.

| Capability | State | Notes |
| --- | --- | --- |
| Entitlements | **Zero.** No `.entitlements` file exists in the tree, and `project.yml` sets no `CODE_SIGN_ENTITLEMENTS`. | The app has no associated domains, no push environment, no keychain sharing, no app groups. |
| `Info.plist` usage-description keys | **Zero.** No `INFOPLIST_KEY_NS…UsageDescription` is declared. | The app asks the user for nothing: no camera, no microphone, no photo library, no location, no contacts, no tracking. |
| Background modes | **Zero.** No `UIBackgroundModes` key. | No background fetch, no remote notification, no location updates. |
| `Info.plist` keys that *do* exist | Three, all generated and none of them a usage request: `INFOPLIST_KEY_CFBundleDisplayName` (the on-device name), `INFOPLIST_KEY_UILaunchScreen_Generation` and `INFOPLIST_KEY_UIApplicationSceneManifest_Generation` (both `YES`). | `GENERATE_INFOPLIST_FILE: YES` means the `Info.plist` is produced from build settings, not from a committed file. |
| The privacy manifest | **Not present.** There is no `PrivacyInfo.xcprivacy` in the tree. | It is a consumer task, and it has no third-party SDKs to collect manifests from, because the runtime dependency count is zero. |
| Network access | **None.** No App Transport Security exception is declared. | The app makes no request; ATS therefore applies in its strictest form by default. |

Adding any capability is an **adoption step**: it needs a register row here,
a justification, a test and a security review
([Security & Keychain](09-security-and-keychain.md)).

## 5 · What is deliberately **not** in the template

The absences are as deliberate as the presences. Each of these is an
**adoption step** with a chapter, not an oversight.

| Not present | Why it is not there yet | The adoption path |
| --- | --- | --- |
| **A DI framework** (Factory, Swinject, Resolver) | One app target, six packages and a graph of a handful of objects. A DI framework is a runtime dependency and a registration file per layer; manual wiring is smaller and compile-time checked. See [ADR 0003](adr/0003-explicit-app-environment.md). | When the graph exceeds what a human can hold, or when the team wants to test injection graphs, adopt a container **in the app target only**. |
| **SwiftData** (or GRDB) | The `notes` repository is an in-memory adapter that returns two fixed notes. Persistence is a decision the template cannot make for you. | [08 — Offline, Outbox & Sync](08-offline-outbox-and-sync.md) |
| **A network client** | The app talks to no server; the `LoadNotes` use case calls a port. | [07 — Auth & Platform Features](07-auth-and-platform-features.md) |
| **A Keychain wrapper** | There is no secret to encrypt. | [09 — Security & Keychain](09-security-and-keychain.md) |
| **An APNs client** | There is no push pipeline; the system framework is enough when the time comes. | [10 — Push Notifications](10-push-notifications.md) |
| **A logger** | There is no telemetry sink, and a logger that logs nothing is still a dependency. | [11 — Telemetry](11-telemetry.md) |
| **A crash reporter** | The app has no crash reporting policy yet, and every SDK that ships one is a privacy-manifest obligation. | [11 — Telemetry](11-telemetry.md) |
| **A mocking framework** | The fakes are hand-written and small — a `StubRepository` of five lines. | When a port has more than three implementations in a test. |
| **A localization catalog** | UI strings are Swift string literals in the view. | [03 — Packages, Layers & UDF](03-packages-layers-and-udf.md) §7 |

## 6 · The rules

| Rule | Why |
| --- | --- |
| Every dependency gets a **register row in the same change** | A dependency without a removal path is a dependency without an owner |
| A dependency that is not used is **removed**, not kept "for later" | An unused dependency is still an attack surface and still a license to track |
| **Local path packages are not third-party dependencies** | The distinction is what keeps the runtime count at zero |
| Every dependency is checked for its **license** and its **maintenance health** | A dependency with one maintainer is a risk |
| A binary artefact (`.xcframework`, `.a`) needs a review | A binary is opaque, unsigned by default and carries its own privacy manifest obligation |
| No dependency is added for something the platform already does | The platform is the cheapest dependency and the one with the smallest attack surface |
| The `evals/` package keeps **zero npm dependencies** | The contract evals must run on any machine with Node, with no install step |

## 7 · The review process

A dependency change is a review, in this order:

1. **Read the register.** What is the dependency for, and what would it cost
   to remove?
2. **Add the edge** — a `.package(path:)` or a `.package(url:)` entry in the
   package that needs it, or the tool in the section that owns it.
3. **Update this register** in the same change: the version and the date.
4. **Run the whole gate set** (`make ci`) — a bump that breaks the build is
   only visible at compile time.
5. **Check the side effects**: a new SDK may request a **capability** or
   **collect data**. If it does, the capability table in §4, the permissions
   register and the App Store privacy answers change too
   ([09](09-security-and-keychain.md)).
6. **Check the removal path**: if the dependency became unnecessary, remove
   it in this change and record that in the register.

## 8 · The removal path, exercised

Every dependency has a way out, and the way out is exercised the same way:

| Step | What it proves |
| --- | --- |
| Remove the dependency edge from the manifest | The package is no longer resolved |
| Remove the import and the call sites | No dead code keeps the dependency alive |
| Run the gate set | Nothing referenced the removed API |
| Update this register | The evidence matches the build |

For a **tool** (XcodeGen), the removal path is different: replace the
generator in `scripts/native-gates.sh` and in the `make generate` target, or
commit the project file and accept the churn that
[ADR 0001](adr/0001-xcodegen-generated-project.md) refuses.

## See also

- [Project Layout & Build Logic](05-project-layout-and-build-logic.md) — the
  manifests and the generated project.
- [Security & Keychain](09-security-and-keychain.md) — the capability and
  privacy side effects.
- [Playbooks](13-playbooks.md) — the recipe for changing a dependency.
- [ADR 0002 — One SwiftPM package per layer](adr/0002-one-swiftpm-package-per-layer.md)
  — why the package list is the architectural contract.
