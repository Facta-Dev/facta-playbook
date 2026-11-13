# PLAYBOOK_MOBILE_ANDROID_KOTLIN.md — Native Android playbook (Kotlin · Jetpack Compose · Game · Market · Avatar)

Architecture and engineering contract for building the **same three-variant
product family** as `PLAYBOOK_FRONTEND_MOBILE.md`, but as a **native Android
app in Kotlin**:

| Variant | What it is | Core capabilities |
|---|---|---|
| **Game** | Location-based 3D game in the style of Pokémon GO | Real-world map, spawns and encounters, 3D creatures, optional AR (ARCore), catching, collection, events, social |
| **Market** | Marketplace in the style of Amazon | Catalog, search and filters, product pages, reviews, cart, checkout, orders and tracking, wishlist, seller tooling |
| **Avatar** | 3D avatar with voice and movement | 3D character, speech in/out, lip-sync, gestures/emotions, optional camera-driven motion, live conversation |

**All three** share: **onboarding, login, registration, forgot/reset password,
push notifications (FCM)**, the same Kotlin SDK, design system, security model
and release pipeline, and all three must run well on **low- and mid-range
Android phones** (including Android Go–class devices), not only flagships.

It extends and must stay consistent with `PLAYBOOK_FRONTEND_MOBILE.md` (same
variants, contracts, budgets, device tiers and flows), `PLAYBOOK_FRONTEND.md`
(MVVM + SDK), `PLAYBOOK_URLS.md` (routes and permissions), `PLAYBOOK_BACKEND.md`
(slices, realtime, idempotency), `PLAYBOOK_SECURITY.md`, `PLAYBOOK_TESTS.md`,
`PLAYBOOK_AUDIT.md`, `PLAYBOOK_CLAUDE.md`, and reuses patterns from
`PLAYBOOK_DESKTOP_QT.md` (realtime state machine, quality governor) and
`PLAYBOOK_CHROME_EXTENSION.md` (managed config, kill switch, diagnostics).

The rules that carry the whole file:

1. **Native means we can do things React Native can't cheaply do**: baseline
   profiles, startup profiles, R8 full mode, platform performance hints,
   thermal/memory APIs, Play Feature/Asset Delivery. Use them; they are the
   reason to go native.
2. **Gradle modules enforce architecture at compile time.** Domain modules are
   pure Kotlin (no Android); a module that doesn't depend on another cannot
   import it.
3. **Unidirectional data flow.** `UiState` down, `UiEvent` up, effects as a
   separate one-shot channel; no business logic in composables.
4. **One variant = one app module = one binary.** The market app must not
   contain a game engine; permissions and native code are scoped per variant.
5. **The main thread is sacred.** Coroutines with injected dispatchers; StrictMode
   in debug; baseline profile and benchmarks in CI.
6. **Design for the floor, enhance upward** with capability tiers and a runtime
   quality governor (same contract as the RN playbook).
7. **The server is authoritative** for game state, prices, payments and
   entitlements; the app is untrusted.
8. **Push is a channel, not a data path.** Minimal payloads, fetch after tap.
9. **No vendor lock-in for irreplaceable assets** (avatars, models, user data).
10. **Measure on real low-end devices** and gate releases on Android vitals-style
    metrics.

