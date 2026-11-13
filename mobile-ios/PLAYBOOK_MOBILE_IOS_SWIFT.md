# PLAYBOOK_MOBILE_IOS_SWIFT.md — Native iOS playbook (Swift 6.x · SwiftUI in code · Game · Market · Avatar)

Architecture and engineering contract for building the **same three-variant
product family** as `PLAYBOOK_FRONTEND_MOBILE.md` and
`PLAYBOOK_MOBILE_ANDROID_KOTLIN.md`, as a **native iOS app in Swift**, with one
hard rule about how we build UI: **every view is written as code**. No
Storyboards, no XIBs, no Interface Builder, no visual workflow editors as a
source of truth. We develop with Claude Code; code is diffable, reviewable,
testable and generatable, visual canvases are not.

| Variant | What it is | Core capabilities |
|---|---|---|
| **Game** | Location-based 3D game in the style of Pokémon GO | Real-world map, spawns and encounters, 3D creatures, optional AR (ARKit), catching, collection, events, social |
| **Market** | Marketplace in the style of Amazon | Catalog, search and filters, product pages, reviews, cart, checkout (Apple Pay), orders and tracking, wishlist, seller tooling |
| **Avatar** | 3D avatar with voice and movement | 3D character, speech in/out, lip-sync, gestures/emotions, optional face-tracking (TrueDepth), live conversation |

**All three** share **onboarding, login, registration, forgot/reset password,
push notifications (APNs)**, the same Swift SDK, design system, security model
and release pipeline, and all three must run well on **older and mid-range
iPhones** (A12/A13-class), not only the newest models.

It extends and must stay consistent with `PLAYBOOK_FRONTEND_MOBILE.md` (variants,
backend contracts, device tiers, flows), `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` (parity
of slices, ports and budgets), `PLAYBOOK_FRONTEND.md` (MVVM + SDK),
`PLAYBOOK_URLS.md`, `PLAYBOOK_BACKEND.md`, `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_TESTS.md`, `PLAYBOOK_AUDIT.md`, `PLAYBOOK_CLAUDE_CODE.md`, and reuses patterns
from `PLAYBOOK_DESKTOP_QT.md` (realtime state machine) and
`PLAYBOOK_CHROME_EXTENSION.md` (managed config, kill switch, diagnostics).

The rules that carry the whole file:

1. **Views are code.** SwiftUI first (UIKit only in code, never via Interface
   Builder). Project and build configuration are text manifests, not GUI state.
2. **Swift 6 language mode with strict concurrency.** Data races are compile
   errors; isolation is explicit; the main actor owns UI.
3. **Modules enforce architecture.** Swift packages with `Domain` targets that
   import no UI frameworks; a target that doesn't depend on another can't import it.
4. **Unidirectional data flow.** `@Observable` ViewModels expose state; views send
   intents; side effects go through ports.
5. **One variant = one app target = one binary.** The market app doesn't ship a
   3D engine; entitlements and usage strings are scoped per variant.
6. **Design for the floor, enhance upward** with capability tiers and a runtime
   quality governor, using Metal GPU families and memory APIs, not device lists.
7. **The server is authoritative** for game state, prices, payments and
   entitlements; the app is untrusted.
8. **Push is a channel, not a data path.** Minimal payloads, fetch after tap.
9. **No vendor lock-in for irreplaceable assets** (avatars, models, user data).
10. **Measure on real older iPhones** and gate releases on MetricKit-style metrics.