> Verification note: status as of November 2026. **Android 17 (API 37)** is out
> (June 2026); **Google Play requires new apps and updates to target API 36
> (Android 16)** from 31 August 2026; **Jetpack Compose 1.12 (August '26)** moves
> to `compileSdk 37` and requires **AGP 9.1.1+**; **Navigation 3 is stable
> (1.0)**; AndroidX libraries require `minSdk 24` since July 2026. Items marked
> **(verify)** depend on exact versions, Play policy dates or library maintenance
> and must be re-checked in the Android developer documentation, the Play Console
> policy center and each library's release notes before adoption. Budgets are
> starting points to calibrate on your own device pool.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Language | **Kotlin 2.x (K2 compiler)**, coroutines + Flow, kotlinx.serialization | Idiomatic, no reflection-heavy serialization (friendly to R8 and low-end startup) |
| D2 | UI | **Jetpack Compose** (Material 3) as the default UI toolkit; Views only for engine surfaces (`SurfaceView`, `AndroidView` interop) | Compose 1.12 reports first-frame times comparable to Views in Google's benchmarks; verify on your Low pool |
| D3 | Architecture | **Hexagonal + vertical slices + UDF (MVVM/MVI)**, enforced by Gradle modules | Same slices as backend and RN playbooks |
| D4 | Variants | **Three thin application modules** (`:apps:game`, `:apps:market`, `:apps:avatar`) depending on shared library modules; **not** product flavors that merely hide code | Compile-time guarantee that a variant can't pull another variant's engine/permissions |
| D5 | Navigation | **Navigation 3** (typed `NavKey` routes, state you own) | Stable 1.0; full back-stack control; routes mirror `PLAYBOOK_URLS.md` |
| D6 | DI | **Hilt** (compile-time graph, KSP) as default; Koin only if KMP is planned | Startup and correctness; avoid kapt |
| D7 | Networking | **OkHttp 5 + kotlinx.serialization** under a generated Kotlin SDK; Retrofit optional | HTTP/2, interceptors, first-party WebSocket and SSE |
| D8 | Persistence | **Room** (SQLite, WAL) + **DataStore** (prefs); secrets via **Android Keystore + Tink** | Room 3 is a KMP-oriented major version: adopt when stable for your needs **(verify)** |
| D9 | Images | **Coil 3** (Compose-first, size-aware) | Memory-safe loading with explicit sizes |
| D10 | Lists | LazyColumn/LazyVerticalGrid with keys + contentType; **Paging 3** for network/DB feeds | Virtualization on the main thread budget |
| D11 | Background | **WorkManager** for deferrable guaranteed work (outbox, uploads); foreground service only where policy-allowed | Doze/standby-friendly |
| D12 | Push | **Firebase Cloud Messaging** via **FCM HTTP v1** from your backend; `POST_NOTIFICATIONS` runtime permission on API 33+ | Section 8 |
| D13 | Auth | First-party email/password + **Credential Manager** (passkeys, passwords, Google) + OIDC/PKCE via **AppAuth + Custom Tabs**; tokens encrypted with Keystore | `PLAYBOOK_SECURITY.md` §4 |
| D14 | Payments | **Stripe Android SDK** (PaymentSheet, Google Pay) for physical goods/services; **Play Billing Library** (current major required by Play) for digital goods | Store rules differ by good type **(verify)** |
| D15 | 3D | Tiered behind a `SceneEngine` port: **2D fallback → Filament (directly or via SceneView) → Godot (Compatibility renderer) → Unity (AR-heavy, high tier only)** | Same tier model as RN; see section 13 |
| D16 | AR | **ARCore** (optional, never required for install); Geospatial API for location-anchored AR | `uses-feature … required="false"` + `com.google.ar.core=optional` |
| D17 | Maps (Game) | **MapLibre Native Android** (or Mapbox SDK); Google Maps Compose only for small marker counts | GL layers for entities |
| D18 | Min/target SDK | `minSdk` **24** (AndroidX/Play services floor; raise with fleet data), `compileSdk` **37** with AGP **9.1.1+** (required by Compose 1.12), `targetSdk` **37** — Play's floor is **36** (since 31 Aug 2026), so 36 is compliant but 37 is the target once the §15.2 Android 17 checklist is green | Verify the current Play target-API rule each year |
| D19 | ABIs | **arm64-v8a** primary; **armeabi-v7a** only where your fleet needs it (many low-end devices still run 32-bit userland) | Smaller downloads; use fleet analytics |
| D20 | Native libs | All `.so` files **16 KB page-size aligned** | Play requirement for native code; enforcement dates have been extended before **(verify current date)** |
| D21 | Optimization | **R8 full mode + Baseline Profiles + Startup Profiles** mandatory for release builds | Target reported ~6% smaller APK from R8 full mode and ~10% faster startup from Baseline Profiles, with ~11% faster startup and ~6% smaller APK after enabling full mode |
| D22 | Build | Gradle Kotlin DSL, **convention plugins** in `build-logic`, version catalog, **dependency verification**, configuration cache | Reproducible, auditable builds |
| D23 | Observability | Crashlytics or Sentry + `ApplicationExitInfo` + JankStats + Perfetto tracing | Section 16 |
| D24 | Distribution | AAB, Play App Signing, staged rollouts, Play Feature Delivery + Play Asset Delivery for heavy modules/assets | Section 18 |
| D25 | Non-GMS devices | Declare a policy: GMS-required (simpler) or GMS-optional (ports with AOSP fallbacks for push/location/integrity) | Affects Huawei-class devices |

### 1.1 AGP 9 migration notes (if you are not yet on it)

AGP 9 compiles Kotlin itself and rejects an explicit `org.jetbrains.kotlin.android`
plugin; it also removes the legacy `applicationVariants` API (use the Variant API
via `androidComponents`). Teams that cannot migrate immediately hold Compose BOM and
a few AndroidX libraries one release back until AGP 9.1.x/`compileSdk 37` is adopted.
Plan this as a dedicated upgrade task with a Low-pool performance check.

---

## 2. The product family

### 2.1 Variant matrix (same as the RN playbook; native specifics added)

| Capability | Game | Market | Avatar |
|---|---|---|---|
| Shared shell (auth, onboarding, push, settings, design system, SDK) | ✔ | ✔ | ✔ |
| Location | Fused Location + geofencing; optional foreground service | Optional approximate | No |
| Camera | Optional (AR via ARCore) | Optional (Google code scanner, CameraX photos) | Optional (CameraX → face/pose models) |
| Microphone | Optional | Optional (voice search) | **Core** (`AudioRecord`, communication mode) |
| 3D rendering | **Core** | Optional (off by default) | **Core** |
| Realtime | **Core** (WebSocket/SSE) | Order status, price alerts | **Core** (WebRTC/WebSocket) |
| Payments | Play Billing (digital) | **Core** (Stripe/Google Pay) | Play Billing (items/subscriptions) |
| Heavy modules | Map SDK + engine (Filament/Godot/Unity) as **dynamic feature** | Stripe, scanner | Engine + audio/WebRTC as **dynamic feature** |
| Target download (Low) | Base ≤ ~100 MB + PAD packs | ≤ ~25–35 MB | ≤ ~80 MB + PAD packs |

### 2.2 Identical across all three

Login, registration, forgot/reset password, onboarding, push registration and
routing, session/token handling, account deletion, consent, i18n, accessibility,
SDK, error taxonomy, correlation IDs, telemetry, kill switch/remote config, design
tokens (see `PLAYBOOK_FRONTEND_MOBILE.md` §2.2 and §7–§8).

### 2.3 May differ

Feature modules, native dependencies (via dynamic feature modules where useful),
permissions and foreground-service types, store listings, theme, onboarding steps.

---

## 3. Device classes, budgets and the quality governor

### 3.1 Device classes

Same `low | mid | high` classes and device-pool rule as `PLAYBOOK_FRONTEND_MOBILE.md` §3.1
(Low ≈ ≤ 3 GB RAM / little-core CPUs / entry GPUs / eMMC; Mid ≈ 4–6 GB; High ≥ 8 GB). Keep
a real-device pool (≥ 2 Low, 2 Mid, 1 High) plus a cloud farm for breadth; include at least one
**Android Go-edition** device.

### 3.2 Budgets (Low-class, release build with baseline profile; starting points)

Native apps should beat the RN numbers; targets below are tighter.

| Metric | Budget |
|---|---|
| Cold start, time to initial display (TTID) | ≤ 2.5 s (Low), ≤ 1.8 s (Mid) |
| Time to fully drawn (TTFD, `reportFullyDrawn`) | ≤ 3.5 s (Low), ≤ 2.5 s (Mid) |
| Slow/frozen frames | Jank rate < 5% in main flows; zero frozen frames in scroll benchmarks |
| Touch response | ≤ 100 ms |
| Shell memory (PSS, no 3D) | ≤ ~120 MB |
| 3D scene memory (total process PSS) | Low ≤ ~300 MB, Mid ≤ ~450 MB (Android 17 enforces RAM-based app memory limits) |
| 3D frame rate | Low: stable 30 fps (resolution scale ≤ 0.75); Mid: 30–60; High: 60 |
| Download size (per-device, AAB) | Market ≤ ~25–35 MB; Game/Avatar base ≤ ~100 MB, assets in PAD |
| Battery (Game active session) | Tracked %/hour on the Low pool; no regression release over release |
| Network | Works on slow 3G; request count and payload budgets per screen |
| Memory growth over 30 min | ≈ 0 |
| Android vitals | User-perceived crash and ANR rates well under Play's bad-behavior thresholds, per device model **(verify current thresholds)** |

### 3.3 Capability detection (native signals)

```text
DeviceTier hint ← ActivityManager.isLowRamDevice(), MemoryInfo.totalMem (reported RAM is below marketing RAM),
                  Build.VERSION.MEDIA_PERFORMANCE_CLASS (API 31+), Build.SOC_MODEL (API 31+), SDK_INT,
                  supported ABIs, GL ES / Vulkan feature levels (PackageManager features),
                  Display refresh rates, ArCoreApk.checkAvailability()
Runtime state   ← PowerManager thermal status + headroom (API 29/30+), isPowerSaveMode, ComponentCallbacks2.onTrimMemory,
                  Window frame metrics / JankStats, ApplicationExitInfo (previous OOM/memory-limiter exits)
Measured        ← first-run micro-benchmark (reference scene p95 frame time), refreshed after OS/app updates
Remote override ← remote config keyed by Build.MODEL / SOC / API level (blocklist or forced tier)
```

Rules: the hint is a **hint**; the **measured** result and the **runtime governor** win. Persist the tier and
re-evaluate after OS/app updates and repeated frame-drop events. Android 17's RAM-based memory limits mean a
memory ceiling is a **correctness** constraint, not just a performance goal: when a process is killed by the
limiter, `ApplicationExitInfo` reports it (reason `OTHER` with a memory-limiter description), so ingest that
signal into telemetry and lower the tier for that device class.

### 3.4 Quality governor

Same `QualityProfile` knobs as `PLAYBOOK_FRONTEND_MOBILE.md` §3.4 (render scale, frame cap, shadows/post,
texture pack, LOD, particles, map layers, camera tracking/AR). Native additions:

* **Thermal**: listen to thermal status changes and, on API 30+, the thermal headroom forecast; step down
  before throttling hits.
* **Performance hints (ADPF)**: for engine threads, use `PerformanceHintManager` sessions so the scheduler gets
  accurate work durations; report actual vs target frame time.
* **Game Mode / frame rate**: respect the user's Game Mode (performance/battery) where available; use
  `Surface.setFrameRate`/`Window` frame-rate hints to avoid rendering above what the screen/budget needs;
  consider frame-pacing (Swappy) for NDK render loops.
* **Memory pressure**: `onTrimMemory` levels trigger cache eviction and scene LOD reduction before the OS acts.
* **Power saver / low-RAM**: force the Low profile and disable AR/camera tracking.
* Step **down** immediately on breach; step **up** only after a sustained healthy period (hysteresis). Expose
  Auto / Battery saver / High quality in settings; remote config and managed policy can force a profile.

---

## 4. Architecture: hexagonal + vertical slices + UDF on Android

### 4.1 Layers mapped to Gradle modules

```text
┌────────────────────────────── one app module (per variant) ──────────────────────────────┐
│ :apps:<variant>  (Application, manifest, Nav3 root, variant config, Hilt entry points)     │
└──────────┬───────────────────────────────────────────────────────────────────────────────┘
           ▼
 :feature:<variant>:<slice>:ui        Compose screens + ViewModels (presentation adapters)
 :feature:<variant>:<slice>:data      Repositories implementing ports (SDK, Room, push, location…)   [adapters/out]
 :feature:<variant>:<slice>:domain    Use cases + ports + domain model (PURE KOTLIN, JVM-only)        [application + domain]
 :feature:<variant>:<slice>:contracts Public API of the slice (DTO-free interfaces/events) for other slices
           ▼
 :shell:*  (identity, notifications, devices, consent, settings)   :core:* (kernel, contracts, sdk, designsystem, …)
```

| Layer / module type | Contains | May depend on | Must not depend on |
|---|---|---|---|
| `domain` (JVM library) | Entities, value objects, rules, **ports**, use cases, `Result`/errors | `:core:kernel`, kotlinx-coroutines | Android SDK, Compose, OkHttp, Room, Hilt annotations beyond `@Inject` constructor (optional) |
| `data` (Android library) | Port implementations: SDK calls, Room DAOs, DataStore, push, location, camera, audio, scene adapters | own `domain`, `:core:*`, Android/Jetpack | `ui` modules |
| `ui` (Android library) | Composables, ViewModels, UiState/UiEvent/UiEffect, navigation entries | own `domain`, `:core:designsystem`, `:core:ui-core` | `data` (reach it only through DI) |
| `contracts` (JVM) | Cross-slice interfaces, event types, route keys | `:core:kernel` | everything else |
| `:core:sdk` (JVM or Android lib) | Generated DTOs + thin SDK over OkHttp | `:core:contracts`, `:core:kernel` | UI, Android UI APIs |
| `:apps:*` | Wiring only | feature modules | business logic |

Rules:

* **Domain modules are pure Kotlin** (`kotlin("jvm")`): the compiler forbids Android imports there.
* `ui` never depends on `data`; Hilt provides implementations at the app module (dependency inversion).
* **Cross-slice access only through `contracts`** (same rule as the backend).
* Add **architecture tests** (Konsist or equivalent) and module-graph checks in CI: forbidden dependencies,
  naming, `@Composable` only in `ui`, no `android.*` in `domain`.
* Android lint + `dependency-analysis` plugin to prevent unused/transitive leakage; module count is a build-time
  trade-off, so keep modules coarse enough (per slice, not per class).

### 4.2 Presentation: UDF with statuses

```kotlin
sealed interface UiStatus { data object Loading; data object Ready; data object Empty; data object Offline
  data class Forbidden(val requiredPermission: String?) : UiStatus; data object NotFound
  data class Error(val correlationId: String?) : UiStatus; data object PolicyBlocked }

@Immutable data class LeadDetailState(val status: UiStatus, val lead: LeadUi?, val canEdit: Boolean, val isMoving: Boolean)
sealed interface LeadDetailEvent { data class MoveToStage(val stage: Stage) : LeadDetailEvent; data object Retry : LeadDetailEvent }
sealed interface LeadDetailEffect { data class ShowMessage(val key: MessageKey) : LeadDetailEffect; data class Navigate(val route: Route) : LeadDetailEffect }
```

* ViewModels expose `StateFlow<State>` (collected with `collectAsStateWithLifecycle`) and a `Channel`-backed
  effect flow; commands via `onEvent`.
* Every screen handles the shared statuses `loading | ready | empty | forbidden | not-found | offline | error | policy-blocked`
  (same as `PLAYBOOK_FRONTEND.md` §4).
* ViewModels depend on use cases and ports (`NavigatorPort`, `NotifierPort`, `PermissionsPort`), never on `Context`
  or composables; use `SavedStateHandle` for restorable inputs.
* **Compose rules for Low devices**: stable/immutable state (`@Immutable`, kotlinx.collections.immutable), strong
  skipping enabled (default with current Kotlin), `key` and `contentType` in lazy lists, `derivedStateOf`, deferred
  state reads (lambda modifiers), no allocation in composition hot paths, `remember` for expensive objects, no
  business logic in composables; review Compose compiler reports in CI for unstable hot paths.

### 4.3 Ports (examples)

```kotlin
interface LeadRepositoryPort { suspend fun byId(id: LeadId): Outcome<Lead>; suspend fun list(q: LeadQuery): Outcome<LeadPage>
  suspend fun updateStage(id: LeadId, stage: Stage, expected: Version, key: IdempotencyKey): Outcome<Lead> }
interface PushPort          { suspend fun permissionState(): PermState; suspend fun registerDevice(): Outcome<DeviceReg>; suspend fun unregisterDevice() }
interface SecureStorePort   { suspend fun get(key: String): ByteArray?; suspend fun put(key: String, value: ByteArray); suspend fun remove(key: String) }
interface LocationPort      { fun updates(opts: LocOpts): Flow<Fix>; suspend fun startBackground(opts: BgLocOpts); suspend fun stopBackground() }
interface DeviceCapabilityPort { suspend fun tier(): DeviceTier; fun capabilities(): Capabilities; val thermal: StateFlow<Thermal> }
interface SceneEnginePort   { suspend fun init(cfg: SceneConfig); suspend fun load(scene: SceneSpec); fun send(cmd: SceneCommand)
                              val events: Flow<SceneEvent>; fun setQuality(q: QualityProfile); suspend fun dispose() }
interface RealtimePort      { fun start(cfg: RtConfig); fun stop(); fun setTopics(t: Set<String>); val state: StateFlow<RtState>; val events: Flow<RtEvent> }
```

`Outcome<T>` is a sealed result (`Success | Failure(AppError)`), errors are values; **always rethrow
`CancellationException`**. Platform/library types stay inside `data`.

### 4.4 Concurrency

* Structured concurrency; `viewModelScope`/`lifecycleScope`; injected dispatchers (`@IoDispatcher`, `@DefaultDispatcher`,
  `@MainImmediate`); main-safe suspend APIs; no `GlobalScope`; no blocking calls on Main (StrictMode thread policy
  in debug: disk, network, slow calls).
* Low-end CPUs have 2–4 usable cores: cap parallelism (`limitedParallelism`) for CPU-bound work; avoid launching
  hundreds of coroutines for tiny tasks; batch.
* Flows: `stateIn(WhileSubscribed(5_000))`, `conflate` for high-rate sensor streams, `debounce` for search.

### 4.5 State, data, offline and outbox

| Concern | Choice |
|---|---|
| Server data | Repository + Room cache (projection) + `Flow`; stale-while-revalidate |
| Prefs/flags | DataStore (Preferences/Proto) |
| Secrets | Keystore-backed AES-GCM via Tink (never SharedPreferences/DataStore plaintext) |
| Feeds | Paging 3 with `RemoteMediator` |
| Mutations offline | **Outbox** table + WorkManager sync with `Idempotency-Key`, backoff, per-entity ordering (mirrors the backend idempotency contract) |
| Conflicts | Typed results (`409`/version) surfaced to the ViewModel for resolution |
| Migrations | Room schema export + migration tests from every released version; destructive fallback forbidden |

The local DB is a **projection**; the backend is the authority.

---

## 5. Project layout, variants and build logic

```text
android/
├── settings.gradle.kts  build.gradle.kts  gradle/libs.versions.toml  gradle/verification-metadata.xml
├── build-logic/convention/            # plugins: app, library, jvm-library, compose, hilt, detekt, baseline-profile, signing, variant-config
├── apps/{game,market,avatar}/         # thin application modules (variant ID, manifest, Nav3 root, feature wiring)
├── core/
│   ├── kernel/        # pure Kotlin: Outcome, ids, errors, time, strong types
│   ├── contracts/     # DTOs, error codes, notification payloads, realtime envelope, performance-script schema, route table
│   ├── sdk/           # Kotlin SDK (OkHttp + kotlinx.serialization), generated from OpenAPI
│   ├── designsystem/  # tokens + Compose components + variant themes
│   ├── ui-core/       # UiStatus, base ViewModel helpers, navigation helpers, a11y utils
│   ├── device/        # DeviceCapabilityPort, QualityGovernor, remote overrides
│   ├── telemetry/     # logging+redaction, crash, performance markers, diagnostics bundle
│   ├── security/      # Keystore/Tink wrappers, biometrics, integrity client
│   ├── scene/         # SceneEnginePort + scene-2d, scene-filament, scene-godot, scene-unity (separate modules)
│   └── testing/       # fakes, rules, builders, fixtures
├── shell/{identity,notifications,devices,consent,settings}/{domain,data,ui,contracts}
├── feature/
│   ├── game/{map,spawns,encounters,catching,inventory,collection,events,social,shop,safety}/…
│   ├── market/{catalog,search,product,cart,checkout,orders,reviews,wishlist,seller,support}/…
│   └── avatar/{avatars,sessions,voice,performance,tracking,wardrobe,safety}/…
├── baselineprofile/  benchmark/       # profile generators + macrobenchmarks per variant
└── tests/<slice>/android/{e2e,contract,perf,security,a11y}/   tests/_platform/{architecture,device-perf,push,auth,store-readiness}/
docs/{adr,support-matrix.md,dependency-register.md,permissions-register.md,device-pool.md,runbooks/}
```

**Scope of the shipped template:** the template that ships with this playbook is the **minimal instantiation** of the graph above, not the recommended production
graph: one variant (a single `:app`, not `:apps/{game,market,avatar}`), one slice (`:feature:notes` split into `contracts`/`domain`/`data`/`ui`) and the shared
modules that slice needs (`:core:kernel`, `:core:ui-core`), plus `:evals` for the contract tests. The full variant grid, the `shell/*` modules and the remaining
`:core/*` modules are the **growth path**: add them when the product needs them, one consumer at a time. The rules in §4.1 hold for the minimal instantiation —
the module boundary is the enforcement — so the template proves the architecture without shipping modules nothing consumes.

**Tests placement ADR:** Gradle requires JVM and instrumented tests inside each module (`src/test`, `src/androidTest`);
colocate **unit/ViewModel/UI-component** tests there. Cross-module suites (e2e, contract, device performance, security,
accessibility audits, architecture) live under `tests/…/android/` to match `PLAYBOOK_TESTS.md` and are wired as separate
Gradle modules/tasks.

### 5.1 Build rules

* **Convention plugins** define SDK levels, Kotlin/JVM targets, Compose, lint, Detekt, R8, baseline profile and signing once;
  modules stay declarative.
* **Version catalog** (`libs.versions.toml`) is the single source of versions; use the Compose BOM; no dynamic versions.
* **Dependency verification** (`verification-metadata.xml` with checksums/signatures) and dependency locking; Renovate/Dependabot
  with a **release-age cooldown** for new versions; Gradle wrapper validation in CI.
* **Configuration cache + build cache** on; parallel builds; KSP instead of kapt.
* **Per-variant manifests** merge only permissions/services declared by modules that variant includes; CI asserts the merged
  manifest and native libs against an allowlist per variant (permission diff vs register).
* **R8**: full mode, resource shrinking, `isDebuggable=false`; keep rules per library in consumer rules; mapping files archived.
* **Baseline Profile + Startup Profile** modules per variant, generated in CI on real devices and committed/artifacted; refresh on every
  release candidate and after major UI changes.
* **Dynamic feature modules** (Play Feature Delivery) for heavy engines (Godot/Unity/AR) and rarely used features; PAD for assets.
* **16 KB alignment** check in CI for every native library (e.g., verify ELF `LOAD` segment alignment); run a test on a 16 KB-page emulator.
* **Full JDK, pinned.** *Toolchain reality:* Gradle runs on `JAVA_HOME`, or on `java` from `PATH` when `JAVA_HOME` is unset, and AGP compiles with the `javac` of that
  same installation — so a JRE-only default JVM fails the build with `does not provide the required capabilities: [JAVA_COMPILER]`. Pin a **full JDK (17+)**, and
  verify that `bin/java` and `bin/javac` come from the same installation: a `java` from one JDK and a `javac` from another fails the same way. The build entry point
  — the CI job and the developer target — resolves such a JDK and exports `JAVA_HOME` before the first `./gradlew` call. A Kotlin `jvmToolchain` does not replace
  that: the toolchain drives compilation, not the plugin that runs the build.
* **Every shipped linter runs in the gate.** Android Lint, Detekt and the architecture checks are part of the PR gate, not side commands nobody remembers. A linter the
  project ships but the gate does not run finds nothing until a human review finds it for it.
* **The instantiation script validates its arguments before it mutates the tree.** A slug and a scope that cannot be package segments are rejected up front; a
  half-applied rename leaves an unbuildable copy behind and the failure surfaces far from its cause.

---

## 6. Navigation, routes and deep links

* **Navigation 3**: routes are `@Serializable` `NavKey` types; back stacks are state you own (`rememberNavBackStack`); screens are
  declared in an `entryProvider`; multiple back stacks for tabs; adaptive scenes (list-detail) for tablets/foldables.
* The **route table** (shared JSON, same as other clients) is the source of truth for path grammar:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]
/app/market/products/{productId}   /app/market/cart   /app/game/map   /app/game/encounters/{encounterId}
/app/avatar/sessions/{sessionId}   /app/notifications/preferences
Session-less: /onboarding  /login  /register  /forgot-password  /reset-password  /verify-email
```

* A **parity test** asserts that the Kotlin `NavKey` set and `RouteBuilders` match the route table; no hand-written path strings.
* **App Links**: `https://<domain>/app/…` with `android:autoVerify="true"` and `assetlinks.json` per variant; optional custom scheme
  `<scheme>://app/…`. The intent filter delivers into a `DeepLinkParser` (allowlist of routes, validated params, size limits).