> Verification note: status as of November 2026. **Xcode 27.2** (29 Oct 2026) with
> **Swift 6.4(.2)** is current; it requires a **Mac with Apple silicon on macOS 26.6+**.
> The App Store submission floor has been **Xcode 26 and the iOS 26 SDK since
> 28 April 2026**; plan the move to the iOS 27 SDK when Apple announces its
> deadline. **iOS 27** shipped in September 2026. Items marked **(verify)** depend
> on exact versions, App Store policy dates or library maintenance and must be
> re-checked in Apple's documentation, App Store Review Guidelines and release
> notes before adoption. Budgets are starting points to calibrate on your own
> device pool.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Language | **Swift 6 language mode** on current toolchain (Swift 6.4 in Xcode 27.x; 6.3 in Xcode 26.4+), complete strict concurrency | Data-race safety at compile time; migrate packages individually |
| D2 | UI | **SwiftUI, written in code**; UIKit only when measured necessary (code-only: `UICollectionView` compositional layout, `UIViewRepresentable` for engine surfaces) | Declarative, previews as code (`#Preview`), Claude Code can read/write every view |
| D3 | No visual builders | **Forbidden as source of truth:** Storyboards, XIBs, Interface Builder, Reality Composer Pro scenes, Core Data model editor, Xcode Cloud GUI workflows, visual Shortcuts. CI blocks adding these files | Reviewability, merge-ability, AI-assisted development |
| D4 | Project definition | **SwiftPM local packages + a generated Xcode project** from a text manifest (**Tuist** `Project.swift` or **XcodeGen** `project.yml`); `.xcodeproj` is a build artifact (git-ignored) with settings in **`.xcconfig`** files | Hand-edited `.pbxproj` is merge-hostile and agent-hostile |
| D5 | Architecture | **Hexagonal + vertical slices + UDF (MVVM)**, enforced by package/target dependencies | Same slices as backend, RN and Android playbooks |
| D6 | State | **Observation** (`@Observable`) ViewModels, `@MainActor` isolation for UI; async/await + `AsyncSequence`; actors for shared mutable state | Fine-grained view invalidation is cheaper on old CPUs than `ObservableObject` |
| D7 | Navigation | `NavigationStack` with typed route values + path state owned by a router; routes mirror `PLAYBOOK_URLS.md` | Deep links and restoration |
| D8 | DI | **Constructor injection + composition root** (`AppEnvironment`); optional `swift-dependencies` if the team adopts it | No heavy frameworks, no reflection |
| D9 | Networking | **URLSession** (HTTP/2, HTTP/3 where available) under a Swift SDK generated by **swift-openapi-generator** from the backend OpenAPI spec; `URLSessionWebSocketTask`; SSE over `URLSession.bytes` | First-party, no third-party transport weight |
| D10 | Persistence | **GRDB** (SQLite, WAL, code-defined migrations) for offline cache/outbox; `UserDefaults`/`@AppStorage` for non-sensitive prefs; Keychain for secrets | SwiftData is optional for simple local state; the Core Data model editor is excluded by D3 **(verify SwiftData maturity for your needs)** |
| D11 | Images | **Nuke/NukeUI** with downsampling processors (SwiftUI `AsyncImage` doesn't cache) | Memory-safe |
| D12 | Push | **APNs (token-based `.p8` auth)** sent directly by the backend; `UNUserNotificationCenter`; notification service extension (code-only) | Section 8 |
| D13 | Auth | First-party email/password + **Sign in with Apple** + **passkeys** (AuthenticationServices) + OIDC/PKCE via `ASWebAuthenticationSession`; tokens in Keychain | `PLAYBOOK_SECURITY.md` §4 |
| D14 | Payments | **Apple Pay (PassKit) + Stripe iOS SDK PaymentSheet** for physical goods/services; **StoreKit 2** for digital goods | Store rules differ by good type **(verify)** |
| D15 | 3D | Tiered behind a `SceneEngine` port: **SpriteKit/2D fallback → RealityKit (ECS, `RealityView`) → Metal custom renderer or embedded Godot → Unity (AR-heavy, high tier only)**. **SceneKit is soft-deprecated since iOS 26; don't start new work on it** | Section 13 |
| D16 | AR | **ARKit + RealityKit**, optional; geo-anchoring only where supported **(verify city coverage)** | Capability-gated |
| D17 | Maps (Game) | **MapLibre Native / Mapbox Maps** (GL layers, same tiles as Android); **MapKit for SwiftUI** for simple maps (store locator) | Entities as GL layers |
| D18 | Deployment target | **iOS 18.0** for all variants (covers iPhone XS/XR and later; RealityView on iOS, modern SwiftUI/Observation). Lower to **iOS 17** only with fleet data | Verify against your analytics each release |
| D19 | Linking | **Static linking** for SwiftPM libraries (default), few dynamic frameworks; mergeable libraries where useful | Fewer dyld loads at launch |
| D20 | Observability | **MetricKit + os.Logger/signposts + Sentry Cocoa (or Crashlytics)** | Section 16 |
| D21 | CI/CD | **Code-defined CI** (GitHub Actions or equivalent, Apple-silicon runners with pinned Xcode) + fastlane or scripts + App Store Connect API; **no Xcode Cloud GUI workflows** | Reproducible and reviewable |
| D22 | Testing | **Swift Testing** for unit/domain/ViewModel, **XCTest/XCUITest** for UI/perf, snapshot tests | Section 17 |
| D23 | Localization | **String Catalogs** (`.xcstrings`, plain JSON) with typed keys via `String(localized:)` | Editable as text; Xcode's editor optional |
| D24 | Assets | Vector (PDF/SVG) + asset catalogs for images/colors with **generated Swift symbols**; design tokens defined **in code** | Colors/fonts come from tokens, not from the catalog editor |

### 1.1 What "no visual workflows" means in practice

* **Allowed:** Xcode as an editor/debugger/profiler; `#Preview` blocks **in the same Swift file** as the view; Instruments; the asset-catalog and
  String Catalog editors as convenience over their JSON.
* **Forbidden as source of truth:** `.storyboard`, `.xib`, SwiftUI-canvas-only edits that are not in Swift source, `.rcproject`/Reality Composer Pro scenes
  (author 3D content in DCC tools → export USDZ/glTF; build entities **in code**), `.xcdatamodeld`, Xcode Cloud workflows configured in the GUI,
  GUI-only signing/capability toggles (declare entitlements, Info keys and capabilities in the manifest/`.xcconfig`/`.entitlements`).
* **CI guard:** fail the build if a PR adds forbidden file types or edits `project.pbxproj` by hand (the project is generated).
* **Claude Code loop:** Xcode 26.3+ integrates Claude Agent (the Claude Agent SDK harness), including Xcode Previews for visual checks, and exposes Xcode capabilities through MCP; for headless work
  use `swift build/test`, `xcodebuild`, `xcrun simctl` or an MCP bridge such as XcodeBuildMCP **(verify tool versions)**.

---

## 2. The product family

### 2.1 Variant matrix (same as other mobile playbooks; iOS specifics added)

| Capability | Game | Market | Avatar |
|---|---|---|---|
| Shared shell (auth, onboarding, push, settings, design system, SDK) | ✔ | ✔ | ✔ |
| Location | Core Location (live updates, `CLMonitor`), optional background | Optional approximate | No |
| Camera | Optional AR (ARKit) | Optional (`DataScannerViewController`, photos) | Optional (ARKit face tracking / Vision) |
| Microphone | Optional | Optional (voice search) | **Core** (`AVAudioEngine`, voice processing) |
| 3D rendering | **Core** (RealityKit/Metal) | Optional, off by default | **Core** (RealityKit) |
| Realtime | **Core** (WebSocket/SSE) | Order status, price alerts, Live Activities (optional) | **Core** |
| Payments | StoreKit 2 (digital) | **Core** (Apple Pay/Stripe) | StoreKit 2 (items/subscriptions) |
| Extensions | Notification service, Background Assets | Notification service, Widgets/Live Activities (optional) | Notification service, Background Assets |
| Target download (Low) | Base ≤ ~100 MB + Background Assets packs | ≤ ~35 MB | ≤ ~80 MB + packs |

### 2.2 Identical across all three

Login, registration, forgot/reset password, onboarding, push registration/routing, session/token handling, account deletion, consent, i18n,
accessibility, SDK, error taxonomy, correlation IDs, telemetry, kill switch/remote config, design tokens
(`PLAYBOOK_FRONTEND_MOBILE.md` §2.2, §7, §8).

### 2.3 May differ

Feature packages, native dependencies, entitlements, background modes, usage-description strings, App Store listings, theme, onboarding steps.

---

## 3. Device classes, budgets and the quality governor

### 3.1 Device classes (by silicon, not by model list)

| Class | Typical devices | Traits |
|---|---|---|
| **Low** | iPhone XS/XR (A12, iOS 18 ceiling), iPhone 11 / SE 2 / SE 3 (A13/A15), 3–4 GB RAM | Older GPU family, thermal throttling sooner, small RAM headroom |
| **Mid** | iPhone 12/13/14 (A14–A16), 4–6 GB RAM | Bulk of the active base |
| **High** | A17 Pro and newer, 8 GB RAM | Optional extras; Apple Intelligence-capable devices |

Reports after WWDC indicate iOS 27 runs on every iPhone that ran iOS 26 (back to iPhone 11 and SE 2/3), with a back-ported scheduler aimed at older hardware, while Apple Intelligence features require
newer devices **(verify the official compatibility list)**. Keep a **real-device pool** (≥ 2 Low incl. an A12 on iOS 18 and an A13, 2 Mid, 1 High) plus a cloud farm.

### 3.2 Budgets (Low-class, release build; starting points)

| Metric | Budget |
|---|---|
| Cold launch to first frame (TTID) | ≤ 1.5 s (Low), ≤ 1.0 s (Mid) |
| Time to usable content (cached) | ≤ 2.5 s (Low) |
| Hangs | No main-thread hangs > 250 ms in main flows; hang rate under App Store Connect/MetricKit "good" thresholds **(verify)** |
| Scroll | Smooth 60 fps lists (no sustained dropped frames in the scroll benchmark) |
| Touch response | ≤ 100 ms |
| Shell memory | ≤ ~150 MB footprint (no 3D) |
| 3D scene memory | Low ≤ ~600 MB footprint, Mid ≤ ~1 GB; keep a margin under `os_proc_available_memory()` and jetsam limits |
| 3D frame rate | Low: stable 30 fps (resolution scale ≤ 0.75); Mid: 30–60; High: 60/120 |
| Download size | Market ≤ ~35 MB; Game/Avatar base ≤ ~100 MB with Background Assets |
| Battery (Game active) | Tracked %/hour on the Low pool; no regression release over release |
| Memory growth over 30 min | ≈ 0 |
| Crash-free sessions | ≥ 99.5% (Low pool measured separately) |

### 3.3 Capability detection (native signals)

```text
Hint     ← ProcessInfo.physicalMemory, os_proc_available_memory(), Metal GPU family (MTLDevice.supportsFamily(.apple6…)),
           chip generation via model identifier → table maintained in code/remote config, OS version
Features ← ARWorldTrackingConfiguration.isSupported, ARFaceTrackingConfiguration.isSupported, DataScannerViewController.isSupported,
           SpeechTranscriber/SFSpeechRecognizer on-device support, display refresh (ProMotion)
Runtime  ← ProcessInfo.thermalState (+ notification), isLowPowerModeEnabled, memory warnings,
           frame timing (CADisplayLink / CAMetalDisplayLink), MetricKit history (previous hangs/jetsam)
Measured ← first-run micro-benchmark (reference scene p95 frame time), refreshed after OS/app updates
Remote   ← remote config keyed by model identifier / OS (blocklist or forced tier)
```

Rules: the hint is a **hint**; the **measured** result and the **runtime governor** win; persist the tier; re-evaluate after OS/app updates and repeated hitches.
Prefer **GPU family and feature checks** over model tables wherever possible.

### 3.4 Quality governor

Same `QualityProfile` knobs as `PLAYBOOK_FRONTEND_MOBILE.md` §3.4 (render scale, frame cap, shadows/post, texture pack, LOD, particles, map layers, tracking/AR). Native additions:

* **Thermal**: observe `ProcessInfo.thermalState`; step down at `.fair`/`.serious` before throttling hurts.
* **Low Power Mode**: force the Low profile, disable AR/camera tracking, cut frame cap to 30.
* **Memory**: respond to memory warnings immediately (drop caches, LOD down); use `os_proc_available_memory()` to cap scene budgets dynamically.
* **Frame pacing**: `CAMetalDisplayLink`/`preferredFrameRateRange` to avoid rendering above the budget; ProMotion only on High/Mid when thermally healthy.
* **UI visuals**: honor `accessibilityReduceMotion` and `accessibilityReduceTransparency`; provide a **Compatibility visual mode** that replaces translucent/glass effects with solid surfaces on the Low tier.
* Step **down** immediately on breach; step **up** only after a sustained healthy period (hysteresis). Expose Auto / Battery saver / High quality in settings; remote config/managed policy can force a profile.

### 3.5 Liquid Glass and visual cost

Adopt system components (they get the platform look for free). Custom translucent/“glass” effects, blurs and materials are GPU-expensive on older devices: use few, avoid stacking them behind scrolling lists, gate
custom effects by tier, and honor Reduce Transparency. Apple provided an Info.plist opt-out for the iOS 26 design during the transition (`UIDesignRequiresCompatibility`); do not rely on it as a long-term strategy **(verify its status in the iOS 27 SDK)**.

---

## 4. Architecture: hexagonal + vertical slices + UDF on iOS

### 4.1 Layers mapped to Swift packages and targets

```text
App target (per variant)   Heimdall Game | Heimdall Market | Heimdall Avatar     (composition root, @main App, navigation root)
  │
  ├── Features/<Variant>/<Slice>/   (one Swift package per slice)
  │     ├── <Slice>Contracts   public cross-slice API (protocols, events, route keys)
  │     ├── <Slice>Domain      entities, value objects, rules, PORTS, use cases     → PURE SWIFT (no UI or platform frameworks)
  │     ├── <Slice>Data        port implementations: SDK calls, GRDB, push, location, camera, audio, scene adapters   [adapters/out]
  │     └── <Slice>UI          SwiftUI views + @Observable ViewModels + Router entries                               [presentation adapters]
  ├── Shell/{Identity,Notifications,Devices,Consent,Settings}/…   (same four-target layout)
  └── Core/{Kernel,Contracts,SDK,DesignSystem,UICore,Device,Telemetry,Security,Scene*,Testing}
```

| Target type | Contains | May depend on | Must not depend on |
|---|---|---|---|
| `Domain` | Entities, value objects, ports, use cases, `Outcome`/errors | `Kernel` | SwiftUI, UIKit, URLSession, GRDB, ARKit/RealityKit, CoreLocation, anything platform-specific |
| `Data` | Adapters (SDK, GRDB, Keychain, push, location, camera, audio, scene engines) | own `Domain`, `Core/*`, system frameworks | `UI` targets |
| `UI` | Views, ViewModels, route entries, previews | own `Domain`, `DesignSystem`, `UICore` | `Data` (resolved through the composition root) |
| `Contracts` | Cross-slice protocols/events | `Kernel` | everything else |
| `SDK` | Generated OpenAPI client + thin hand-written wrapper | `Contracts`, `Kernel`, Foundation | UI, UIKit/SwiftUI |
| App target | Wiring only | all needed packages | business logic |

Rules:

* **`Domain` targets compile without Apple UI frameworks**, so their tests can run with `swift test` on macOS *and Linux CI* for speed.
* Use the **`package` access level** for intra-package visibility and `public` only for cross-package APIs; keep protocols/ports `public` only where another target implements them.
* **Cross-slice access only via `Contracts`**, like the backend.
* **Architecture tests**: a script validates `swift package show-dependencies --format json` against an allowlist; SwiftLint custom rules ban `import SwiftUI` in `Domain`/`Data`; CI fails on violations.
* **Static linking** by default; avoid many small dynamic frameworks; keep package granularity per slice (not per type) to protect build/launch time.

### 4.2 Concurrency model (Swift 6)

* **Language mode 6, complete strict concurrency** in every package. Domain/data packages: default nonisolated; **UI packages set default isolation to `MainActor`** (Swift 6.2+ `defaultIsolation`) so view/ViewModel code is main-actor by default.
* Adopt the Swift 6.2 "approachable concurrency" settings consistently (nonisolated-nonsending by default, explicit `@concurrent` for work that must hop off the caller's actor); verify the exact flag set against the Swift 6.4 release notes before standardizing.
* **Repositories and caches are actors** (or wrap GRDB's `DatabaseQueue/Pool`); DTOs are `Sendable` value types; no `@unchecked Sendable` without a documented reason.
* **Structured concurrency only**: `async let`, task groups, `.task {}` in views (cancelled with the view); no unmanaged `Task {}` in ViewModels without storing/cancelling the handle.
* **Typed throws / `Outcome<T, AppError>`**: errors are values across layers; always propagate `CancellationError` correctly.
* Low-end CPUs have 2–6 cores with 2 performance cores: cap concurrent work (`TaskGroup` with limits), avoid thousands of tiny tasks, use `AsyncStream` with buffering policies (`.bufferingNewest`) for high-rate sensor/realtime data.

### 4.3 Presentation: Observation + UDF

```swift
// <Slice>UI/LeadDetailViewModel.swift
@MainActor @Observable
final class LeadDetailViewModel {
    enum Status: Equatable { case loading, ready, empty, offline, notFound, policyBlocked
                             case forbidden(requiredPermission: String?), error(correlationID: String?) }
    private(set) var status: Status = .loading
    private(set) var lead: LeadUI?
    private(set) var canEdit = false
    private(set) var isMoving = false

    private let getLead: any GetLeadUseCase, moveStage: any MoveLeadStageUseCase, permissions: any PermissionsPort
    init(getLead: some GetLeadUseCase, moveStage: some MoveLeadStageUseCase, permissions: some PermissionsPort) { … }

    func onAppear(id: LeadID) async { … }                    // loads, maps Outcome → status
    func send(_ intent: Intent) { … }                         // .moveToStage(Stage), .retry — returns quickly, work in tracked tasks
}
```

* Views are **pure functions of state**: `struct LeadDetailView: View { @State var viewModel: LeadDetailViewModel … }`; no business logic or networking in views.
* Every screen renders the shared statuses (`loading | ready | empty | forbidden | not-found | offline | error | policy-blocked`), same as the other clients.
* ViewModels depend on use cases and ports (`NavigatorPort`, `NotifierPort`, `PermissionsPort`), never on `UIApplication`, `UIViewController` or concrete adapters.
* **SwiftUI performance rules (Low tier)**: small `body`s with extracted subviews; stable `id`s in `ForEach`; `List`/`LazyVStack` for long collections; avoid `AnyView` and deep `GeometryReader`
  nesting; read observable properties as late as possible (fine-grained tracking); no heavy work in `body`; precompute display models off the main actor; avoid per-frame `State` writes; use `Canvas`/`TimelineView`
  only with measured budgets; avoid `.blur`, large `.shadow`, stacked materials; profile with the SwiftUI and Time Profiler instruments.

### 4.4 Ports (examples)

```swift
public protocol LeadRepositoryPort: Sendable {
    func lead(id: LeadID) async -> Outcome<Lead>
    func list(_ query: LeadQuery) async -> Outcome<LeadPage>
    func updateStage(id: LeadID, to: Stage, expected: Version, key: IdempotencyKey) async -> Outcome<Lead>
}
public protocol PushPort: Sendable { func permission() async -> PermState; func requestPermission() async -> PermState
    func registerDevice() async -> Outcome<DeviceReg>; func unregisterDevice() async }
public protocol SecureStorePort: Sendable { func get(_ key: String) async throws -> Data?; func set(_ v: Data, for key: String) async throws; func remove(_ key: String) async throws }
public protocol LocationPort: Sendable { func updates(_ opts: LocOpts) -> AsyncStream<Fix>; func startBackground(_ o: BgLocOpts) async; func stopBackground() async }
public protocol DeviceCapabilityPort: Sendable { func tier() async -> DeviceTier; func capabilities() -> Capabilities; func thermal() -> AsyncStream<Thermal> }
public protocol SceneEnginePort { /* @MainActor where it touches views */ func load(_ spec: SceneSpec) async throws; func send(_ cmd: SceneCommand)
    var events: AsyncStream<SceneEvent> { get }; func setQuality(_ q: QualityProfile); func dispose() async }
public protocol RealtimePort: Sendable { func start(_ cfg: RtConfig); func stop(); func setTopics(_ t: Set<String>)
    var state: AsyncStream<RtState> { get }; var events: AsyncStream<RtEvent> { get } }
```

### 4.5 State, data, offline and outbox

| Concern | Choice |
|---|---|
| Server data | Repository + GRDB cache (projection), stale-while-revalidate, `ValueObservation` for reactive reads |
| Prefs/flags | `UserDefaults`/`@AppStorage` (non-sensitive), remote config |
| Secrets | Keychain (`AfterFirstUnlockThisDeviceOnly` or stricter) + optional Secure Enclave key (CryptoKit) |
| Feeds | Cursor pagination + diffable, bounded in-memory windows |
| Offline mutations | **Outbox** table + `BGProcessingTask`/foreground sync with `Idempotency-Key`, backoff, per-entity ordering |
| Conflicts | Typed results (`409`/version) surfaced to the ViewModel |
| Migrations | GRDB code migrations (versioned, forward-only) tested from every released schema |

The local DB is a **projection**; the backend is the authority.

---

## 5. Project layout, build system and variants

```text
ios/
├── Tuist/ or project.yml            # text project manifest (Tuist Project.swift / XcodeGen) — the .xcodeproj is generated, git-ignored
├── Config/                          # Base.xcconfig, Debug/Staging/Release.xcconfig, Game|Market|Avatar.xcconfig, *.entitlements
├── Apps/
│   ├── Game/   Sources/ HeimdallGameApp.swift  AppEnvironment+Game.swift   Resources/ PrivacyInfo.xcprivacy
│   ├── Market/ …
│   └── Avatar/ …
├── Packages/
│   ├── Core/{Kernel,Contracts,SDK,DesignSystem,UICore,Device,Telemetry,Security,Scene,SceneRealityKit,SceneMetal,SceneGodot,SceneUnity,Testing}
│   ├── Shell/{Identity,Notifications,Devices,Consent,Settings}/       # each: Domain, Data, UI, Contracts targets
│   └── Features/{Game,Market,Avatar}/<Slice>/…
├── Extensions/                      # NotificationService (code-only), NotificationContent (optional), Widgets/LiveActivities (optional), BackgroundAssets
├── Scripts/                         # ci, tdd runner, architecture checks, asset validation
└── tests/<slice>/ios/{e2e,contract,perf,security,a11y}/   tests/_platform/{architecture,device-perf,push,auth,store-readiness}/
docs/{adr,support-matrix.md,dependency-register.md,permissions-register.md,device-pool.md,runbooks/}
```

**Tests placement ADR:** unit/ViewModel/snapshot tests live next to their package (`Tests/` in each SwiftPM package, runnable with `swift test`/`xcodebuild test`); cross-cutting suites
(e2e, contract, device performance, security, accessibility audits, architecture) live under `tests/…/ios/` to match `PLAYBOOK_TESTS.md`.

### 5.1 Build rules

* **Text manifests** define targets, schemes, test plans, entitlements, Info keys and capabilities; **`.xcconfig`** holds build settings; regenerate the project in CI and locally (`tuist generate`/`xcodegen`).
* **Swift 6 language mode** in every package (`// swift-tools-version: 6.x`), with upcoming features enabled deliberately (for example `ExistentialAny`, `InternalImportsByDefault`) via `swiftSettings`.
* **Package resolution**: pinned versions, committed `Package.resolved`, release-age cooldown for new versions (Renovate/Dependabot), `swift package` dependency scanning (OSV) in CI.
* **Per-variant** entitlements, `Info` keys, background modes and usage strings only in that variant's manifest; CI diffs the merged Info/entitlements against the permissions register.
* **Compiler/linker settings**: whole-module optimization in Release, `-Osize` vs `-O` decided by measurement, dead-strip, strip symbols with dSYMs archived, bitcode not applicable, explicit-module builds as supported by the toolchain.
* **Xcode versions**: pin the Xcode version in CI; Xcode 27.x needs Apple silicon and macOS 26.6+; Xcode 26.6 is the last 26-series release if a runner can't move yet. `PreviewProvider` is deprecated in Xcode 27: use the `#Preview` macro everywhere.
* **Generated code** (OpenAPI client, asset/string symbols) is produced by plugins/scripts in CI and never hand-edited.
* **Per-variant size guard**: CI fails if the app slice size or the linked-framework list for a variant exceeds its allowlist.

---

## 6. Navigation, routes and deep links

* **NavigationStack** with value-typed routes (`enum Route: Hashable, Codable`) and a `Router` (`@Observable`, `@MainActor`) owning `NavigationPath`; one stack per tab; state restoration via `Codable` paths.
* The shared **route table** (JSON, same as other clients) is the source of truth; route builders and the `Route` enum are generated/verified against it with a **parity test**; no hand-written paths.

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]
/app/market/products/{productId}   /app/market/cart   /app/game/map   /app/game/encounters/{encounterId}
/app/avatar/sessions/{sessionId}   /app/notifications/preferences
Session-less: /onboarding  /login  /register  /forgot-password  /reset-password  /verify-email
```

* **Universal Links** (`https://<domain>/app/...`) with the `apple-app-site-association` file and Associated Domains entitlement per variant (also used for passkeys/`webcredentials` and AutoFill);
  avoid custom URL schemes except for development.
* A `DeepLinkParser` (allowlist of routes, validated params, size limits) feeds the router; a **synthetic back stack** is built from the route hierarchy; links received while signed out are queued and resumed after login/bootstrap.
* Guards (`RequireAuth`, `RequireUserType`, `RequirePermission`) only shape UX (redirect to `/login`, `/403`, allowed home); **the server authorizes every call**.
* Link and notification parameters are **untrusted**; never perform state-changing actions without UI confirmation.

---

## 7. Shared platform features: onboarding, login, registration, forgot password

Backend contract, UX rules and flows are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §7 (endpoints under `/api/v1/public/auth/*`, device registration at `/api/v1/app/devices`, consent, account deletion,
uniform forgot-password responses, single-use reset tokens, session revocation after reset). iOS-specific implementation:

### 7.1 Onboarding

3–5 skippable SwiftUI screens driven by remote config with bundled fallback; value first, permissions later (primers at point of use); variant steps (Game: safety + age gate; Market: country/language/interests;
Avatar: avatar choice, voice consent, AI disclosure). Low tier: static images, no looping heavy animation, honor Reduce Motion, support Dynamic Type up to accessibility sizes.

### 7.2 Registration

Minimal fields; inline password-policy feedback; **AutoFill** (`textContentType(.username/.newPassword/.emailAddress)`, Strong Password suggestions); versioned consent capture; age gate where required (consider the Declared Age Range API for
age assurance where applicable **(verify availability and regional requirements)**); verification by **Universal Link** or OTP with `textContentType(.oneTimeCode)` (SMS AutoFill) or email code. Abuse controls: server rate limits, **App Attest** assertion (and DeviceCheck)
as a risk signal, challenge only on high risk.

### 7.3 Login

* **Sign in with Apple** (`SignInWithAppleButton` / `ASAuthorizationAppleIDProvider`) — required when other third-party social logins are offered unless an equivalent privacy-preserving option exists **(verify guideline 4.8)**.
* **Passkeys** via AuthenticationServices (`ASAuthorizationPlatformPublicKeyCredentialProvider`, sign-in and registration), with the backend's WebAuthn options endpoint; AutoFill-assisted passkey sign-in on the username field.
* **Enterprise/social SSO**: OIDC Authorization Code + PKCE through **`ASWebAuthenticationSession`** (no embedded WKWebView, no Google/Facebook SDK weight unless required).
* **MFA/step-up** for sensitive actions; **Face ID/Touch ID** via LocalAuthentication gating a Keychain item (`SecAccessControl` with `.biometryCurrentSet`/`.userPresence`) for app lock; biometrics never replace server authentication.
* Throttling is server-side; the client shows generic errors and `Retry-After`.

### 7.4 Session and token storage

* Access token in memory; **refresh token in the Keychain** (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, or `WhenUnlockedThisDeviceOnly` for stricter apps), non-synchronizable, with an app-specific access group only if extensions need it.
* Single-flight refresh (an actor), rotation with server-side reuse detection, immediate wipe on sign-out/suspension; Data Protection class `complete`/`completeUnlessOpen` for sensitive files.
* Multi-role users: active role switch resets caches/subscriptions; `{userType}` selects the experience only.

### 7.5 Forgot password and reset

Request screen with **uniform response** and cooldown → email with a **single-use short-lived Universal Link** (`https://<domain>/reset-password?token=…`) and/or a code → reset screen validates on submit → all sessions revoked → login. Handle cold start from the link, expired/used tokens, offline,
keyboard avoidance. Never log or persist the token.

### 7.6 Permission primers (iOS specifics)

| Permission | Mechanism | Notes |
|---|---|---|
| Notifications | `UNUserNotificationCenter.requestAuthorization` (alert/badge/sound; optional **provisional**) | Show a primer first; the system prompt appears once, afterwards deep-link to Settings |
| Location | `CLServiceSession`/`CLLocationManager` **When In Use** first; **Always** upgrade only when needed; Precise vs Approximate | `NSLocationWhenInUseUsageDescription`, `NSLocationAlwaysAndWhenInUseUsageDescription` strings must be specific |
| Camera | `AVCaptureDevice`/ARKit | `NSCameraUsageDescription`; prefer `PhotosPicker` for images (no permission) |
| Microphone | `AVAudioApplication.requestRecordPermission` | `NSMicrophoneUsageDescription`; show an in-app live-mic cue |
| Speech recognition | `SFSpeechRecognizer`/`SpeechAnalyzer` authorization | Only if on-device STT is used |
| Face ID | `NSFaceIDUsageDescription` | For biometric lock |
| Tracking (ATT) | Only if you actually track across apps/sites | Otherwise omit |

Each denial has a degraded path (browse-only map, text chat, no AR).

### 7.7 Account lifecycle

In-app account deletion (initiate → re-authenticate → grace period → confirmation) required by the App Store **(verify guideline 5.1.1(v))**; server cascades to devices/push tokens, avatar assets, location history, caches; data export and consent management in settings;
terms/privacy version bumps trigger re-consent. Revoke Sign in with Apple tokens on deletion as required by Apple's REST API.

---

## 8. Push notifications (APNs, all variants)

### 8.1 Architecture

```text
App (UIApplicationDelegateAdaptor) ──register──► APNs token ──► POST /api/v1/app/devices {platform:"ios", token, provider:"apns",
      environment:"sandbox|production", appVariant, build, locale, tz, authorization:"authorized|provisional|denied", categories}
Backend `notifications` slice → provider adapter (APNs HTTP/2, token-based .p8 auth) → delivery receipts / dead-token pruning
```

Same backend contract and payload semantics as `PLAYBOOK_FRONTEND_MOBILE.md` §8.2 (`v`, `type`, `category`, `deepLink`, `correlationId`, collapse, `ttl`, priority, minimal `data`), mapped to APNs headers:
`apns-push-type` (`alert`/`background`/`liveactivity`), `apns-priority`, `apns-expiration` (ttl), `apns-collapse-id` (collapse), `apns-topic` (bundle ID per variant), `thread-id` for grouping.

### 8.2 Client implementation

* **Registration**: after login and on every launch call `registerForRemoteNotifications()`; the token arrives in `application(_:didRegisterForRemoteNotificationsWithDeviceToken:)`; upload with an idempotent call (retry/backoff via a background task); send the **environment**
  (sandbox vs production tokens differ) and re-register on token change.
* **Authorization**: request at a contextual moment (first order, first catch, first avatar chat) after a primer; **provisional authorization** delivers quietly if you want an earlier soft opt-in; report authorization state to the server.
* **Categories/threads**: map payload `category` to `UNNotificationCategory` (actions are optional; keep them non-destructive); promotions remain **off until opted in** (server-side preference + category).
* **Interruption levels**: `passive`/`active` by default; **time-sensitive** only for genuinely urgent events (requires the capability); **critical alerts** only with Apple's special entitlement.
* **Tap handling**: `UNUserNotificationCenterDelegate.didReceive` → validate payload schema → parse `deepLink` against the route allowlist → build the synthetic back stack → route after auth/bootstrap. **No state-changing action from the payload.**
* **Foreground presentation**: `willPresent` decides banner/sound/none by category and current screen; no duplicate alerts for the visible conversation.
* **Lock-screen privacy**: no PII in `title/body`; use `UNNotificationContent.hiddenPreviewsBodyPlaceholder`/category hidden-preview settings; details are fetched after the tap.
* **Notification Service Extension (code-only)**: attach media (download with strict size/time limits) and optionally decrypt payloads; keep memory and time small (extensions run under tight limits); skip rich media on Low tier or poor networks.
* **Background (silent) pushes** (`content-available`): hints only, heavily throttled by the system; never rely on them for correctness. Use `BGAppRefreshTask`/`BGProcessingTask` for deferrable work.
* **Live Activities (optional)** for Market order tracking and Game events; updates via `liveactivity` push type; keep payloads small and respect update budgets **(verify limits)**.
* **Token lifecycle**: unregister on logout/account deletion; server prunes on APNs `410 Unregistered`/`BadDeviceToken`.

### 8.3 Reliability and low-end notes

APNs delivery is best-effort; Low Power Mode and Focus modes affect presentation/timing; never depend on push for state. **Reconcile on foreground** via API/realtime. Test: sandbox and production APNs, simulator payload injection
(`xcrun simctl push`), permission states (denied, provisional), Focus modes, app killed/background/foreground, and real devices on the Low pool.

### 8.4 Server-side

Device registry (per user/tenant/environment), preferences, templates/i18n, scheduling by timezone, rate limits/collapse, idempotent send jobs, delivery receipts, dead-token pruning, audit trail for transactional/security notifications, per-category kill switch
(`PLAYBOOK_BACKEND.md` batch layer with Redis/Dragonfly coordination). Provider adapters stay behind a port so APNs-direct and FCM can coexist across platforms.

---

## 9. Library selection (works on older and mid-range iPhones)

### 9.1 Selection rules

1. **Prefer Apple frameworks and first-party Swift packages** (URLSession, CryptoKit, AuthenticationServices, StoreKit 2, MetricKit, swift-openapi-generator, swift-async-algorithms, swift-collections).
2. **Every dependency passes a weight check** on the Low pool: binary size, launch time, memory, and maintenance health (releases, issues, bus factor, license, Swift 6/strict-concurrency readiness). Record it in `docs/dependency-register.md`.
3. **No binary XCFrameworks without review**: privacy manifest present, signed, reproducible provenance, App Store–compatible.
4. **Static linking and few dependencies**; prefer small, focused packages; no UI kits.
5. **Swift 6 readiness**: packages must compile in Swift 6 mode (or be isolated behind a wrapper) before adoption.
6. **Pin versions** (`Package.resolved`), cooldown for fresh releases, SCA/SBOM in CI.

### 9.2 Curated stack

| Area | Library (default) | Low/mid-end notes | Alternatives / cautions |
|---|---|---|---|
| Language/runtime | Swift 6.x, Observation, Swift Concurrency | Strict concurrency | — |
| UI | **SwiftUI** (+ UIKit in code where measured) | Fine-grained invalidation; avoid heavy effects | No Storyboards/XIBs |
| Navigation | `NavigationStack` + own `Router` | Codable paths | TCA only if the team commits to it |
| DI | Constructor injection + composition root | No reflection | `swift-dependencies` optional |
| Networking | **URLSession**, **swift-openapi-generator** (+ runtime, URLSession transport), `URLSessionWebSocketTask`, SSE parser (own) | HTTP/2/3, tiny footprint | Alamofire adds weight without need |
| Concurrency utils | **swift-async-algorithms** (debounce, throttle, merge), swift-collections | Search debounce, event coalescing | — |
| Images | **Nuke/NukeUI** + ImageIO downsampling | Memory/disk caches, decoding off-main | Kingfisher is comparable |
| DB | **GRDB** | WAL, observation, migrations in code | SwiftData for simple cases **(verify)**; Core Data editor excluded |
| Secrets/crypto | Keychain (thin wrapper), **CryptoKit**, Secure Enclave | Small, native | — |
| Auth | **AuthenticationServices** (passkeys, Sign in with Apple, `ASWebAuthenticationSession`), **LocalAuthentication** | System UI | No embedded WebView login |
| Push | **UserNotifications**, APNs (server), notification service extension | Direct APNs, no Firebase SDK on iOS | FCM for iOS only if you already run it server-side |
| Maps (Game) | **MapLibre Native iOS** or **Mapbox Maps SDK** | GL layers; tile cache; cap pitch/3D buildings | **MapKit for SwiftUI** for few annotations; clustering needs care |
| Location | **Core Location** (`CLLocationUpdate.liveUpdates`, `CLMonitor`, `CLBackgroundActivitySession`, `CLServiceSession`) | Adaptive accuracy, geofencing | — |
| Camera/vision | AVFoundation, **VisionKit `DataScannerViewController`**, Vision | Disable analysis on Low | — |
| AR | **ARKit + RealityKit** | Optional; capability-gated | Unity AR Foundation for heavy AR |
| 3D | **RealityKit** (ECS, `RealityView`), **Metal/MetalKit** (+ MetalFX where supported), embedded **Godot** (Compatibility renderer) / **Unity as a Library** | RealityKit preferred; SceneKit is soft-deprecated | Verify engine embedding/maintenance and size (Godot adds roughly tens of MB) |
| 2D fallback | **SpriteKit**, SwiftUI `Canvas` | Cheap on Low | — |
| Game services | **GameKit** (Game Center), **GameController** | Optional provider behind a port | — |
| Audio | **AVAudioEngine** (+ voice processing), **AVAudioSession**, AVFoundation | AEC/NS via voice processing; route/interruption handling | — |
| Speech | **Speech** framework (`SFSpeechRecognizer`; newer `SpeechAnalyzer`/`SpeechTranscriber` where available **(verify)**), cloud streaming for quality | On-device only on capable devices | `AVSpeechSynthesizer` fallback TTS |
| Realtime voice | **LiveKit Swift SDK** (WebRTC) **(verify maintenance)** | Opus, echo cancel | WebSocket streaming fallback |
| On-device ML | **Core ML**, Vision, SoundAnalysis (VAD/event models) | Neural Engine on A11+; Mid/High for heavy models | Server fallback |
| On-device LLM | **Foundation Models** framework | Only Apple Intelligence-capable devices; optional | Never required for core flows |
| Payments | **PassKit (Apple Pay)** + **Stripe iOS SDK**; **StoreKit 2** for digital goods | PCI scope minimized | RevenueCat optional |
| Search | Backend engine; `.searchable` with debounce | Cached suggestions | — |
| i18n | String Catalogs, `String(localized:)`, `Locale` | Pseudo-locales | — |
| Observability | **MetricKit**, `os.Logger` + signposts, **Sentry Cocoa** (or Crashlytics) | Sampling tuned for Low | — |
| Remote config | Backend config / GrowthBook | Tier overrides | — |
| Testing | **Swift Testing**, XCTest/XCUITest (incl. `performAccessibilityAudit`), **swift-snapshot-testing**, Maestro (optional) | Real-device pool | — |
| Quality | **SwiftLint**, **swift-format**/SwiftFormat, **Periphery** (dead code), Instruments | CI gates | — |
| Asset tooling | **gltf-transform/gltfpack**, `usdz` tools, `toktx`/KTX2 or ASTC textures | CI-enforced budgets | — |

### 9.3 Avoid on low-end

WKWebView-hosted games/3D; third-party UI kits and heavy animation libraries; many dynamic frameworks; reflection-heavy/runtime-parsing libraries on the launch path; unbounded image caches; `AsyncImage` for lists;
SceneKit for new work; large Lottie stacks; polling timers; synchronous disk/network on the main actor; Combine pipelines where `async`/Observation suffice (keep Combine only where a framework requires it).

---

## 10. Variant A — Game (location-based 3D, Pokémon-GO-style)

Gameplay, server-authoritative design, anti-cheat, economy and compliance are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §10. iOS-native specifics:

### 10.1 Slices

`map`, `spawns`, `encounters`, `catching`, `inventory`, `collection`, `events`, `social`, `shop`, `progression`, `safety`; permissions like `game.encounters.create`, `game.inventory.read`; backend slice `game`.

### 10.2 Map and world

* **MapLibre/Mapbox GL layers** for spawns/POIs/player marker (symbol/circle/fill-extrusion fed by GeoJSON sources); no per-entity SwiftUI views over the map; offline tile cache; cap zoom/pitch; disable 3D buildings and dense labels on Low;
  simplified style per tier. MapKit annotations are acceptable only for small counts (use clustering).
* Viewport queries + realtime events (`GET /api/v1/app/game/map/nearby?bbox=…&cursor=…` + `game.spawn.created/expired`), snapshot + stream.
* Entities on the map are 2D sprites/billboards by default; 3D appears in the encounter screen and optionally on higher tiers.

### 10.3 Location, battery and background rules

| Mode | API | Notes |
|---|---|---|
| Foreground, high accuracy | `CLLocationUpdate.liveUpdates(.otherNavigation/.fitness)` with `CLServiceSession` | Map open; adaptive by speed |
| Foreground, balanced | Reduced accuracy/distance filter | Walking |
| Geofences | `CLMonitor` conditions (region limit applies) | Cheapest; OS-driven |
| Background (optional) | `UIBackgroundModes: location` + **`CLBackgroundActivitySession`** (shows the blue status indicator) + **Always** authorization | App Store review needs a clear justification; default to foreground-only play |

Rules: batch uploads (deltas, not streams), pause when stationary/low battery/thermal pressure, use `CLLocationSourceInformation.isSimulatedBySoftware` and accessory flags as **anti-cheat signals only**, detect vehicle movement via Core Motion activity for safety warnings,
prove battery impact on the Low pool each release.

### 10.4 3D encounter and AR

* **Tiers**: T0 SpriteKit/sprite flipbook; T1 RealityKit low-poly with baked lighting at 30 fps; T2 Metal custom renderer or embedded Godot (Compatibility renderer) for richer scenes; T3 Unity/ARKit for AR-heavy high-end (section 13).
* **RealityKit in code**: build `Entity` hierarchies and components programmatically inside a `RealityView` (iOS 18+); load USDZ assets via `Entity(named:)`/async loaders; **don't depend on Reality Composer Pro scene files** (D3).
* **AR is optional and capability-gated**: check `ARWorldTrackingConfiguration.isSupported`, device tier and thermal state; geo-anchoring (`ARGeoTrackingConfiguration`) works only in supported regions and needs location/GPS **(verify coverage)**, so the default AR is world-tracking with GPS-informed
  spawn placement and a **non-AR fallback** encounter.
* Throw/catch input → intent to the server; the engine renders; only events cross to the shell (`encounter.thrown`, `encounter.result`).
* **Game platform APIs**: Game Center via a port; Game Mode behavior for game-category apps **(verify)**; `GameController` for controllers; frame pacing with `CAMetalDisplayLink`.
* SceneKit is **not** used for new work (soft-deprecated since iOS 26; `SceneView` formally deprecated); existing SceneKit code follows Apple's migration sample to RealityKit.

### 10.5 Assets and performance

Base bundle + **Background Assets** (system-managed asset packs, Apple-hosted or self-hosted **(verify hosting limits)**) or On-Demand Resources for region/creature packs; compressed textures (ASTC) per tier; prefetch on Wi-Fi; memory ceilings enforced by the governor;
unload encounter assets on exit; 20-minute soak on the Low pool (temperature, battery, frame time, memory); map updates batched at ≤ 10–20 Hz.

### 10.6 Compliance

Digital items through StoreKit 2 with **server-side validation of signed transactions/App Store Server Notifications** and idempotent grants (`appAccountToken` links purchases to users); odds disclosure for loot-box-style mechanics (an App Store requirement) and regional rules **(verify)**;
spending limits and parental controls for minors; moderation/reporting for user-generated names/chat; safety reminders and speed warnings.

---

## 11. Variant B — Market (Amazon-style)

Slices, flows, server-authoritative pricing, idempotent checkout, search, reviews and compliance are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §11. iOS specifics:

* **Lists**: `LazyVStack`/`LazyVGrid`/`List` with stable IDs and fixed-aspect tiles; cursor pagination with prefetch near the end; skeleton placeholders; item models precomputed off-main; if Instruments shows layout cost on very large feeds, host a code-only `UICollectionView` (compositional layout + diffable data source).
* **Images**: Nuke with exact display size and CDN-resized WebP/AVIF/HEIC variants; downsampling; memory cache cap proportional to RAM class; placeholders (blurhash/thumbhash).
* **Home rails**: server-driven *layout* of typed native rail components (no remote code).
* **Search**: `.searchable` + `AsyncAlgorithms` debounce (~250 ms), cached suggestions/recents; **barcode** via `DataScannerViewController` (VisionKit; A12+) or AVFoundation metadata output; voice search optional.
* **Checkout**: server computes totals/taxes/shipping; **Apple Pay** (`PayWithApplePayButton`/PassKit) and **Stripe PaymentSheet** for physical goods; PCI scope minimized; `Idempotency-Key` on order creation; hide sensitive content in the app switcher; **StoreKit 2** for digital goods.
* **Photos for reviews**: `PhotosPicker` (no library permission), compress/resize on-device, strip EXIF, upload via background `URLSession`.
* **Order tracking**: push + optional **Live Activities**; Wallet order tracking is optional **(verify requirements)**.
* **Seller mode** (`/app/market/seller/…` or `{userType}=partner`): role-gated, listing creation with the image pipeline, inventory, orders, payouts onboarding via the provider's hosted flow.
* **Performance**: avoid blur/large shadows on cards, flatten view hierarchies, avoid nested scroll views, profile journeys (home → search → product → cart → checkout) with Instruments and XCTest metrics.
* **3D/AR product view**: off by default; lazy-loaded RealityKit `RealityView`/AR Quick Look (system) with a 2D fallback, Mid/High only; AR Quick Look needs no custom engine code.

---

## 12. Variant C — Avatar (3D, voice and movement)

Slices (`avatars`, `sessions`, `voice`, `performance`, `tracking`, `wardrobe`, `safety`), avatar formats and sourcing (glTF/GLB + VRM 1.0 source assets; provider-agnostic; selfie data as sensitive), the performance-script contract, latency budgets, privacy and AI disclosure are **identical** to
`PLAYBOOK_FRONTEND_MOBILE.md` §12. iOS-native implementation:

### 12.1 Rendering

* **RealityKit** (`RealityView`) for the avatar viewer: ship avatars as **USDZ** (converted from GLB/VRM in the asset pipeline) with morph targets driven through RealityKit's blend-shape components (`BlendShapeWeightsComponent`, iOS 18) and skeletal animation **(verify API surface in your SDK)**;
  VRM expressions are mapped to blend shapes in an adapter, and the humanoid bone map is baked in the pipeline. There is no first-party VRM runtime on iOS, so the **asset pipeline** performs the conversion rather than runtime parsing.
* Low tier: **SpriteKit/2D fallback** (pre-rendered sprite avatar with audio-driven mouth frames) or a very low-poly model; Mid: RealityKit at 30 fps; High: richer lighting/expressions.
* Pause rendering when off-screen/backgrounded; animation updates stay in the engine; SwiftUI sends high-level commands.
* Budgets per tier as in `PLAYBOOK_FRONTEND_MOBILE.md` Appendix E (Low ≤ ~10k tris, ≤ 25 bones, 512 px textures, 10–15 blend shapes).

### 12.2 Voice pipeline (iOS specifics)

```text
Mic → AVAudioEngine input (voice processing: AEC/noise suppression) + VAD
   → STT (cloud streaming | Speech framework on-device where available) → text → backend conversation service (LLM + safety)
   → performance script → TTS (cloud with viseme/alignment timestamps | AVSpeechSynthesizer fallback) → AVAudioPlayerNode (low-latency) + timeline
   → PerformancePlayer (audio clock is master) → SceneEngine (visemes, gestures, emotion)
```

* **Audio session**: `AVAudioSession` category `.playAndRecord` with mode `.voiceChat` (or `.default` + voice processing on the engine), options for Bluetooth/speaker routing; handle **interruptions** (calls, Siri), **route changes**, and background audio only if the feature truly needs it (`UIBackgroundModes: audio`).
* **Transport**: WebRTC (LiveKit or equivalent) for real-time audio; WebSocket streaming as the simpler fallback; adaptive bitrate; degrade to text chat.
* **Latency**: stream STT partials, LLM tokens and TTS chunks; barge-in (stop playback when the user speaks); measure end-to-end on the Low/Mid pool.
* **Camera-driven movement (optional)**: **ARKit face tracking** (TrueDepth devices) provides ARKit blend-shape coefficients that map directly to avatar expressions; on non-TrueDepth devices use Vision face landmarks at low frame rates, Mid/High only; disable on Low, on thermal/Low Power Mode, and
  when backgrounded; raw frames never leave the device (derived parameters only, with consent).
* **Lip-sync fallbacks** by quality/cost: TTS visemes → server phoneme alignment → on-device amplitude estimation (Low tier).
* **Privacy/AI**: AI disclosure, server-side safety filters, no recording by default, retention controls, no voice cloning without explicit revocable consent (Personal Voice is a user-owned system feature, not a cloning pipeline for your app), selfie-to-avatar data deleted after generation.
* **On-device models** (Core ML VAD, small classifiers) only on Mid/High; **Foundation Models** (Apple Intelligence devices) may assist with local intent/gesture tagging but is never required.

### 12.3 Sessions and entitlements

Session lifecycle via `POST /api/v1/app/avatar/sessions`, server-enforced usage limits and entitlements (STT/TTS/LLM cost controls), reconnect/resume, push for scheduled sessions, wardrobe items as digital goods via StoreKit 2 with server-validated entitlements.

---

## 13. Scene engines, 3D assets and delivery

### 13.1 `SceneEnginePort` and tiers

| Tier | Package | When |
|---|---|---|
| **T0** | `SceneSprite` (SpriteKit/SwiftUI Canvas flipbooks) | Low devices, safe mode, thermal emergency, unsupported GPU family |
| **T1** | `SceneRealityKit` (`RealityView`, ECS built in code) | Mid 3D viewers, encounters, avatars |
| **T2** | `SceneMetal` (custom renderer, MetalFX where supported) or `SceneGodot` (Compatibility renderer) | Game scenes needing custom rendering/physics/scripting |
| **T3** | `SceneUnity` (AR Foundation) / ARKit via RealityKit | AR-heavy, high tier only |

* Adapters implement the same commands/events; scenes are **data** (scene specs/assets), not shell logic. Fallback chain on init failure or repeated frame-time breach: step down a tier (persisted) and report.
* Render surfaces are hosted through `RealityView`/`UIViewRepresentable` (`MTKView`); lifecycle tied to scene phase (pause on inactive/background, release on dismantle).
* Engine packages are separate optional targets; a variant without 3D contains none of them. **WKWebView 3D/HTML5 games are not allowed** on the Low/Mid path.

### 13.2 Engine notes

* **RealityKit**: Apple's forward path for 3D and AR (ECS, PBR, USD/USDZ, physics, animation); cross-platform API since iOS 18 (`RealityView`); build entities/systems/components in Swift. Apple publishes a sample showing how to port a SceneKit platformer to RealityKit.
* **Metal**: for custom rendering and maximum control (A-series GPU families, MetalFX upscaling on supported chips **(verify chip support)**); budget engineering time; keep a RealityKit path for the Mid tier.
* **Godot**: embeddable in iOS apps; the Compatibility renderer targets low-end; adds binary size (tens of MB) and build complexity **(verify embedding tooling and App Store review considerations)**.
* **Unity as a Library**: strong AR tooling, large footprint; AR-first high tier only.
* **SceneKit**: soft-deprecated since iOS 26 (critical fixes only); no new features or new projects.

### 13.3 Asset pipeline (CI-enforced) and delivery

1. DCC → glTF/GLB/VRM (source of truth) → **USDZ** conversion for RealityKit (and keep GLB for other engines). 2. **gltf-transform/gltfpack**: simplification, **LOD chains**, quantization, dedupe/prune.
3. Textures → **ASTC** (and KTX2 for cross-engine) at tiered resolutions with mipmaps. 4. **Validation + budget checks** (triangles, bones, materials, textures, draw calls, blend shapes) fail the build.
5. **Background Assets** packs (or On-Demand Resources / CDN with a signed manifest): install-time core, on-demand per region/creature set; resumable downloads, Wi-Fi preference, disk quota + LRU eviction, integrity check, rollback to the previous pack.

### 13.4 Memory discipline

Texture memory dominates: GPU-compressed formats, tiered resolutions, shared materials, unload on exit, no duplicate CPU copies, respond to memory warnings, keep the footprint below the tier ceiling **and below the jetsam limit for the device class** (read `os_proc_available_memory()` and refuse to load assets that don't fit).

---

## 14. Realtime, connectivity and background behavior

* Realtime follows `PLAYBOOK_DESKTOP_QT.md` §8 / `PLAYBOOK_FRONTEND_MOBILE.md` §14: `RealtimeMode = auto | websocket | sse | polling | off`, snapshot + stream, dedupe by event ID, resume tokens, resync after gaps, per-topic authorization server-side.
* **WebSocket**: `URLSessionWebSocketTask` with `Authorization` header, app-level heartbeat plus ping, message-size caps, schema-validated envelopes.
* **SSE**: stream with `URLSession.bytes(for:)` and parse the **raw byte stream** with a spec-compliant parser; **do not rely on `AsyncBytes.lines` for SSE** because it does not surface the blank lines that delimit events **(verify on your SDK)**. Request `Accept: text/event-stream`, `Last-Event-ID` on resume, a timeout longer than the server heartbeat,
  token in the header (never in the URL). Run the shared SSE conformance fixtures.
* **Lifecycle**: connect when the scene is active; suspend/disconnect on background; observe reachability with `NWPathMonitor`; backoff with jitter; **push is the wake-up channel**; reconcile on foreground.
* **Background work**: `BGAppRefreshTask`/`BGProcessingTask` for deferrable work (outbox sync, uploads, asset prefetch) and **background `URLSession`** for uploads/downloads that must survive suspension; no polling loops; design for **no background execution** by default.
* **Network adaptation**: `NWPath.isConstrained`/`isExpensive` (Low Data Mode, cellular) → lower image quality, disable prefetch, defer large packs to Wi-Fi.
* Handle Low Power Mode (`NSProcessInfoPowerStateDidChange`) and thermal changes in the governor.

---

## 15. Security, privacy and App Store compliance

### 15.1 Baseline (OWASP MASVS-style)

| Area | Controls |
|---|---|
| Storage | Keychain for secrets (device-only accessibility); Data Protection classes for files; no secrets in `UserDefaults`/GRDB plaintext/logs; encrypt sensitive DB fields (SQLCipher or CryptoKit AES-GCM with a Keychain/Secure Enclave–held key) |
| Network | **ATS** with no exceptions; TLS 1.2+; optional pinning via `NSPinnedDomains` (declarative, supports backup keys/rotation) or `URLSessionDelegate` public-key pinning with a kill switch and rotation plan |
| Auth | Section 7; refresh rotation; biometric gate; step-up; session list/revoke |
| Components | Verified Universal Links; avoid custom URL schemes; validate all inbound URLs/notification payloads; minimal App Groups/Keychain sharing; no unnecessary extensions |
| UI | Privacy overlay in the app switcher (`scenePhase`), `UIScreen.isCaptured` handling for sensitive screens, secure text entry, pasteboard minimization |
| Binary | Release builds stripped, no secrets/API keys in the bundle (inject via CI + restrict keys), debug flags off; obfuscation is not security |
| Platform trust | **App Attest** (+ DeviceCheck) assertions verified server-side as risk signals; jailbreak signals advisory; never block solely on client checks |
| WebView | Avoid `WKWebView`; if unavoidable: restrict origins, disable JS bridges to untrusted content, `limitsNavigationsToAppBoundDomains` where applicable |
| Supply chain | Pinned SwiftPM versions, cooldown for new versions, SCA/SBOM, signed CI artifacts, action pinning by SHA (`PLAYBOOK_SECURITY.md` §11); binary frameworks reviewed |
| Native engines | Review sources/licenses; scan binaries; privacy manifests for third-party SDKs |
| Memory safety | Swift's safety + strict concurrency; audit `unsafe` pointer use and C interop; fuzz parsers (SSE, deep links, manifests) |

### 15.2 Privacy

* Data minimization per variant; **location, microphone, camera, voice and face-derived data are sensitive**: explicit consent, purpose limitation, retention limits, deletion with the account, regional rules **(verify)**.
* **Privacy manifests** (`PrivacyInfo.xcprivacy`) for the app and every third-party SDK, including *required reason API* declarations; **App Privacy nutrition labels** kept in sync with actual SDK behavior; ATT only if you track.
* Children/teens: age gate and age-appropriate defaults for Game/Avatar; Kids Category rules if targeted; restrict chat/voice/purchases by age; consider the Declared Age Range API **(verify)**.
* Analytics/telemetry: consent-aware; no precise location or content; hashed IDs; retention limits.

### 15.3 App Store checklist (verify each release)

| Topic | Requirement to confirm |
|---|---|
| Toolchain | Built with Xcode 26+/iOS 26 SDK (current floor) and the newer SDK when Apple sets the deadline |
| Sign in with Apple | Offered when other third-party social logins exist (guideline 4.8) |
| Account deletion | In-app initiation (5.1.1(v)) and Sign in with Apple token revocation |
| Permissions | Specific usage strings; background location justification; `UIBackgroundModes` limited to what's used |
| Payments | Digital goods via StoreKit; physical goods via Apple Pay/Stripe; correct disclosures |
| UGC/AI | Reporting, blocking, moderation, AI disclosure |
| Privacy | Manifests and nutrition labels match reality; export compliance (`ITSAppUsesNonExemptEncryption`) |
| Age rating | Questionnaire reflects chat, voice, purchases, location |
| Loot boxes | Odds disclosure and regional restrictions if applicable |
| Push | Time-sensitive/critical entitlements only if justified |
| Review notes | Demo account, test instructions, location simulation steps for Game |

---

## 16. Observability

| Signal | Implementation |
|---|---|
| Crashes/hangs | Sentry Cocoa (or Crashlytics) + **MetricKit** diagnostics (`MXCrashDiagnostic`, `MXHangDiagnostic`, `MXCPUExceptionDiagnostic`); dSYMs uploaded in CI; tags: variant, tier, device class |
| Performance | **MetricKit** payloads (launch, hang rate, memory, battery, disk I/O), `os_signpost` intervals for startup phases/screens, frame-time p50/p95 in scenes, thermal/battery state, governor decisions; Xcode Organizer for field trends |
| Jetsam/memory | Previous-launch termination reasons from MetricKit; `os_proc_available_memory()` samples; lower the tier for device classes with repeated terminations |
| Network | Latency/error rates per endpoint and network type; realtime state transitions and lag; asset pack download success |
| Product | Onboarding → registration → verification funnel; checkout; encounter; session start; push opt-in/open rates |
| Tracing | `X-Correlation-Id` per user action, propagated via SDK/realtime/push payloads, shown on error screens |
| Diagnostics bundle | User-initiated, redacted: variant/version/build, device tier/capabilities, engine/renderer, governor history, permissions and notification authorization, push status, recent `OSLogStore` entries/errors with correlation IDs |
| Privacy | Consent-aware sampling, redaction (`Logger` privacy modifiers), no PII/precise location in logs |

---

## 17. Testing strategy

Follows `PLAYBOOK_TESTS.md`; adds an `ios` side (placement ADR in section 5):

| Level | Scope | Tooling |
|---|---|---|
| Unit | Domain use cases, ViewModels, mappers with fakes | **Swift Testing** (`@Test`, `#expect`, parameterized); `swift test` for Domain packages (also on Linux CI) |
| Architecture | Package/target graph and layering rules | dependency-graph script, SwiftLint custom rules |
| Contract | SDK vs OpenAPI (generator output + parity), push payload schema, performance-script schema, route table parity, permission catalog | schema tests, URLProtocol stubs |
| View/snapshot | Screens per status; Dynamic Type, dark mode, locale/RTL, Reduce Transparency | **swift-snapshot-testing**, `#Preview` as living examples |
| Integration | Repository + GRDB + outbox + background tasks, push handling, realtime transports conformance | Swift Testing + fakes |
| **UI/E2E** | Onboarding → register → verify → login → forgot → reset; permission denial paths; push-tap deep links; market checkout; game encounter (simulated location); avatar session (mock audio/script) | XCUITest (accessibility identifiers) / Maestro, on simulators **and** real devices |
| **Device performance** | Launch, scroll hitches, scene frame time/memory/thermal on the Low/Mid pool | **XCTest metrics** (`XCTApplicationLaunchMetric`, `XCTOSSignpostMetric`, `XCTMemoryMetric`, `XCTCPUMetric`), Instruments traces, custom scene harness |
| Memory/limits | Memory ceilings, leaks, jetsam behavior | Instruments (Leaks/Allocations), memory graph checks |
| Location/anti-cheat | GPX routes (walk/drive/teleport), poor GPS, permission changes | Simulator location, `simctl location`, test servers |
| Audio/voice | Latency, barge-in, route changes, interruptions, lip-sync drift | Instrumented harness with recorded audio |
| Push | Simulator payloads (`xcrun simctl push`), sandbox/production APNs on devices, authorization states, Focus/Low Power, collapse/ttl | device farm + test sender |
| Accessibility | VoiceOver flows, Dynamic Type XXXL/accessibility sizes, contrast, Reduce Motion/Transparency, touch targets | `performAccessibilityAudit()` + manual |
| Security | MASTG-style checks, Keychain/file inspection, URL-scheme/Universal Link fuzzing, ATS/pinning config, secrets scan, dependency/SBOM scan | MobSF-class tools, custom |
| Compatibility | iOS 18 → current, iPhone SE → Pro Max, iPad (if supported), languages/RTL, offline/slow networks | simulators + device farm |
| Upgrade | N-1 → N data/migration and Keychain/token continuity | scripts |
| Store readiness | Entitlements/Info/privacy-manifest diffs, App Store validation, TestFlight processing | CI scripts, `xcodebuild -exportArchive` validation |

Rules: tests run on the **Low pool**, not only simulators; no production data; flaky tests quarantined with owner/expiry; every notification type and route has a contract/e2e test; performance gates compare to the last release (e.g., +10% regression fails the build) and are stored as release evidence.

### 17.1 TDD loop with Claude Code (`PLAYBOOK_CLAUDE_CODE.md`)

The TDD runner dispatches by marker: a package directory with `Package.swift` → `swift test` in that directory (optionally `--filter <Pattern>` to narrow by test name); app/UI suites → `xcodebuild test -only-testing:…` with an `.xctestplan`; output piped through a formatter (xcbeautify/xcsift). Roles stay separate (test-writer edits `Tests/**`, implementer edits `Sources/**`).
Visual verification uses `#Preview` rendering in Xcode 26.3+ or simulator screenshots compared with snapshots; **views remain code**.

---

## 18. Build, release and operations

* **Code-defined CI** on Apple-silicon runners with a pinned Xcode, cached SwiftPM/derived data; steps: generate project → build variants → unit/snapshot tests → architecture/privacy/entitlement diffs → archive → validate → upload to TestFlight (App Store Connect API key), all in version control.
* **Signing**: App Store Connect API key + cloud-managed or fastlane-match style certificates; secrets in the CI secret store; no developer-machine signing for releases; entitlement changes reviewed by CODEOWNERS.
* **Release channels**: internal TestFlight (every merge to main) → external TestFlight (pilot) → App Store with **phased release** and pause; App Store Connect metrics + MetricKit gates promotion.
* **Remote config** (data only, validated/signed): feature flags, device-tier overrides and blocklists, quality defaults, onboarding content, realtime mode, push categories, minimum version (force-upgrade screen linking to the App Store), emergency kill switches (AR, background location, voice, 3D, checkout provider).
* **Pipeline**: PR (build, lint/format, architecture, unit/snapshot/contract, dependency/SBOM/secrets scan, size and entitlement diff, asset budgets) → main (variant builds, simulator UI smoke) → nightly (full UI/e2e, device-farm, Low-pool performance, soak, push suite) → RC (all + store-readiness, accessibility, security, evidence bundle) → phased release → monitoring and rollback (pause phased release, kill switch, expedited hotfix).
* **Runbooks**: crash/hang spike on a device class (blocklist + tier override), APNs credential expiry/outage, App Review rejection/appeal, jetsam wave (memory ceilings), engine regression, asset-pack corruption (rollback), payment provider incident, IdP outage, background-location review, privacy incident,
  avatar-provider loss, voice/LLM outage (degrade to text).

---

## 19. Governance, compliance and working with Claude Code

* **Ownership:** CODEOWNERS for the project manifest, `Config/**`, entitlements/Info keys, `PrivacyInfo.xcprivacy`, `Package.resolved`, `Packages/Core/{Security,Scene*}`, push/auth packages, `docs/permissions-register.md`.
* **Documents:** ADRs, dependency and permissions registers, device pool/tiers, support matrix, threat model, data-flow diagrams, privacy review, App Store submission checklist, runbooks.
* **Compliance mapping** (`PLAYBOOK_AUDIT.md`): SDLC controls via CI gates; vendor management for SDKs/providers (maps, payments, voice/LLM, avatar, crash reporting); privacy evidence for location/voice/face data; change management via TestFlight + phased release; incident runbooks;
  evidence bundle per release (builds, SBOM, test/perf reports, entitlement/privacy diffs, approvals). CRA applicability for distributed apps: assess with counsel **(verify)**.
* **Claude Code** (`PLAYBOOK_CLAUDE_CODE.md`): path-scoped rules for `ios/**`, `Packages/**`, `Apps/**`; **rule: every view is Swift code; never create `.storyboard`, `.xib`, `.rcproject`, `.xcdatamodeld`**; the integrator role owns manifests, `.xcconfig`, entitlements, `Package.resolved` and CI;
  skills: `new-ios-feature` (package scaffold: Domain/Data/UI/Contracts + tests), `new-notification-type` (schema, category, deep link, handler, test), `new-scene-asset` (pipeline + budgets), `ios-perf-check`, `privacy-manifest-update`;
  specs list variants, entitlements/usage strings, tiers, budgets, push types, routes and test levels per AC. Xcode 26.3+ can host Claude Agent directly; prefer MCP/CLI-driven builds for reproducibility in CI.

---

## 20. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| Storyboards/XIBs/Interface Builder, Reality Composer Pro scenes or `.xcdatamodeld` as source of truth | Unreviewable, merge-hostile, not agent-editable |
| Hand-editing `project.pbxproj`; committing a generated project | Merge conflicts and drift |
| Xcode Cloud GUI-only workflows | CI not reviewable as code |
| One app with all three products and all engines | Size, memory, entitlements and review surface |
| UI code importing `Data` or platform adapters; `Domain` importing SwiftUI/URLSession | Breaks layering and fast tests |
| Business logic in views; networking in `body` | Untestable, re-render cost |
| `ObservableObject` + broad `@Published` everywhere on hot screens | Over-invalidation on old CPUs |
| Unstructured `Task {}`/detached tasks without cancellation; `@unchecked Sendable` sprinkled around | Data races and leaks |
| Blocking the main actor (disk, network, JSON, crypto) | Hangs and watchdog kills |
| `AsyncImage` for lists; unbounded image caches; full-size bitmaps | Memory/jetsam |
| Many dynamic frameworks; heavy work in `App.init` or static initializers | Slow launch |
| SceneKit for new work; WKWebView 3D/games on Low/Mid | Deprecated/slow |
| Custom translucent/glass stacks over scrolling content without tier gating | GPU cost on older devices |
| Requiring ARKit/TrueDepth/Apple Intelligence for core flows | Excludes many devices |
| Using `AsyncBytes.lines` for SSE | Drops blank-line event delimiters |
| Tokens in `UserDefaults`/GRDB plaintext/logs; embedded login WebViews; custom crypto | Credential theft |
| Asking all permissions at first launch | Denials and review risk |
| Push payloads with sensitive data; push as the only sync path | Privacy leaks, missed state |
| Dependence on one avatar/voice vendor runtime | Shutdown risk (e.g., Ready Player Me closed in January 2026) |
| No Low-pool testing; trusting simulators | Unproven claims |

---

## 21. Definition of done and checklists

### 21.1 Feature DoD

- [ ] Spec lists variant(s), slice, routes, permissions, push types, tier behavior, budgets and test levels per AC.
- [ ] Views are Swift code with `#Preview`; no forbidden visual-builder files; ViewModels/use cases tested with fakes; all statuses covered; no logic in views.
- [ ] Low-tier path implemented and verified on the Low pool; governor behavior defined; Compatibility visual mode honored.
- [ ] Uses the generated Swift SDK; no hand-written paths/DTOs; error taxonomy handled; correlation IDs surfaced.
- [ ] Permissions requested in context with primer + degraded path; usage strings/entitlements and the permissions register updated.
- [ ] Accessibility verified (VoiceOver, Dynamic Type accessibility sizes, contrast, Reduce Motion/Transparency); strings in String Catalogs; RTL/pseudo-locale checked.
- [ ] Offline/slow-network behavior defined; outbox/idempotency for mutations.
- [ ] Swift 6 strict concurrency clean; no main-actor blocking (Instruments checked); launch and scroll metrics within budget.
- [ ] Telemetry and redaction verified; no sensitive data in logs/analytics/push.

### 21.2 Release checklist

- [ ] Reproducible builds per variant (generated project, pinned Xcode); binary size, entitlement, Info-key, privacy-manifest and dependency diffs reviewed; SBOM and scans green.
- [ ] Unit/architecture/contract/snapshot/UI/accessibility/security suites green on simulators and real devices.
- [ ] Low/Mid pool performance within budgets (launch, hangs, scroll, scene fps, memory, battery, crash-free).
- [ ] Push verified end to end (authorization states, foreground/background/killed, deep links, token lifecycle, sandbox vs production).
- [ ] Auth flows verified: register, verify, login (password/passkey/Sign in with Apple), MFA/step-up, forgot, reset, sign-out, deletion.
- [ ] App Store checklist verified (toolchain/SDK, guideline items, privacy manifests/labels, usage strings, age rating).
- [ ] Phased release, kill switch and rollback plan ready; runbooks current.

### 21.3 Architecture acceptance checklist

- [ ] Current Xcode/Swift toolchain, Swift 6 mode, deployment target decision recorded; `#Preview` everywhere.
- [ ] Text project manifest + `.xcconfig` + entitlements in files; generated project git-ignored; CI blocks storyboards/XIBs/RCP/`.xcdatamodeld`/hand-edited pbxproj.
- [ ] Three app targets; per-variant packages, entitlements and usage strings verified by CI.
- [ ] `Domain` targets import no UI/platform frameworks; `UI` never depends on `Data`; cross-slice only via `Contracts`; dependency-graph check green.
- [ ] Observation ViewModels with shared statuses; UI packages default to `@MainActor`; repositories are actors; no unstructured tasks leaking.
- [ ] Route table parity with `Route` enum; Universal Links verified; deep links validated and resumed after auth.
- [ ] Shared shell: onboarding, login (Sign in with Apple/passkeys/OIDC), registration, forgot/reset, session security, deletion.
- [ ] Push: APNs token auth, categories, authorization flow, token lifecycle, notification service extension, deep-link routing.
- [ ] `DeviceTier`, measured benchmark, `QualityGovernor` (thermal, Low Power Mode, memory, frame pacing), remote overrides, performance modes.
- [ ] `SceneEnginePort` with sprite, RealityKit, Metal/Godot, Unity adapters; lazy load/dispose; fallback chain; no SceneKit for new work.
- [ ] Asset pipeline (USDZ/ASTC, LODs, budgets), Background Assets or CDN delivery with signed manifests.
- [ ] Game: server-authoritative, GL-layer map, Core Location modes, ARKit optional with fallback, anti-cheat signals.
- [ ] Market: server-authoritative pricing, idempotent checkout, Apple Pay/Stripe, lazy lists + Nuke sizing.
- [ ] Avatar: open formats converted in the pipeline, provider port, voice pipeline with barge-in, performance-script contract, optional face tracking.
- [ ] MetricKit/os_signpost/crash reporting, remote config, kill switch, diagnostics bundle.
- [ ] Device-lab testing with Low/Mid pool and CI performance gates.

---

## Appendix A — Package manifest for a slice (sketch)

```swift
// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "MarketCart",
    platforms: [.iOS(.v18)],
    products: [
        .library(name: "CartContracts", targets: ["CartContracts"]),
        .library(name: "CartDomain",    targets: ["CartDomain"]),
        .library(name: "CartData",      targets: ["CartData"]),
        .library(name: "CartUI",        targets: ["CartUI"]),
    ],
    dependencies: [ .package(path: "../../Core/Kernel"), .package(path: "../../Core/SDK"), .package(path: "../../Core/DesignSystem"), .package(path: "../../Core/UICore") ],
    targets: [
        .target(name: "CartContracts", dependencies: ["Kernel"]),
        .target(name: "CartDomain",    dependencies: ["Kernel", "CartContracts"]),                         // PURE: no UI/platform imports
        .target(name: "CartData",      dependencies: ["CartDomain", "SDK", "Kernel"]),
        .target(name: "CartUI",        dependencies: ["CartDomain", "DesignSystem", "UICore"],
                swiftSettings: [.defaultIsolation(MainActor.self)]),                                      // UI code is @MainActor by default
        .testTarget(name: "CartDomainTests", dependencies: ["CartDomain"]),                              // Swift Testing
        .testTarget(name: "CartUITests",     dependencies: ["CartUI"]),                                  // snapshot/ViewModel tests
    ],
    swiftLanguageModes: [.v6]
)
```

## Appendix B — Tuist manifest for the three variants (sketch; verify the Tuist API version you adopt)

```swift
import ProjectDescription

func app(_ name: String, bundleId: String, features: [TargetDependency], entitlements: Path) -> Target {
    .target(
        name: name, destinations: .iOS, product: .app, bundleId: bundleId,
        deploymentTargets: .iOS("18.0"),
        infoPlist: .extendingDefault(with: [
            "UILaunchScreen": [:],                                           // code-defined launch screen, no storyboard
            "NSLocationWhenInUseUsageDescription": "…specific, honest purpose…",
        ]),
        sources: ["Apps/\(name)/Sources/**"], resources: ["Apps/\(name)/Resources/**"],
        entitlements: .file(path: entitlements),
        dependencies: features,
        settings: .settings(configurations: [.debug(name: "Debug", xcconfig: "Config/Debug.xcconfig"), .release(name: "Release", xcconfig: "Config/Release.xcconfig")])
    )
}

let project = Project(
    name: "Heimdall",
    targets: [
        app("Game",   bundleId: "com.example.heimdall.game",   features: [.package(product: "GameMapUI")],  entitlements: "Config/Game.entitlements"),
        app("Market", bundleId: "com.example.heimdall.market", features: [.package(product: "CartUI")],     entitlements: "Config/Market.entitlements"),
        app("Avatar", bundleId: "com.example.heimdall.avatar", features: [.package(product: "AvatarSessionUI")], entitlements: "Config/Avatar.entitlements"),
    ]
)
```

## Appendix C — Device tier from native signals (sketch)

```swift
struct DeviceCapabilities: DeviceCapabilityPort {
    func hintTier() -> DeviceTier {
        let gb = Double(ProcessInfo.processInfo.physicalMemory) / 1_073_741_824
        let gpu = MTLCreateSystemDefaultDevice()
        let apple7 = gpu?.supportsFamily(.apple7) ?? false      // A14-class GPU and newer
        let apple9 = gpu?.supportsFamily(.apple9) ?? false      // A17/M3-class and newer
        switch (gb, apple7, apple9) {
        case (..<3.5, _, _), (_, false, _): return .low         // 3 GB class or pre-A14 GPU family
        case (_, _, true) where gb >= 7:    return .high
        default:                            return .mid
        }
    }
    func thermal() -> AsyncStream<Thermal> {
        AsyncStream { c in
            let t = NotificationCenter.default.addObserver(forName: ProcessInfo.thermalStateDidChangeNotification, object: nil, queue: nil) { _ in
                c.yield(Thermal(ProcessInfo.processInfo.thermalState)) }
            c.onTermination = { _ in NotificationCenter.default.removeObserver(t) }
        }
    }
}
// Memory guard before loading a scene: refuse or step the tier down if os_proc_available_memory() < required budget.
```

## Appendix D — Push delegate, payload and routing (sketch)

```swift
final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    func application(_ app: UIApplication, didFinishLaunchingWithOptions opts: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        UNUserNotificationCenter.current().delegate = self; return true
    }
    func application(_ app: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken token: Data) {
        Task { await environment.devices.upload(tokenHex: token.map { String(format: "%02x", $0) }.joined()) }   // idempotent, includes sandbox/production
    }
    func userNotificationCenter(_ c: UNUserNotificationCenter, didReceive r: UNNotificationResponse) async {
        guard let payload = PushPayload(userInfo: r.notification.request.content.userInfo) else { return }       // versioned schema
        await environment.router.handle(deepLink: payload.deepLink)                                              // allowlist + resume after auth
    }
    func userNotificationCenter(_ c: UNUserNotificationCenter, willPresent n: UNNotification) async -> UNNotificationPresentationOptions {
        environment.foregroundPolicy.options(for: n)                                                            // by category and current screen
    }
}
```

## Appendix E — SSE over `URLSession.bytes` (sketch; parse raw bytes, not `.lines`)

```swift
func stream(url: URL, lastID: String?) -> AsyncThrowingStream<SSEEvent, Error> {
    AsyncThrowingStream { continuation in
        let task = Task {
            var req = URLRequest(url: url, timeoutInterval: 45)               // > server heartbeat (15 s)
            req.setValue("text/event-stream", forHTTPHeaderField: "Accept")
            req.setValue("Bearer \(await session.accessToken())", forHTTPHeaderField: "Authorization")
            if let lastID { req.setValue(lastID, forHTTPHeaderField: "Last-Event-ID") }
            var parser = SSEParser(maxLine: 64 * 1024, maxEvent: 256 * 1024)   // spec-compliant: CR/LF/CRLF, comments, retry, id
            do {
                let (bytes, resp) = try await dedicatedSession.bytes(for: req)
                try validate(resp)
                for try await byte in bytes { for e in parser.feed(byte) { continuation.yield(e) } }
                continuation.finish()
            } catch { continuation.finish(throwing: error) }
        }
        continuation.onTermination = { _ in task.cancel() }
    }
}
```

## Appendix F — Info.plist/entitlements checklist per variant

| Item | Game | Market | Avatar |
|---|---|---|---|
| `NSLocationWhenInUseUsageDescription` | ✔ | optional | ✘ |
| `NSLocationAlwaysAndWhenInUseUsageDescription` + `UIBackgroundModes: location` | only if background play | ✘ | ✘ |
| `NSCameraUsageDescription` | AR | scan/photos | tracking |
| `NSMicrophoneUsageDescription` | optional | optional | ✔ |
| `NSSpeechRecognitionUsageDescription` | optional | optional | if Speech framework |
| `NSFaceIDUsageDescription` | optional | optional | optional |
| `UIBackgroundModes: remote-notification / processing / audio` | remote-notification, processing | remote-notification, processing | remote-notification, processing (+ audio only if needed) |
| Capabilities | Push, Associated Domains, Sign in with Apple, Game Center (opt.), In-App Purchase | Push, Associated Domains, Sign in with Apple, Apple Pay, Live Activities (opt.) | Push, Associated Domains, Sign in with Apple, In-App Purchase |
| `UILaunchScreen` | code-defined (no storyboard) | same | same |
| `PrivacyInfo.xcprivacy` | app + SDKs | app + SDKs | app + SDKs |

## Appendix G — Support matrix template

```markdown
# Support matrix (per release)
| Item | Value |
| Toolchain | Xcode 27.x (Swift 6.4) · App Store floor: Xcode 26 / iOS 26 SDK |
| Deployment target | iOS 18.0 (iOS 17 only with fleet data) |
| Devices | iPhone XS/XR and later on iOS 18; iPhone 11/SE 2+ on iOS 26/27 · Low/Mid/High tiers · device pool: <list> |
| Capabilities by tier | AR, face tracking (TrueDepth), on-device speech, Foundation Models (Apple Intelligence devices only), ProMotion |
| Known limitations per tier | background location, camera tracking, voice features, 3D fidelity |
```