* **Synthetic back stack** from notification/deep link taps (Up navigation lands on a sensible parent) built from the route hierarchy.
* Guards (`RequireAuth`, `RequireUserType`, `RequirePermission`) shape UX only (redirect to `/login`, `/403`, allowed home); **the server
  authorizes every call**. Deep links received while signed out are queued and resumed after login/bootstrap.
* Deep-link and notification parameters are **untrusted**; never perform state-changing actions without UI confirmation.

---

## 7. Shared platform features: onboarding, login, registration, forgot password

The **backend contract, UX rules and flows are identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §7 (endpoints under
`/api/v1/public/auth/*`, device registration at `/api/v1/app/devices`, consent, account deletion, generic forgot-password
responses, single-use reset tokens, session revocation after reset). Android-specific implementation:

### 7.1 Onboarding

3–5 skippable screens driven by remote config with bundled fallback; value first, permissions later (primers at point of use);
variant-specific steps (Game: safety + age gate; Market: country/language/interests; Avatar: avatar choice, voice consent, AI disclosure).
Low-end: static WebP/AVIF assets, no looping heavy animation, honor `ANIMATOR_DURATION_SCALE`/reduce-motion, large font scale support.

### 7.2 Registration

Minimal fields; password policy feedback; **autofill/Credential Manager integration** (`ContentType.Username/Password/NewPassword` semantics
in Compose); versioned consent capture; age gate where required (consider Play's age-signal facilities where available **(verify)**);
email/phone verification via **App Link** or OTP. **Do not read SMS with SMS permissions**: Android 17 delays SMS-delivered OTP access
for apps that are not the intended recipient, so use the **SMS Retriever** or **SMS User Consent** APIs (or email OTP).
Abuse controls: server rate limits, **Play Integrity** verdict (standard request) as a risk signal, CAPTCHA only on high risk.

### 7.3 Login

* **Credential Manager** (`androidx.credentials`) as the single entry for **passkeys**, saved passwords and **Sign in with Google**
  (`GetGoogleIdOption`); fall back to the first-party form. Compose 1.12 adds integration with Credential Manager **(verify API surface)**.
* Enterprise SSO: **OIDC Authorization Code + PKCE** via **AppAuth-Android** and Custom Tabs; no embedded WebView logins.
* MFA/step-up for sensitive actions (payments, password change, deletion); biometric app lock via `BiometricPrompt` with a
  Keystore-bound key (`setUserAuthenticationRequired`) gating the refresh token; biometrics never replace server authentication.
* Throttling is server-side; the client shows generic errors and `Retry-After`.

### 7.4 Session and token storage

* Access token in memory; **refresh token encrypted with Tink AEAD whose key lives in Android Keystore** (StrongBox when available);
  DataStore/file holds only ciphertext. `android:allowBackup="false"` and `dataExtractionRules`/`fullBackupContent` exclude token stores.
* **Keystore hygiene (Android 17):** the platform now caps the number of Keystore keys per app (a much larger cap applies to apps
  targeting older SDKs); create a **small, fixed set of keys** (e.g., one master key per purpose) rather than per-item keys.
* Single-flight refresh (a `Mutex` in the OkHttp `Authenticator`), rotation with server-side reuse detection, immediate wipe on sign-out/suspension.

### 7.5 Forgot password and reset

Same flow as the RN playbook: request screen with **uniform response** and cooldown → email with a **single-use short-lived App Link**
(`https://<domain>/reset-password?token=…`) and/or code → reset screen validates the token on submit → all sessions revoked → login.
Handle cold start from the link, expired/used tokens, offline, IME insets. Never log or persist the token.

### 7.6 Permission primers (Android specifics)

| Permission | Mechanism | Notes |
|---|---|---|
| Notifications (API 33+) | `POST_NOTIFICATIONS` via `ActivityResultContracts.RequestPermission` | Show a primer first; after repeated denial the system stops prompting → deep-link to app notification settings |
| Location | Foreground first (`ACCESS_COARSE/FINE_LOCATION`), **background (`ACCESS_BACKGROUND_LOCATION`) only after** foreground is granted and only if needed | Two-step flow required; Play declaration + justification |
| Camera | `CAMERA` (CameraX/ARCore) | Prefer the Photo Picker for images (no storage permission) |
| Microphone | `RECORD_AUDIO` | Show a live-mic indicator in-app |
| Activity recognition (Game safety/anti-cheat) | `ACTIVITY_RECOGNITION` | Optional; degrade gracefully |

Each denial has a degraded path (browse-only map, text chat, no AR).

### 7.7 Account lifecycle

In-app account deletion (initiate → re-authenticate → grace period → confirmation) plus the web deletion path required by Play's data-deletion
policy **(verify)**; cascades server-side to devices/push tokens, avatar assets, location history, caches; data export and consent management in
settings; terms/privacy version bumps trigger re-consent.

---

## 8. Push notifications (FCM, all variants)

### 8.1 Architecture

```text
App (FirebaseMessagingService) ──token──► POST /api/v1/app/devices {platform:"android", token, provider:"fcm",
                                           appVariant, versionCode, locale, tz, notificationsEnabled, channelStates}
Backend `notifications` slice → provider adapter (FCM HTTP v1; OAuth2 service account) → delivery receipts / dead-token pruning
```

Same backend contract and payload as `PLAYBOOK_FRONTEND_MOBILE.md` §8.2 (`v`, `type`, `category`, `deepLink`, `correlationId`,
`collapseKey`, `ttl`, `priority`, minimal `data`). The legacy FCM endpoint is retired; use **FCM HTTP v1**.

### 8.2 Client implementation

* `FirebaseMessagingService`: `onNewToken` → enqueue token upload with **WorkManager** (retry/backoff, only if logged in);
  `onMessageReceived` → validate against the versioned schema, build a notification, return quickly (hand heavy work to WorkManager;
  you have only a short window).
* **Channels** (API 26+) per category: `transactional`, `account-security`, `game-events`, `social`, `avatar`, `promotions`
  (promotions **off until opted in**). Create channels at app start; changing importance later requires a new channel ID.
* **Permission**: on API 33+, request `POST_NOTIFICATIONS` at a contextual moment (first order, first catch, first avatar chat) after a primer;
  report permission and channel states to the server.
* **Tap handling**: `PendingIntent` to an **Activity directly** (Android 12+ forbids notification trampolines via services/receivers), with
  `FLAG_IMMUTABLE`; the Activity parses the `deepLink`, validates it against the route allowlist, builds a synthetic back stack, and routes
  after auth/bootstrap. **No state-changing action from the payload.**
* **Lock-screen privacy**: no PII in titles/bodies; `setPublicVersion` for sensitive content; details are fetched after the tap.
* **Rich content** (images, `MessagingStyle` for chat-like avatar sessions): images loaded with Coil under strict size/time budgets; skip on Low tier.
* **Token lifecycle**: register after login and on every app start; `onNewToken`; `deleteToken()` + server unregister on logout/account deletion;
  server prunes on `UNREGISTERED`.
* **Message types**: *notification* messages for user-visible alerts; *data* messages for sync hints only. High-priority data messages that don't result
  in a user-visible notification can be deprioritized by the system, so use priority sparingly.

### 8.3 Low-end and OEM realities

Aggressive OEM battery managers kill background work and delay messages. Keep payloads small, avoid depending on background execution, never request
battery-optimization exemptions (restricted by policy), offer a one-time in-app guide to the OEM's settings when delivery fails repeatedly, and always
**reconcile on foreground** via API/realtime. Test with Doze/standby (`dumpsys deviceidle`, standby buckets) and on the OEM devices in your pool.

### 8.4 Server-side

Device registry, preferences, templates/i18n, scheduling by timezone, rate limits and collapse, idempotent send jobs, delivery/open receipts, dead-token pruning,
audit trail for transactional/security notifications, per-category kill switch (`PLAYBOOK_BACKEND.md` batch layer with Redis/Dragonfly coordination).

---

## 9. Library selection (works on low- and mid-range devices)

### 9.1 Selection rules

1. **Prefer Jetpack/first-party libraries and Kotlin-first libraries with compile-time processing** (KSP, no runtime reflection).
2. **Every dependency passes a weight check** on the Low pool: APK size, method count, cold-start, memory, and maintenance health
   (releases, issues, bus factor, license). Record it in `docs/dependency-register.md`.
3. **No library without a fallback** if it is unmaintained or GMS-dependent (when you declare GMS-optional).
4. **Pin and verify** (version catalog, dependency verification, cooldown); keep AndroidX/Play services within supported `minSdk` rules.
5. **Native code only when needed**; every `.so` must be 16 KB-aligned and ABI-filtered per variant.
6. **No heavy UI kits**; Material 3 + your design system; avoid runtime-theming engines and image libs that decode on the main thread.

### 9.2 Curated stack

| Area | Library (default) | Low/mid-end notes | Alternatives / cautions |
|---|---|---|---|
| Language/runtime | Kotlin 2.x, coroutines, Flow | K2 compiler | — |
| UI | **Jetpack Compose (BOM 2026.08 = Compose 1.12)**, Material 3 | Baseline profiles + R8 full mode; strong skipping | Views only for engine surfaces |
| Navigation | **Navigation 3** | State you own; adaptive scenes | Nav2 (Navigation Compose) legacy |
| DI | **Hilt (KSP)** | Compile-time graph | Koin for KMP; Metro (compile-time) is an option **(verify maturity)** |
| Network | **OkHttp 5**, `okhttp-sse`, WebSocket; **kotlinx.serialization** | HTTP/2, small footprint, no reflection | Retrofit on top if desired; Cronet only for HTTP/3 needs |
| Images | **Coil 3** | Explicit `size`, memory/disk caches | Glide (more weight) |
| DB | **Room** (KSP) + **Paging 3** | WAL, schema export | Room 3 when stable **(verify)**; SQLDelight for KMP |
| Prefs | **DataStore** | Async, safe | Never SharedPreferences for secrets |
| Crypto | **Tink** + Android Keystore (StrongBox if present) | Small fixed key set | `androidx.security:security-crypto` is deprecated **(verify)** |
| Background | **WorkManager**, App Startup | Constraints, backoff | Foreground services only for policy-allowed cases |
| Auth | **Credential Manager**, **AppAuth-Android**, `androidx.browser` (Custom Tabs), `androidx.biometric`, Google Identity | System UI; passkeys | No embedded WebView login |
| Push | **Firebase Messaging** (+ FCM HTTP v1 server) | Channels, short handlers | UnifiedPush/HMS only if non-GMS support is required |
| Maps (Game) | **MapLibre Native Android** or **Mapbox Maps SDK** | Vector tiles, GL layers, tile cache | `maps-compose` for few markers only |
| Location | **Fused Location (Play services)** + AOSP `LocationManager` fallback; Geofencing API; Activity Recognition | Balanced power/accuracy; batching | Fused unavailable on non-GMS |
| Camera | **CameraX** (+ `ImageAnalysis`) | Disable analysis on Low | Google Code Scanner (`play-services-code-scanner`) avoids camera permission and APK weight for barcodes |
| ML on-device | **MediaPipe Tasks** (face/pose), **ML Kit**, TFLite via LiteRT, ONNX Runtime Mobile (VAD) | Mid/High only; quantized models | Server fallback |
| 3D (default) | **Filament** (Google, GLES3/Vulkan) directly or via **SceneView** (`io.github.sceneview`: Compose nodes, glTF, ARCore) | Native render thread; baked lighting; KTX2 | Verify SceneView version/maintenance and 16 KB alignment |
| 3D (engine) | **Godot** (Android library/export, Compatibility renderer for low-end), **Unity as a Library** (AR Foundation), **libGDX** (lightweight GLES) | Dynamic feature module; PAD for assets | Heavier engines never on Low path |
| AR | **ARCore** (+ Geospatial API: Terrain/Rooftop anchors, Streetscape) | Optional; needs API key + billing; capability-gated | Unity AR Foundation for heavy AR |
| Audio | `AudioRecord` + platform `NoiseSuppressor`/`AcousticEchoCanceler`, **AudioTrack** (low-latency), **Oboe** (C++) if needed, **Media3** for media | Communication mode + audio focus | — |
| Realtime voice | **LiveKit Android SDK** or `libwebrtc` wrappers **(verify maintenance)** | Opus, echo cancel | WebSocket streaming fallback |
| STT/TTS | Cloud streaming; Android `SpeechRecognizer` (on-device mode where available); `TextToSpeech` fallback | On-device heavy on Low | Lip-sync needs timing data (section 12) |
| Payments | **Stripe Android SDK**, **Google Pay**; **Play Billing Library** (current major) | PCI scope minimized | RevenueCat optional |
| Search | Backend (Algolia/Typesense/Meilisearch) | Debounce, cached suggestions | — |
| i18n | Android resources, per-app language (`LocaleManager`) | `localeConfig`, pseudo-locales | — |
| Observability | **Crashlytics** or **Sentry Android**, **JankStats**, `androidx.tracing`, `ApplicationExitInfo` | Sampling tuned for Low | Timber + redaction |
| Remote config | Firebase Remote Config / GrowthBook | Device-tier overrides | — |
| Testing | JUnit4/5, **Turbine**, **kotlinx-coroutines-test**, MockK/**fakes**, Compose UI test, Robolectric, **Roborazzi/Paparazzi** or Compose Preview Screenshot Testing, **Macrobenchmark**, **Maestro/UI Automator** | Real-device pool | Robolectric support for newest API levels lags **(verify)** |
| Quality | **Detekt (+ Compose rules)**, **ktlint/Spotless**, Android Lint, **Konsist**, LeakCanary (debug), StrictMode | CI gates | — |
| Asset tooling | **gltf-transform/gltfpack, toktx (KTX2)**, meshopt/Draco | CI-enforced budgets | — |

### 9.3 Avoid on low-end

WebView-hosted games/3D; image loading without target sizes; reflection-heavy serialization/DI at startup; many simultaneous animated layers,
blur/shadow stacks and full-screen alpha; per-frame allocation in Canvas/draw code; unbounded caches; large native engines on the Low path;
polling timers; `AsyncTask`/`Handler` soup instead of coroutines; kapt.

---

## 10. Variant A — Game (location-based 3D, Pokémon-GO-style)

Gameplay, server-authoritative design, anti-cheat, economy and compliance are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §10 (server owns spawns, RNG,
outcomes, inventory; client sends intents; plausibility checks; Play Integrity as a signal; location privacy). Android-native specifics:

### 10.1 Slices

`map`, `spawns`, `encounters`, `catching`, `inventory`, `collection`, `events`, `social`, `shop`, `progression`, `safety`; permissions like
`game.encounters.create`, `game.inventory.read`; backend slice `game`.

### 10.2 Map and world

* **MapLibre/Mapbox GL layers** (symbol/circle/fill-extrusion fed by GeoJSON sources) for spawns, POIs and the player marker; no per-entity Views;
  offline tile cache for the player's area; cap zoom/pitch; disable 3D buildings and dense labels on Low; simplified style per tier.
* **Viewport queries + realtime events** (`GET /api/v1/app/game/map/nearby?bbox=…&cursor=…` + `game.spawn.created/expired`), snapshot + stream.
* Entities on the map are 2D sprites/billboards by default; 3D appears in the encounter screen and optionally on higher tiers.

### 10.3 Location, battery and background rules

| Mode | API | Notes |
|---|---|---|
| Foreground, high accuracy | Fused `LocationRequest` (priority high), adaptive interval by speed | Map open |
| Foreground, balanced | `PRIORITY_BALANCED_POWER_ACCURACY`, min distance | Walking |
| Geofences | Geofencing API (limited count), significant-change patterns | Cheapest; OS-driven |
| Background (optional) | **Foreground service with type `location`** (`FOREGROUND_SERVICE_LOCATION` permission) + `ACCESS_BACKGROUND_LOCATION` | Visible notification; Play declaration and review; offer foreground-only play by default |

Rules: batch uploads (deltas, not streams), pause when stationary or low battery, respect thermal/power-saver state, use Activity Recognition to detect
vehicle movement (safety warnings and anti-cheat signal), `Location.isMock` as a **signal only**, verify the battery impact on the Low pool each release.
Android 17 includes location-privacy changes: re-verify permission flows and background behavior on API 37 **(verify details)**.

### 10.4 3D encounter and AR

* **Tiers**: T0 sprite-sheet/turntable flipbook; T1 Filament low-poly with baked lighting at 30 fps; T2 Godot Compatibility for richer scenes; T3 Unity/ARCore
  for AR-heavy high-end (section 13).
* **Render surface**: `SurfaceView` (hardware overlay, lowest overhead) embedded in Compose (`AndroidView` / `AndroidExternalSurface`); avoid `TextureView` on Low;
  tie engine lifecycle to the Android lifecycle (pause on `ON_STOP`, release on dispose).
* **AR is optional and capability-gated**: declare `<uses-feature android:name="android.hardware.camera.ar" android:required="false"/>` and
  `<meta-data android:name="com.google.ar.core" android:value="optional"/>` so non-AR devices can still install; check `ArCoreApk.checkAvailability()` at runtime;
  **Geospatial API** needs `ACCESS_FINE_LOCATION`, an ARCore API key restricted to your package/signing certificate, and billing enabled on the Cloud project;
  fall back to the non-AR encounter when unavailable, thermal-limited or on Low tier.
* Throw/catch input → intent to the server; the engine renders; only events cross to the shell (`encounter.thrown`, `encounter.result`).
* **Game platform APIs**: Game Mode / Game State, ADPF performance hints, frame pacing; optional **Play Games Services** (achievements, leaderboards, cloud save) as a
  provider behind a port.

### 10.5 Assets and performance

Base APK + **Play Asset Delivery** packs (install-time for core, fast-follow/on-demand per region/creature set) with **texture compression format targeting**
(ASTC/ETC2 per device); prefetch on Wi-Fi; memory ceilings per tier enforced by the governor; unload encounter assets on exit; 20-minute soak on the Low pool (temperature,
battery, frame time, memory); realtime updates to the map batched at ≤ 10–20 Hz.

### 10.6 Compliance

Digital items through Play Billing with server-side purchase validation and idempotent grants; odds disclosure/regional rules for loot-box-style mechanics **(verify)**;
spending limits and parental controls for minors; moderation/reporting for any user-generated names or chat; safety reminders and speed warnings.

---

## 11. Variant B — Market (Amazon-style)

Slices, flows, server-authoritative pricing, idempotent checkout, search, reviews and compliance are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §11. Android specifics:

* **Lists**: `LazyColumn`/`LazyVerticalGrid` with `key` + `contentType`, fixed aspect-ratio tiles, **Paging 3** (+ `RemoteMediator` with Room for offline-first feeds),
  prefetch distance tuned for Low devices; shimmer/skeleton placeholders; stable item state (`@Immutable`).
* **Images**: Coil 3 with the **exact display size** and CDN-resized WebP/AVIF variants, memory cache cap proportional to RAM class, disk cache quota; placeholders (blurhash/thumbhash).
* **Home rails**: server-driven *layout* using a typed set of native rail components (no arbitrary remote code).
* **Search**: debounced `Flow` (`debounce(250)`), cached suggestions/recents; **barcode** via Google code scanner (no camera permission, no bundled model), voice search optional.
* **Checkout**: server computes totals/taxes/shipping; **Stripe PaymentSheet** + Google Pay for physical goods; PCI scope minimized (no raw card data); `Idempotency-Key` on order creation;
  `FLAG_SECURE` on payment screens; **Play Billing** (current required major) for digital goods; address autofill.
* **Photos for reviews**: Photo Picker (no storage permission), compress/resize on-device, strip EXIF, upload via WorkManager.
* **Seller mode** (`/app/market/seller/…` or `{userType}=partner`): role-gated, listing creation with image pipeline, inventory, orders, payouts onboarding via the provider's hosted flow.
* **Performance**: avoid blur/large shadows on cards, flatten hierarchies, avoid nested scrollables, `Modifier` lambdas for animated reads, baseline profile covering home → search → product → cart → checkout journeys.
* **3D/AR product view**: off by default; lazy dynamic feature module with Filament + a 2D fallback, Mid/High only.

---

## 12. Variant C — Avatar (3D, voice and movement)

Slices (`avatars`, `sessions`, `voice`, `performance`, `tracking`, `wardrobe`, `safety`), avatar formats and sourcing (glTF/GLB + VRM 1.0, provider-agnostic, selfie data as sensitive),
performance-script contract, latency budgets, privacy and AI disclosure are **identical** to `PLAYBOOK_FRONTEND_MOBILE.md` §12. Android-native implementation:

### 12.1 Rendering

* **Filament** (directly or via SceneView) for the avatar viewer: glTF/GLB with meshopt/Draco, KTX2 textures; **VRM 1.0** support through an adapter that reads the glTF
  `VRMC_vrm` extension (humanoid bone map, expressions) and drives Filament **morph targets** (`RenderableManager` morph weights) and skeleton transforms. There is no first-party
  Android VRM runtime, so budget for this adapter or choose Godot with a VRM add-on **(verify)**.
* Low tier: **2D fallback** (pre-rendered sprite avatar with audio-driven mouth frames) or a very low-poly model; Mid: Filament at 30 fps cap; High: richer lighting/expressions.
* Pause rendering when off-screen/backgrounded; animation update stays on the engine side; Compose sends high-level commands.
* Budgets per tier as in `PLAYBOOK_FRONTEND_MOBILE.md` Appendix E (Low ≤ ~10k tris, ≤ 25 bones, 512 px textures, 10–15 morph targets).

### 12.2 Voice pipeline (Android specifics)

```text
Mic → AudioRecord (VOICE_COMMUNICATION source) + NoiseSuppressor/AcousticEchoCanceler/AGC (where present) + VAD
   → STT (cloud streaming | SpeechRecognizer on-device where available) → text → backend conversation service (LLM + safety)
   → performance script → TTS (cloud with viseme/alignment timestamps | TextToSpeech fallback) → AudioTrack (low-latency) + timeline
   → PerformancePlayer (audio clock is master) → SceneEngine (visemes, gestures, emotion)
```

* **Audio session management**: `MODE_IN_COMMUNICATION`, audio focus handling (calls, other apps), route changes (Bluetooth/wired/speaker) with
  `AudioManager` communication-device APIs on newer releases, `AudioTrack` with a low-latency performance mode, `Oboe` only if you need NDK-level latency.
* **Transport**: WebRTC (LiveKit or equivalent) for real-time audio; WebSocket streaming as the simpler fallback; adaptive bitrate; degrade to text chat.
* **Latency**: stream STT partials, LLM tokens and TTS chunks; barge-in (stop playback when the user speaks); measure end-to-end on the Low/Mid pool.
* **Foreground service type `microphone`** only if background voice is a real requirement; default is foreground-only conversation. Show an in-app/privacy-indicator-consistent live-mic cue.
* **Camera-driven movement (optional, Mid/High)**: CameraX `ImageAnalysis` (low resolution, 15–30 fps) → MediaPipe Face/Pose Landmarker → mapped to VRM expressions; disable on Low, on thermal/battery limits, and when
  the app is backgrounded; raw frames never leave the device (derived parameters only, with consent).
* **Lip-sync fallbacks** by quality/cost: TTS visemes → server phoneme alignment → on-device amplitude estimation (Low tier).
* **Privacy/AI**: AI disclosure, safety filters server-side, no recording by default, retention controls, no voice cloning without explicit revocable consent, selfie-to-avatar data deleted after generation.

### 12.3 Sessions and entitlements

Session lifecycle via `POST /api/v1/app/avatar/sessions`, server-enforced usage limits and entitlements (STT/TTS/LLM cost controls), reconnect/resume, push for scheduled sessions,
wardrobe items as digital goods via Play Billing with server-validated entitlements.

---

## 13. Scene engines, 3D assets and delivery

### 13.1 `SceneEnginePort` and tiers (same model as RN)

| Tier | Module | When |
|---|---|---|
| **T0** | `:core:scene:scene-2d` (Compose Canvas/`SurfaceView` sprites, flipbooks) | Low devices, safe mode, thermal emergency, unsupported GPU |
| **T1** | `:core:scene:scene-filament` (Filament / SceneView) | Mid 3D viewers, encounters, avatars |
| **T2** | `:core:scene:scene-godot` (Compatibility renderer for low-end GLES3; Mobile renderer on capable GPUs) | Game scenes needing physics/scripting |
| **T3** | `:core:scene:scene-unity` (AR Foundation) / ARCore via SceneView | AR-heavy, high tier only |

* Adapters implement the same commands/events; scenes are **data**, not shell logic. Fallback chain on init failure or repeated frame-time breach: step down a tier (persisted) and report.
* Engines run on **their own render thread**; the shell communicates by message passing (commands in, events out), never per-frame bridging.
* Engine modules are **dynamic feature modules** (on-demand) where possible; lazily loaded on scene entry, disposed on exit; a variant without 3D contains none of them.
* **WebView 3D / HTML5 games** are not allowed on the Low/Mid path.

### 13.2 Engine notes

* **Filament**: physically based, Vulkan and OpenGL ES backends, Kotlin/Java API, glTF loader (gltfio), morph targets, skinning; verify the chosen version's 16 KB alignment and size.
* **SceneView**: Compose-native nodes over Filament + ARCore (Stable on Android per its docs); pin versions and review maintenance; keep your own `SceneEnginePort` so it stays replaceable.
* **Godot**: embed as an Android library/fragment (verify the current embedding mechanism and export tooling); Compatibility renderer targets OpenGL ES 3 for low-end.
* **Unity as a Library**: single instance, large footprint; AR-first high tier only; isolate in a dynamic feature module.
* **libGDX**: very light GLES engine (Kotlin-friendly) if you want a lean 2D/low-poly game core on the Low tier.

### 13.3 Asset pipeline (CI-enforced) and delivery

1. DCC → glTF/GLB/VRM. 2. **gltf-transform/gltfpack**: simplification, **LOD chains**, meshopt/Draco, animation quantization, dedupe/prune.
3. **KTX2 (Basis)** → ETC2/ASTC per tier/resolution; mipmaps; atlasing. 4. **glTF-Validator + budget checks** (triangles, bones, materials, textures, draw calls) fail the build.
5. **Play Asset Delivery** packs: install-time (core), fast-follow, on-demand; **texture compression format targeting**; or your CDN with a signed manifest and hash verification.
6. Resumable downloads, Wi-Fi preference, disk quota + LRU eviction, integrity check, rollback to the previous pack.

### 13.4 Memory discipline

Texture memory dominates: GPU-compressed formats, tiered resolutions, shared materials, unload on exit, no duplicate CPU copies of models, respond to `onTrimMemory`, and keep the process below the
tier ceiling **and below the Android 17 RAM-based limit for the device class**.

---

## 14. Realtime, connectivity and background behavior

* Realtime follows `PLAYBOOK_DESKTOP_QT.md` §8 / `PLAYBOOK_FRONTEND_MOBILE.md` §14: `RealtimeMode = auto | websocket | sse | polling | off`, snapshot + stream, dedupe by event ID, resume tokens,
  resync after gaps, per-topic authorization server-side.
* **OkHttp provides first-party WebSocket and SSE** (`okhttp-sse` `EventSources`): use them with a dedicated client (read timeout longer than the server heartbeat, e.g., 45 s with a 15 s heartbeat);
  token in the `Authorization` header (never in the URL); conformance-test the SSE handling against the shared fixture suite.
* **Lifecycle**: connect while the app is in the foreground (`ProcessLifecycleOwner`), disconnect on background; observe connectivity with `ConnectivityManager.NetworkCallback`; backoff with jitter; push is the wake-up channel.
* **Background work**: WorkManager for deferrable work (outbox sync, uploads, asset prefetch) with constraints (unmetered, charging, battery-not-low); no exact alarms; foreground services only for policy-allowed
  cases with the correct **foreground-service type** and permission (e.g., `location` for Game background tracking), a visible notification, and a Play declaration.
* **Doze/App Standby**: design for no background execution by default; test with `dumpsys deviceidle` and standby buckets.
* **Network adaptation**: detect metered/slow connections (`NetworkCapabilities`), lower image quality, disable prefetch, defer large packs to Wi-Fi.
* Android 17: re-test local-network access, loopback restrictions and the new lock-free `MessageQueue` behavior (reflection-based hacks on it will break).

---

## 15. Security, privacy and Play compliance

### 15.1 Baseline (OWASP MASVS-style)

| Area | Controls |
|---|---|
| Storage | Secrets only via Keystore + Tink; `allowBackup=false`/`dataExtractionRules`; no secrets in DataStore/Room plaintext/logs; encrypt sensitive Room fields (SQLCipher or field-level AEAD) |
| Network | **Network Security Config**: no cleartext, no user-installed CAs in release (unless required), TLS 1.2+; optional `CertificatePinner` with backup pins, rotation plan and remote kill switch; for apps targeting API 37, Encrypted Client Hello is used by default (configure via `domainEncryption` when needed) |
| Auth | Section 7; rotation; biometric gate; step-up; session list/revoke |
| Components | `android:exported` explicit (default false), verified App Links, immutable `PendingIntent`s, no implicit URI-permission reliance (Android 18 stops implicit grants for share/capture intents; detect with StrictMode), permission-protected broadcasts |
| UI | `FLAG_SECURE` on payment/auth screens, `filterTouchesWhenObscured` against tapjacking, sensitive clipboard flag, hide content in recents where needed |
| Binary | R8 full mode (obfuscation is not security), no secrets/API keys in the APK (use build-time injection + key restrictions), debuggable off, signing keys in Play App Signing + KMS-held upload key |
| Platform trust | **Play Integrity API** (standard requests) as a server-evaluated risk signal; root signals advisory; never block solely on client checks |
| WebView | Avoid; if unavoidable: restrict origins, disable file access, `WebViewAssetLoader`, no JS bridges to untrusted content |
| Supply chain | Dependency verification, lockfiles, cooldown for new versions, SCA/SBOM (CycloneDX), signed CI artifacts, action pinning by SHA (`PLAYBOOK_SECURITY.md` §11) |
| Native engines | Review sources/licenses, scan binaries, keep versions current, 16 KB-aligned |

### 15.2 Android 17 checklist (verify against the current behavior-change pages)

* **App memory limits** based on total device RAM apply to all apps: baseline memory per variant, detect limiter kills via `ApplicationExitInfo`, and test with the `am memory-limiter` commands.
* **Keystore key cap** for apps targeting API 37 (small fixed key set).
* **SMS OTP protections** (including WebOTP-format messages): use SMS Retriever/User Consent, not SMS permissions.
* **ECH** for TLS on apps targeting 37; **cross-profile loopback** blocked; **local-network access** changes; **static final fields** immutable; **IME visibility after rotation** not restored automatically;
  **new lock-free MessageQueue** for apps targeting 37; stricter RemoteViews memory limits.
* Run the app on API 37 devices/emulators in CI before raising `targetSdk` to 37; until then hold at `targetSdk` 36 (Play's floor — compliant, not the target).

### 15.3 Privacy and Play policy

* Data minimization per variant; **location, microphone, camera, voice and face-derived data are sensitive**: explicit consent, purpose limitation, retention limits, deletion with the account, regional rules **(verify)**.
* Children/teens: age gate and age-appropriate defaults for Game/Avatar; Families policy where applicable; restrict chat, voice and purchases by age.
* **Play Data safety form** and **permission declarations** (background location, foreground-service types, photo/video access) generated from the permissions and dependency registers; keep in sync with SDK behavior.
* Analytics/telemetry: consent aware; no precise location or content; hashed IDs; retention limits.

### 15.4 Play policy checklist (verify each release)

| Topic | Requirement to confirm |
|---|---|
| Target API | New apps/updates target API 36 (or the current floor) |
| Native code | 16 KB page-size support (check the current enforcement date) |
| Billing | Play Billing Library at the currently required major |
| Account deletion | In-app path and web path declared |
| Permissions | Background location, foreground-service types, notifications justified; no unnecessary sensitive permissions |
| Data safety | Matches actual SDK/data behavior |
| UGC/AI | Reporting, blocking, moderation, AI disclosure |
| Content rating | Reflects chat, voice, purchases, location |
| Edge-to-edge/predictive back/large screens | Verified (orientation locks ignored on large screens at target 36) |
| Loot boxes | Odds disclosure/regional restrictions if applicable |

---

## 16. Observability

| Signal | Implementation |
|---|---|
| Crashes/ANRs | Crashlytics or Sentry (NDK crash handling for engines), R8 mapping + native symbols uploaded; tags: variant, tier, device class |
| Exit reasons | `ApplicationExitInfo` on next start (OOM, memory limiter, ANR, signal) → telemetry and tier adjustment |
| Performance | **Macrobenchmark** in CI; production **JankStats**, startup (`reportFullyDrawn`), screen TTI via `androidx.tracing`/custom markers, frame time p50/p95 in scenes, memory (PSS) samples, thermal/battery state, governor decisions |
| Vitals | Play Console/Developer Reporting API alerts for crash/ANR/jank/wakelock/startup per device model |
| Network | Latency/error rates per endpoint and network type; realtime state transitions and lag; asset pack download success |
| Product | Onboarding → registration → verification funnel; checkout; encounter; session start; push opt-in/open rates |
| Tracing | `X-Correlation-Id` per user action, propagated via SDK/realtime/push payloads, shown on error screens |
| Diagnostics bundle | User-initiated, redacted: variant/version/build, device tier and capabilities, engine/renderer, governor history, permissions and channel states, push status, recent logs/errors with correlation IDs |
| Privacy | Consent-aware sampling, redaction filters, no PII/precise location in logs |

---

## 17. Testing strategy

Follows `PLAYBOOK_TESTS.md`; adds an `android` side (placement ADR in section 5):

| Level | Scope | Tooling |
|---|---|---|
| Unit | Domain use cases, ViewModels, mappers with fakes | JUnit, Turbine, coroutines-test, MockK/fakes (`src/test`) |
| Architecture | Module graph and layering rules | Konsist, Gradle dependency checks |
| Contract | SDK vs OpenAPI, notification payload schema, performance-script schema, route table parity, permission catalog | schema tests, MockWebServer |
| Compose UI | Screens per status; semantics/accessibility | Compose test rules, Robolectric |
| Screenshot | Layouts per font scale/dark mode/locale/RTL/large screen | Roborazzi/Paparazzi/Compose Preview Screenshot Testing |
| Integration | Repository + Room + outbox + WorkManager (test driver), push handling, realtime transports conformance | AndroidX test, WorkManager testing, MockWebServer |
| **E2E** | Onboarding → register → verify → login → forgot → reset; permission denial paths; push tap deep links; market checkout; game encounter (mock location); avatar session (mock audio/script) | Maestro / UI Automator / Espresso on emulators **and** real devices |
| **Device performance** | Cold/warm startup, scroll jank, scene frame time/memory/thermal on the Low/Mid pool | **Macrobenchmark**, Perfetto, JankStats, custom scene harness |
| Baseline profile | Generation and verification per variant | Baseline Profile Gradle plugin on real devices |
| Memory/limits | Memory ceilings, leak checks, memory-limiter behavior | LeakCanary, `am memory-limiter`, heap profiling |
| Location/anti-cheat | Mock routes (walk/drive/teleport), poor GPS, permission changes | Emulator routes, test providers |
| Audio/voice | Latency, barge-in, route changes, interruptions, lip-sync drift | Instrumented harness with recorded audio |
| Push | FCM test sender, token refresh, permission/channel states, Doze/standby, OEM devices, collapse/ttl | Device farm + `adb` |
| Accessibility | TalkBack flows, 200% font scale, contrast, touch targets, reduce motion | Accessibility Test Framework + manual |
| Security | MASTG-style checks, storage inspection, deep-link fuzzing, network config, secrets scan, dependency/SBOM scan | MobSF-class tools, custom |
| Compatibility | API 24 → 37, screen sizes/foldables/large screens, languages/RTL, offline/slow networks, 16 KB page-size emulator | Gradle Managed Devices, Firebase Test Lab |
| Upgrade | N-1 → N data/migration and token continuity | scripts |
| Store readiness | Manifest/permissions diff, 16 KB alignment, Data safety forms, listing assets, Play pre-launch report | CI scripts |

Rules: tests run on the **Low pool**, not only emulators; no production data; flaky tests quarantined with owner/expiry; every notification type and route has a contract/e2e test;
performance gates compare to the last release (e.g., +10% regression fails the build) and are stored as release evidence. **The contract tests cover every declared failure
mode**: each schema's *Validation Failure Modes* list — missing key, bad division, missing section, placeholder leak, CRLF, empty frontmatter, name/slug mismatch — has
a fixture and an assertion. A declared mode without a test is a gap, not a documentation line.

---

## 18. Build, release and operations

* **AAB** with Play App Signing; per-ABI/density/language splits; R8 full mode; baseline + startup profiles bundled; **dynamic feature modules** and **PAD** for heavy modules/assets.
* **Tracks**: internal → closed → open/production with **staged rollout** and halt; pre-launch report gating; Android vitals thresholds gate promotion.
* **In-app updates** (flexible/immediate) for forced upgrades via minimum-version config; **in-app review** at good moments.
* **Remote config** (data only, validated/signed): feature flags, device-tier overrides and blocklists, quality defaults, onboarding content, realtime mode, push categories, minimum version, emergency kill switches
  (AR, background location, voice, 3D, checkout provider).
* **Pipeline**: PR (typecheck/lint/Detekt/architecture/unit/Compose/contract, dependency+SBOM+secrets scan, size/permission diff, asset budgets) → main (variant builds, emulator e2e smoke, baseline-profile verify) → nightly (full e2e,
  device-farm run, Low-pool macrobenchmarks, soak, push suite) → RC (all + store-readiness, accessibility, security, evidence bundle) → staged rollout → post-release monitoring with rollback (halt rollout, remote kill switch, hotfix release).
* **Signing**: upload key in KMS/HSM with an isolated CI job and approval; never on developer machines.
* **Runbooks**: crash/ANR spike on a device model (blocklist + tier override), OEM push delivery failure, FCM credential expiry/outage, Play rejection/appeal, memory-limiter kill wave, engine regression, asset-pack corruption (rollback),
  payment provider incident, IdP outage, background-location policy review, privacy incident, avatar-provider loss, voice/LLM outage (degrade to text).

---

## 19. Governance, compliance and working with Claude Code

* **Ownership:** CODEOWNERS for `build-logic/**`, `gradle/libs.versions.toml`, `gradle/verification-metadata.xml`, manifests, `network_security_config`, R8/keep rules, `:core:security`, `:core:scene`, push/auth modules, `docs/permissions-register.md`.
* **Documents:** ADRs, dependency and permissions registers, device pool/tiers, support matrix, threat model, data-flow diagrams, privacy review, Play submission checklist, runbooks.
* **Compliance mapping** (`PLAYBOOK_AUDIT.md`): SDLC controls via CI gates; vendor management for SDKs/providers (FCM, maps, payments, voice/LLM, avatar, ARCore); privacy evidence for location/voice/face data; change management via staged rollouts; incident runbooks;
  evidence bundle per release (builds, SBOM, test/perf reports, permission diffs, approvals). CRA applicability for distributed apps: assess with counsel **(verify)**.
* **Claude Code** (`PLAYBOOK_CLAUDE.md`): path-scoped rules for `core/**`, `shell/**`, `feature/**`, `core/scene/**`; integrator role owns Gradle build logic, version catalog, manifests and lockfiles; skills: `new-android-feature` (module scaffold: domain/data/ui/contracts + tests),
  `new-notification-type` (schema, channel, deep link, handler, test), `new-scene-asset` (pipeline + budgets), `baseline-profile-refresh`, `device-perf-check`; specs list variants, permissions, tiers, budgets, push types, routes and test levels per AC.

---

## 20. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| One APK with all three products and all engines | Size, memory, permissions and review surface |
| Product flavors that only hide code (still compiling other variants' deps) | Leaks engines/permissions into the wrong binary |
| Android imports in domain modules | Breaks testability and layering |
| An `androidx.*` dependency in a pure JVM module whose artifact has no JVM variant | The module stops being pure; the desktop/JVM build breaks |
| Business logic in composables or in Activities | Untestable, recomposition cost |
| Blocking the main thread (disk, network, JSON, crypto) | Jank/ANR on low-end |
| Reflection-heavy libs at startup (Gson/reflection DI), kapt | Slow startup and builds |
| Unbounded image caches, full-size bitmaps, uncompressed textures | OOM/limiter kills |
| Game entities/products as hundreds of composables over a map | Recomposition/overdraw overload |
| WebView-hosted 3D/games on the Low/Mid path | Order-of-magnitude slowdowns |
| Requiring ARCore/GMS in the manifest for installability | Excludes much of the market |
| SMS permissions to read OTP; trampoline notification taps; per-item Keystore keys | Blocked/limited by modern Android |
| Tokens/secrets in SharedPreferences/DataStore plaintext/logs | Credential theft |
| Embedded login WebViews; custom crypto | Phishing/weak security |
| Asking all permissions at first launch | Denials and policy risk |
| Push payloads with sensitive data; push as the only sync path | Privacy leaks, missed state |
| Dependence on one avatar/voice vendor runtime | Shutdown risk (e.g., Ready Player Me closed in January 2026) |
| Ignoring OEM battery killers and Doze | Silent delivery failures |
| No baseline/startup profile, R8 compat mode, no macrobenchmarks | Slow startup, unproven performance |
| Ignoring 16 KB alignment, target-API and edge-to-edge changes | Upload rejection/UI breakage |
| No Low-pool testing; trusting emulators | Unproven claims |
| A linter the project ships but the gate does not run | It finds nothing until a review finds it for it |
| An instantiation/rename script that mutates the tree before validating its arguments | A half-renamed tree or an unbuildable namespace, discovered far from its cause |

---

## 21. Definition of done and checklists

### 21.1 Feature DoD

- [ ] Spec lists variant(s), slice, routes, permissions, push types, tier behavior, budgets and test levels per AC.
- [ ] Domain/use cases/ViewModels unit-tested with fakes; all `UiStatus` values covered; no logic in composables.
- [ ] Low-tier path implemented and verified on the Low pool; governor behavior defined.
- [ ] Uses the Kotlin SDK; no hand-written paths/DTOs; error taxonomy handled; correlation IDs surfaced.
- [ ] Permissions requested in context with primer + degraded path; register and merged-manifest diff updated.
- [ ] Accessibility verified (TalkBack, 200% font scale, contrast, touch targets); strings are resources — every module owns its own `strings.xml` and no user-visible literal lives in Kotlin; RTL/pseudo-locale checked.
- [ ] Offline/slow-network behavior defined; outbox/idempotency for mutations.
- [ ] Baseline profile journeys updated; macrobenchmark within budget; no main-thread violations (StrictMode clean).
- [ ] Telemetry and redaction verified; no sensitive data in logs/analytics/push.

### 21.2 Release checklist

- [ ] Reproducible builds per variant; APK/AAB size, permission, native-lib (ABI, 16 KB) and dependency diffs reviewed; SBOM and scans green.
- [ ] Unit/architecture/contract/Compose/e2e (emulator + real devices)/accessibility/security suites green.
- [ ] Low/Mid pool performance within budgets (start, jank, scene fps, memory, battery, crash/ANR rates).
- [ ] Push verified end to end (permission states, foreground/background/killed, deep links, token lifecycle, Doze/OEM).
- [ ] Auth flows verified: register, verify, login (password/passkey/Google), MFA/step-up, forgot, reset, sign-out, deletion.
- [ ] Play checklist verified (target API, billing library, Data safety, permissions, deletion, content rating, 16 KB).
- [ ] Staged rollout configured, vitals gates and kill switch ready, runbooks current.

### 21.3 Architecture acceptance checklist

- [ ] Kotlin 2.x, Compose (current BOM), Navigation 3, Hilt/KSP; `compileSdk 37`/AGP 9.1.1+ (or a documented hold) and `targetSdk 37` (36 is Play's floor, not the target — see D18).
- [ ] Three thin app modules; per-variant dependencies, permissions and native libs verified by CI.
- [ ] Domain modules are pure Kotlin; `ui` never depends on `data`; cross-slice access only via `contracts`; Konsist/graph checks green.
- [ ] Pure JVM modules import no `android.*`; an `androidx.*` dependency is legal only where the artifact publishes a JVM variant (the multiplatform Navigation3 runtime in `contracts`).
- [ ] The build runs on a pinned full JDK (`javac`), and the PR gate runs every linter the project ships — Android Lint, Detekt, the architecture check and the contract tests.
- [ ] UDF with immutable UiState, effects channel, shared statuses; Compose compiler reports reviewed.
- [ ] Route table parity with `NavKey` set; App Links verified; deep links validated and resumed after auth.
- [ ] Shared shell: onboarding, login (Credential Manager/AppAuth), registration, forgot/reset, session security, deletion.
- [ ] Push: FCM v1, channels/categories, permission flow, token lifecycle, direct-activity PendingIntents, OEM guidance.
- [ ] `DeviceTier`, measured benchmark, `QualityGovernor` (thermal, ADPF, memory pressure), remote overrides, performance modes.
- [ ] `SceneEnginePort` with 2D, Filament, Godot, Unity adapters in dynamic feature modules; lazy load/dispose; fallback chain.
- [ ] Asset pipeline (KTX2, LODs, budgets), PAD delivery with texture format targeting, signed manifests.
- [ ] Game: server-authoritative, GL-layer map, location/battery modes, FGS-type compliance, ARCore optional, anti-cheat signals.
- [ ] Market: server-authoritative pricing, idempotent checkout, Stripe/Google Pay, Paging 3 + Coil sizing.
- [ ] Avatar: open formats, provider port, voice pipeline with barge-in, performance-script contract, optional tracking.
- [ ] R8 full mode, baseline + startup profiles, macrobenchmarks in CI, `ApplicationExitInfo` telemetry.
- [ ] Device-lab testing with Low/Mid pool and CI performance gates; Android 17 checklist run.

---

## Appendix A — Convention plugin and module snippets (sketch)

```kotlin
// build-logic/convention: AndroidAppConventionPlugin
class AndroidAppConventionPlugin : Plugin<Project> {
  override fun apply(target: Project) = with(target) {
    pluginManager.apply("com.android.application")            // AGP 9: Kotlin support is built in (no kotlin-android plugin)
    pluginManager.apply("org.jetbrains.kotlin.plugin.compose")
    extensions.configure<ApplicationExtension> {
      compileSdk = 37
      defaultConfig { minSdk = 24; targetSdk = 37 }
      buildTypes { release { isMinifyEnabled = true; isShrinkResources = true
        proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro") } }
      packaging { jniLibs { useLegacyPackaging = false } }     // uncompressed, aligned native libs
    }
  }
}
```

```kotlin
// feature/market/cart/domain/build.gradle.kts  (PURE KOTLIN)
plugins { id("heimdall.jvm.library") }
dependencies { implementation(projects.core.kernel); implementation(libs.kotlinx.coroutines.core) }

// feature/market/cart/ui/build.gradle.kts
plugins { id("heimdall.android.library"); id("heimdall.compose"); id("heimdall.hilt") }
dependencies { implementation(projects.feature.market.cart.domain); implementation(projects.core.designsystem); implementation(projects.core.uiCore) }
```

## Appendix B — Device tier from native signals (sketch)

```kotlin
class AndroidDeviceCapabilities @Inject constructor(@ApplicationContext private val ctx: Context) : DeviceCapabilityPort {
  override suspend fun hintTier(): DeviceTier {
    val am = ctx.getSystemService<ActivityManager>()!!
    val mem = ActivityManager.MemoryInfo().also(am::getMemoryInfo)
    val gb = mem.totalMem / 1_073_741_824.0                      // reported RAM is lower than marketing RAM (4 GB ≈ 3.5–3.8)
    val perfClass = if (Build.VERSION.SDK_INT >= 31) Build.VERSION.MEDIA_PERFORMANCE_CLASS else 0
    return when {
      am.isLowRamDevice || gb < 3.2                            -> DeviceTier.Low
      perfClass >= Build.VERSION_CODES.TIRAMISU || gb >= 7.0   -> DeviceTier.High
      else                                                     -> DeviceTier.Mid
    }
  }
  override val thermal: StateFlow<Thermal> = callbackFlow {
    val pm = ctx.getSystemService<PowerManager>()!!
    val l = PowerManager.OnThermalStatusChangedListener { trySend(it.toThermal()) }
    pm.addThermalStatusListener(l); awaitClose { pm.removeThermalStatusListener(l) }
  }.stateIn(appScope, SharingStarted.Eagerly, Thermal.None)
}
```

## Appendix C — FCM service skeleton

```kotlin
@AndroidEntryPoint
class PushService : FirebaseMessagingService() {
  @Inject lateinit var tokens: DeviceTokenUploader      // enqueues WorkManager job (only when signed in)
  @Inject lateinit var router: NotificationPresenter    // validates payload, picks channel, builds PendingIntent to MainActivity

  override fun onNewToken(token: String) = tokens.enqueue(token)

  override fun onMessageReceived(message: RemoteMessage) {
    val payload = PushPayload.parseOrNull(message.data) ?: return   // versioned schema; unknown types ignored + counted
    router.present(payload)                                          // no PII on lock screen; deep link validated on tap
  }
}
```

## Appendix D — Manifest and network config snippets

```xml
<!-- optional AR: do not exclude non-AR devices -->
<uses-feature android:name="android.hardware.camera.ar" android:required="false" />
<meta-data android:name="com.google.ar.core" android:value="optional" />

<!-- Game background tracking only -->
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_LOCATION" />
<service android:name=".tracking.TrackingService" android:exported="false" android:foregroundServiceType="location" />

<!-- Verified App Links -->
<activity android:name=".MainActivity" android:exported="true">
  <intent-filter android:autoVerify="true">
    <action android:name="android.intent.action.VIEW" />
    <category android:name="android.intent.category.DEFAULT" />
    <category android:name="android.intent.category.BROWSABLE" />
    <data android:scheme="https" android:host="app.example.com" android:pathPrefix="/app" />
  </intent-filter>
</activity>

<application android:allowBackup="false" android:dataExtractionRules="@xml/data_extraction_rules"
             android:networkSecurityConfig="@xml/network_security_config" android:usesCleartextTraffic="false" />
```

```xml
<!-- res/xml/network_security_config.xml -->
<network-security-config>
  <base-config cleartextTrafficPermitted="false"><trust-anchors><certificates src="system"/></trust-anchors></base-config>
  <!-- optional pinning with backup pins + expiration; coordinate with the remote kill switch -->
</network-security-config>
```

## Appendix E — Credential Manager login (sketch)

```kotlin
suspend fun signIn(activityContext: Context, passkeyRequestJson: String?, googleServerClientId: String): Outcome<AuthResult> {
  val options = buildList {
    add(GetPasswordOption())
    passkeyRequestJson?.let { add(GetPublicKeyCredentialOption(it)) }               // options from POST /api/v1/public/auth/passkey/options
    add(GetGoogleIdOption.Builder().setServerClientId(googleServerClientId).setFilterByAuthorizedAccounts(false).build())
  }
  val result = credentialManager.getCredential(activityContext, GetCredentialRequest(options))
  return when (val c = result.credential) {
    is PasswordCredential        -> authApi.login(c.id, c.password)
    is PublicKeyCredential       -> authApi.verifyPasskey(c.authenticationResponseJson)
    is CustomCredential          -> GoogleIdTokenCredential.createFrom(c.data).let { authApi.oidcExchange(it.idToken) }
    else                         -> Outcome.Failure(AppError.Unsupported)
  }
}
```

## Appendix F — SDK transport and SSE (sketch)

```kotlin
val api = OkHttpClient.Builder()
  .addInterceptor(CorrelationInterceptor()).addInterceptor(TenantInterceptor(session)).addInterceptor(ClientInfoInterceptor(variant, versionCode))
  .authenticator(SingleFlightRefreshAuthenticator(session))                // Mutex-guarded refresh, replay once
  .callTimeout(30, SECONDS).build()

val sseClient = api.newBuilder().readTimeout(45, SECONDS).build()            // > server heartbeat (15 s); dedicated client
val source = EventSources.createFactory(sseClient).newEventSource(
  Request.Builder().url(streamUrl).header("Accept", "text/event-stream").header("Last-Event-ID", lastId ?: "").build(),
  object : EventSourceListener() {
    override fun onEvent(es: EventSource, id: String?, type: String?, data: String) = pipeline.accept(id, type, data) // validate, dedupe, route
    override fun onFailure(es: EventSource, t: Throwable?, r: Response?) = state.reconnectWithBackoff(t, r)
  })
```

## Appendix G — R8, Baseline Profile and Startup Profile setup (checklist)

1. `isMinifyEnabled = true`, R8 full mode (remove any compatibility-mode flag), resource shrinking, keep rules via consumer rules; test reflection paths (serialization uses generated serializers).
2. `:baselineprofile` module (Macrobenchmark + Baseline Profile Gradle plugin) generating profiles for the real journeys per variant: cold start → login → home; market: home → search → product → cart → checkout; game: map → encounter; avatar: session start.
3. Enable **Startup Profile** generation so R8 lays out startup classes together; include library profiles; keep `profileinstaller` for sideload/older paths.
4. CI: generate on a real device, verify via Macrobenchmark `CompilationMode.Partial` vs `None` comparisons; fail on regression.
5. Instrument `reportFullyDrawn()` at the true "usable" point; track TTID/TTFD in production with JankStats and Play vitals.

## Appendix H — Support matrix template

```markdown
# Support matrix (per release)
| Item | Value |
| minSdk / targetSdk / compileSdk | 24 / 37 / 37 (AGP 9.1.1+) |
| ABIs | arm64-v8a (+armeabi-v7a if required) · 16 KB page-size aligned |
| GMS policy | Required | Optional (fallback providers: push/location/integrity) |
| Device tiers | Low (≤3 GB, Go) · Mid (4–6 GB) · High (≥8 GB) · device pool: <list> |
| Form factors | Phone, foldable, tablet, ChromeOS (resizable; orientation locks ignored ≥600dp at target 36) |
| Known limitations per tier | AR availability, background location, on-device ML, camera tracking, voice features |
```
