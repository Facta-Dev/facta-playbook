# PLAYBOOK_FRONTEND_MOBILE.md — React Native mobile playbook (Game · Market · Avatar)

Architecture and engineering contract for a **React Native (TypeScript)** mobile
product family with **three app variants** built from one monorepo:

| Variant | What it is | Core capabilities |
|---|---|---|
| **Game** | Location-based 3D game in the style of Pokémon GO | Real-world map, spawns and encounters, 3D creatures, optional AR, catching, collection, events, social |
| **Market** | Marketplace in the style of Amazon | Catalog, search and filters, product pages, reviews, cart, checkout, orders and tracking, wishlist, seller tooling |
| **Avatar** | 3D avatar with voice and movement | 3D character, speech in/out, lip-sync, gestures/emotions, optional camera-driven motion, live conversation |

**All three** share: **onboarding, login, registration, forgot/reset password,
push notifications**, the same SDK, design system, security model and release
pipeline. And all three must run well on **low- and mid-range phones**, not only
flagships.

It extends `PLAYBOOK_FRONTEND.md` (MVVM + SDK), `PLAYBOOK_URLS.md` (route and
permission model), `PLAYBOOK_BACKEND.md` (slices, realtime, idempotency),
`PLAYBOOK_SECURITY.md`, `PLAYBOOK_TESTS.md`, `PLAYBOOK_AUDIT.md`,
`PLAYBOOK_CLAUDE.md`, and reuses patterns from `PLAYBOOK_CHROME_EXTENSION.md` and
`PLAYBOOK_DESKTOP_QT.md` (managed config, kill switch, diagnostics, realtime).

The rules that carry the whole file:

1. **The app shell is React Native; heavy 3D is a replaceable module.** The shell
   (auth, onboarding, market, settings, push) never depends on a specific 3D
   engine; scenes plug in behind a `SceneEngine` port with tiered adapters.
2. **Design for the floor, enhance upward.** Every feature has a low-end path.
   High-end devices get more fidelity, never different logic.
3. **Capability tiers, not device lists.** Detect and measure; degrade at
   runtime (quality governor) instead of crashing.
4. **One variant = one binary.** Each app ships only the native code, permissions
   and assets it needs (a market app must not carry a game engine).
5. **Shared TypeScript ViewModels and SDK** with web/extension; mobile adds ports.
6. **Authorization stays on the server.** `{userType}` and client permissions
   shape UX only.
7. **Server-authoritative for anything that matters** (game state, prices,
   payments, avatar entitlements); the client is untrusted.
8. **Push is a channel, not a data path.** Payloads are minimal; the app fetches
   truth after a tap.
9. **No vendor lock-in for irreplaceable assets** (avatars, models, user data):
   open formats and your own storage.
10. **Measure on real low-end devices in CI.** Performance claims without a
    device lab are opinions.

> Verification note: the React Native ecosystem moves fast. Status below is
> as of November 2026: React Native **0.88** shipped in October 2026, Expo
> **SDK 58** is current, the **New Architecture is the only architecture** (legacy
> removed from RN 0.82/0.84 and Expo SDK 55), and **Hermes V1 is the default
> engine since RN 0.84**. Items marked **(verify)** depend on exact versions,
> store policy dates or library maintenance and must be re-checked before
> adoption. Budgets are starting points to calibrate on your own device pool.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Framework | **Expo (current SDK, e.g., 58) with Continuous Native Generation + development builds**; escape to bare only for unsupported native needs | New Architecture only, Hermes V1 default, EAS Build/Submit/Update, config plugins, Expo Modules API for custom native code |
| D2 | Language | TypeScript `strict` | Same as web/extension; shared ViewModels and SDK |
| D3 | Variants | **Three apps from one monorepo** via `APP_VARIANT` (`game`, `market`, `avatar`), separate bundle IDs and store listings; a combined "super app" is **not** recommended | Binary size, memory, permissions and review surface stay minimal per product (critical on low-end) |
| D4 | Navigation | **Expo Router** (file-based, deep links by default); a plain app entry is valid when the typed route table is small | Routes mirror `PLAYBOOK_URLS.md`; the typed route table stays the source of truth either way (section 6); verify the router's `root` option for placing the routes folder **(verify)** |
| D5 | Server state | TanStack Query + the shared SDK | Same contract as other clients |
| D6 | Client state | Zustand (selector-based) + MMKV for tiny persisted prefs; no heavy global stores | Fewer re-renders on slow CPUs |
| D7 | Lists | **FlashList v2** | Virtualization with recycling; New Architecture only |
| D8 | Images | **expo-image** (WebP/AVIF, blurhash/thumbhash, disk cache) | Memory-safe image handling |
| D9 | 2D graphics/particles | **@shopify/react-native-skia** (only where needed) | GPU 2D; costs memory, use sparingly |
| D10 | 3D rendering | **Tiered, behind a `SceneEngine` port** (section 13): 2D/pre-rendered fallback → **react-native-filament** (native, separate render thread) → embedded **Godot** (Compatibility renderer for low-end) → embedded **Unity** (AR-heavy, high tier only) | No single engine fits all tiers; R3F on `expo-gl` issues GL calls from the JS thread (see 13.2) |
| D11 | Maps (Game) | **MapLibre** (`@maplibre/maplibre-react-native`) or **Mapbox** (`@rnmapbox/maps`), GL layers for entities | Native vector tiles; never render hundreds of RN Views over a map |
| D12 | Push | **expo-notifications** with **native device tokens (FCM/APNs)** and your backend sending through **FCM HTTP v1** (iOS via APNs); Expo Push Service optional | Control, data residency and no extra processor; see section 8 |
| D13 | Auth | First-party email/password + social + optional passkeys through the `identity` slice; OIDC with PKCE in the system browser for SSO; tokens in the OS keystore | `PLAYBOOK_SECURITY.md` §4 |
| D14 | Payments | **Stripe SDK** (PaymentSheet, Apple Pay/Google Pay) for physical goods/services; **StoreKit/Play Billing** (via `react-native-iap` or RevenueCat) for digital goods | Store rules differ by good type **(verify)** |
| D15 | Avatars | Open formats: **glTF/GLB + VRM 1.0**, your own asset storage, provider behind an `AvatarProviderPort` | Ready Player Me shut down on 31 Jan 2026, and other avatar platforms have disappeared since: never depend on a single vendor |
| D16 | Observability | Sentry (or Crashlytics) + OpenTelemetry-style correlation IDs + `react-native-performance` markers | Section 16 |
| D17 | Release | EAS Build/Submit, EAS Update (OTA) for JS-only, staged store rollouts, remote config + kill switch | Section 18 |
| D18 | Min OS | Derive from Expo SDK/RN minimums and your fleet data; document in `docs/support-matrix.md` | Re-check each SDK upgrade |
| D19 | CPU ABIs | Android **arm64-v8a** primary, **armeabi-v7a** only if your fleet requires it; iOS arm64 | Smaller downloads; many low-end Android phones are still 32-bit userland **(verify with fleet data)** |
| D20 | Android page size | All native libs (engines, ML, audio) must support **16 KB page sizes** | Google Play requirement for apps targeting recent Android versions **(verify date)** |

---

## 2. The product family

### 2.1 Variant matrix

| Capability | Game | Market | Avatar |
|---|---|---|---|
| Shared core (auth, onboarding, push, settings, design system, SDK) | ✔ | ✔ | ✔ |
| Location | **Foreground + optional background** | Optional (store locator, delivery estimate), approximate | No |
| Camera | Optional (AR mode) | Optional (barcode scan, photo reviews, visual search) | Optional (face/pose tracking, avatar from selfie) |
| Microphone | Optional (voice chat, later) | Optional (voice search) | **Core** |
| 3D rendering | **Core** | Optional (3D/AR product view, off by default) | **Core** |
| Realtime (WebSocket/SSE) | **Core** (events, nearby) | Order status, price alerts | **Core** (conversation/performance) |
| Payments | In-app purchases (digital) | **Core** (Stripe, wallets) | Subscriptions/items (digital) |
| Offline | Cached map tiles/collection | Cart, wishlist, recent items | Cached avatar and last sessions |
| Heaviest native deps | Map SDK + engine (Godot/Unity/Filament) | Stripe, scanner | Engine (Filament/Godot) + audio/WebRTC |
| Target download (low-end) | Base ≤ ~100 MB + on-demand packs **(verify limits)** | ≤ ~35 MB | ≤ ~80 MB + on-demand packs |

### 2.2 What must be identical across all three

* Login, registration, forgot/reset password, onboarding (section 7).
* Push notification registration, permission flow, categories, deep-link routing (section 8).
* Session/token handling, secure storage, account deletion, consent, i18n, accessibility.
* SDK, error taxonomy, correlation IDs, telemetry, kill switch, remote config.
* Design tokens and base UI kit (variant themes on top).

### 2.2.1 What may differ

Feature packs (`feature-game`, `feature-market`, `feature-avatar`), native modules, permissions, store
metadata, theme, and onboarding copy.

---

## 3. Device classes, budgets and the quality governor

### 3.1 Device classes (calibrate with your analytics)

| Class | Typical profile | Examples of traits |
|---|---|---|
| **Low** | ≤ 3 GB RAM, little-core CPUs (Cortex-A53/A55 class), entry GPUs, eMMC storage, small screens, aggressive OEM battery killers; iPhones from the oldest supported generation | Android "Go"-type devices, 5–6-year-old phones |
| **Mid** | 4–6 GB RAM, mid-tier SoCs (e.g., Snapdragon 6/7-series, Dimensity 700/800-class, A-series from ~3–4 years ago), UFS storage | Bulk of the global market |
| **High** | ≥ 8 GB, flagship SoC | Optional extras only |

Keep a **device pool** (real phones, not just emulators): at least two Low, two Mid,
one High per OS, plus a cloud device farm for breadth.

### 3.2 Budgets (Low-class device, release build; starting points)

| Metric | Budget |
|---|---|
| Cold start to interactive (JS shell) | ≤ 3.5 s (Low), ≤ 2.5 s (Mid) |
| JS frame rate in lists and forms | 60 fps target; no sustained JS thread > 16 ms work; ≥ 30 fps worst case |
| Touch response | ≤ 100 ms |
| Shell memory (no 3D) | ≤ ~180 MB resident |
| 3D scene memory (total process) | Low ≤ ~300 MB, Mid ≤ ~450 MB (leave headroom for the OS low-memory killer) |
| 3D frame rate | Low: stable 30 fps (capped, resolution scale ≤ 0.75); Mid: 30–60 fps; High: 60 fps |
| App download size (per-ABI) | Market ≤ ~35 MB; Game/Avatar base ≤ ~100 MB with heavy assets in on-demand packs |
| Battery (Game, active map session) | Track %/hour on the Low pool; no degradation release over release |
| Network | Works on slow 3G; request count and payload budgets per screen |
| Memory growth over 30 min of play/browsing | ≈ 0 |
| Crash-free sessions | ≥ 99.5% (Low pool measured separately) |

Budgets are enforced by CI performance jobs on the device pool (section 17.4).

### 3.3 Capability detection (tier selection)

Inputs, combined into a **`DeviceTier`** (`low | mid | high`) plus **feature capabilities**:

| Signal | Source |
|---|---|
| Total RAM, low-RAM flag, OS/API level, ABI | `react-native-device-info` / `expo-device` / a small native module (`ActivityManager.isLowRamDevice`) **(verify APIs)** |
| GPU/renderer support, GLES/Vulkan/Metal level | Engine adapter probe at first launch |
| ARCore/ARKit availability | Native availability checks (many low-end Android phones lack ARCore) |
| Thermal state, low-power mode, battery | Native thermal APIs (Android thermal status, iOS thermal state), `expo-battery` |
| Memory pressure | iOS memory warnings, Android `onTrimMemory` via a small native module |
| **Measured performance** | First-run micro-benchmark (render N frames of a reference scene, record p95 frame time) stored and refreshed |
| Remote overrides | Remote config keyed by device model/OS (blocklist/forced tier) |

Rules: the initial tier is a **hint**; the **measured** result and the runtime
governor win. Persist the tier, re-evaluate after OS/app updates and on repeated
frame-drop events.

### 3.4 Quality governor (runtime)

A `QualityGovernor` (application layer) adjusts a `QualityProfile` live:

| Knob | Low → High |
|---|---|
| Render scale | 0.6–0.75 → 1.0 |
| Frame cap | 30 → 60 |
| Shadows / post-processing / reflections | off → on |
| Texture resolution (mip bias / pack) | 512 → 2048 |
| Mesh LOD and max visible entities | aggressive → generous |
| Particle count, animation fidelity | minimal → full |
| Map layers (3D buildings, labels density) | off/simple → rich |
| Camera tracking / AR | off → on |

Triggers to **step down**: rolling p95 frame time over budget, thermal state ≥
"serious", battery saver, memory warning, repeated JS frame drops. Step **up**
only after a sustained healthy period with hysteresis to avoid flapping. Users
see a "Performance mode" setting (Auto / Battery saver / High quality); managed
policy and remote config can force a profile.

---

## 4. Architecture: hexagonal + MVVM + SDK on mobile

### 4.1 Layers

```text
┌───────────────────────────────────── mobile app (one variant) ─────────────────────────────────────┐
│ INBOUND ADAPTERS                                      OUTBOUND ADAPTERS                          │
│ Views (RN) + ViewModel hooks · Push events · Deep links    SDK/HTTP · Realtime (WS/SSE) · SQLite/MMKV │
│ Location/sensor events · Timers/background tasks           Secure store · Push provider · Location   │
│ Realtime events                                            Camera/Audio · SceneEngine · Payments     │
│            │                                                      ▲                                │
│            ▼                                                      │                                │
│      APPLICATION (use cases, ports, policy, sync, quality governor) ─┘                              │
│            ▼                                                                                        │
│      DOMAIN (entities, value objects, rules, events)                                                │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Contains | May import | Must not import |
|---|---|---|---|
| **domain** | Entities, value objects, rules (pure TS) | `shared-kernel` | `react`, `react-native`, Expo modules, SDK |
| **application** | Use cases, ports, governor, sync/outbox | domain, `contracts`, `core` | RN/Expo native modules, UI |
| **viewmodels** (shared with web/extension) | Presentation state and commands as hooks | application ports/types, SDK types, `react` (hooks) | `react-native`, `expo-*`, router, `window` |
| **adapters/out** | SDK, realtime, storage, push, location, camera, audio, scene engines, payments | application ports, RN/Expo APIs | views |
| **views** | RN components, layout, styling | ViewModel hooks, `ui-kit` | SDK, storage, native modules directly |
| **contracts** | DTOs, error codes, notification payloads, route table | `shared-kernel` | everything else |

The ViewModel contract is exactly as in `PLAYBOOK_FRONTEND.md` §4: hooks that
return state + commands + a `status` enum
(`loading | ready | empty | forbidden | not-found | offline | error | policy-blocked`),
with platform effects behind ports (`Navigator`, `Notifier`, `Haptics`, `Analytics`).
Enforce with dependency-cruiser/ESLint boundaries; violations fail CI.

### 4.2 Mobile ports

```ts
export interface PushPort        { permissionState(): Promise<PermState>; requestPermission(): Promise<PermState>;
                                    registerDevice(): Promise<DeviceReg>; unregisterDevice(): Promise<void>;
                                    onNotification(cb: (n: IncomingNotification) => void): Unsub;
                                    onTap(cb: (n: IncomingNotification) => void): Unsub; }
export interface SecureStorePort { get(key: string): Promise<string | null>; set(key: string, v: string): Promise<void>; remove(key: string): Promise<void>; }
export interface BiometricsPort  { available(): Promise<boolean>; authenticate(reason: string): Promise<boolean>; }
export interface LocationPort    { current(opts: LocOpts): Promise<Fix>; watch(opts: LocOpts, cb: (f: Fix) => void): Unsub;
                                    startBackground(opts: BgLocOpts): Promise<void>; stopBackground(): Promise<void>; }
export interface DeviceCapabilityPort { tier(): Promise<DeviceTier>; capabilities(): Promise<Capabilities>; thermal(): ThermalState; }
export interface SceneEnginePort { init(cfg: SceneConfig): Promise<void>; load(scene: SceneSpec): Promise<void>;
                                    send(cmd: SceneCommand): void; onEvent(cb: (e: SceneEvent) => void): Unsub;
                                    setQuality(q: QualityProfile): void; dispose(): Promise<void>; }
export interface AudioPort       { startCapture(cfg: CaptureCfg): Promise<AudioStream>; play(clip: AudioSource): Promise<PlaybackHandle>; }
export interface PaymentsPort    { presentPaymentSheet(intent: PaymentIntentRef): Promise<PaymentResult>; }
export interface NetworkStatePort{ current(): NetState; onChange(cb: (s: NetState) => void): Unsub; }
export interface DeepLinkPort    { initial(): Promise<ParsedLink | null>; onLink(cb: (l: ParsedLink) => void): Unsub; }
export interface RealtimePort    { start(cfg: RtConfig): void; stop(): void; setTopics(t: Set<string>): void; /* events: state, event, resync */ }
```

Ports return `Promise<Result<T,E>>` or typed events; errors are values. Platform libraries are
confined to `adapters/out`.

### 4.3 State and data

| Concern | Choice |
|---|---|
| Server state | TanStack Query keyed `[userType, module, resource, tenantId?, params]` (same as web) |
| Client state | Small Zustand stores with selectors; derive, don't duplicate |
| Persisted prefs | MMKV (synchronous JSI KV) for non-sensitive; **SecureStore** for secrets |
| Offline DB | `expo-sqlite` (WAL) for cart, wishlist, collection cache, outbox |
| Forms | react-hook-form + zod schemas shared with web |
| Re-render control | Memoized selectors, stable props, React Compiler where validated on the Low pool (**verify** benefit and regressions), no giant context providers |

### 4.4 Offline, outbox and sync

* The local DB is a **projection**; the backend is the authority.
* Mutations while offline go to an **outbox** with `Idempotency-Key`, retry with
  backoff and per-entity ordering, mirroring the backend idempotency contract.
* Conflicts return typed results and are resolved in the ViewModel.
* Version/schema gates prevent old clients from corrupting newer data.

---

## 5. Monorepo layout and variant configuration

```text
apps/
├── mobile-game/      app.config.ts  eas.json  src/routes (thin)   # APP_VARIANT=game
├── mobile-market/    app.config.ts  eas.json  src/routes (thin)   # APP_VARIANT=market
└── mobile-avatar/    app.config.ts  eas.json  src/routes (thin)   # APP_VARIANT=avatar
packages/
├── sdk/                 # TS SDK (generated from OpenAPI; shared with web/extension)
├── viewmodels/          # shared ViewModels (all clients)
├── core/                # ports, route builders, permission helpers, error taxonomy
├── tokens/  ui-kit/     # design tokens; RN components (variant theming)
├── mobile-core/         # shell: auth, onboarding, push, settings, device capability, governor, telemetry, deep links
├── scene-engine/        # SceneEnginePort + adapters: 2d-fallback, filament, godot, unity (each its own package/entry)
├── feature-game/        # map, spawns, encounters, inventory, events (+ native deps declared here)
├── feature-market/      # catalog, search, product, cart, checkout, orders, reviews, wishlist, seller
└── feature-avatar/      # avatar creator/viewer, voice, lip-sync, gestures, sessions
tests/<slice>/mobile/{unit,component,integration,contract,e2e,perf,a11y,security}/
tests/_platform/{architecture,device-perf,push,auth,store-readiness}/
docs/{adr,support-matrix.md,dependency-register.md,permissions-register.md,device-pool.md,runbooks/}
```

Rules:

* **Each variant app imports only its own feature pack**; the Metro/dependency graph
  and config plugins must make it impossible for the market app to pull engine code.
  CI checks the installed native modules per variant against an allowlist.
* Slice names match backend slices and URL modules: `identity`, `notifications`,
  `devices`, `game`, `market`, `avatar` (and finer slices inside each pack).
* `app.config.ts` selects variant-specific name, bundle IDs, icons, permissions,
  plugins, associated domains and push settings from `APP_VARIANT` (Appendix A).
* Native dependencies are declared **in the feature package** and included by
  config plugins only for the variant that needs them.

**Workspace and bundler rules (pnpm + Metro).** The apps and the shared packages
live in one pnpm workspace, and React Native has one non-negotiable invariant:
**exactly one copy of `react` and `react-native` in the tree**. pnpm's default
isolated layout nests a second copy under the app and Metro does not resolve a
nested duplicate the way a bundler would — the renderer breaks at runtime, and
the export dies with Metro's `Invariant Violation: Failed to collapse`
(metro-file-map cannot collapse a workspace symlink that resolves above the
project root). Two settings prevent it, and both belong in the repository:

```text
.npmrc               node-linker=hoisted                 # flat node_modules, like npm
app/metro.config.js  watchFolders = [workspaceRoot]
                      resolver.nodeModulesPaths = [app/node_modules, root/node_modules]
                      resolver.disableHierarchicalLookup = true
```

* `.npmrc` sets `node-linker=hoisted`, so the committed lockfile resolves the flat
  layout: the app and the shared packages install from the same tree.
* `metro.config.js` (in the app package) watches the **workspace root**, not only
  the app folder — the shared packages live outside it and pnpm links them as
  symlinks. `nodeModulesPaths` lists the app's `node_modules` first and the
  workspace root's second; `disableHierarchicalLookup` stops Metro from walking
  up from a file and finding a nested duplicate of React Native.
* The reference kit ships exactly this pair (`frontend-mobile/.npmrc`,
  `frontend-mobile/app/metro.config.js`). `expo export` is the gate that proves
  it: a workspace that resolves is the only way to know before a store build.

### 5.1 Directory inside a feature pack

```text
packages/feature-<x>/src/<slice>/{domain,application,adapters/{in,out},viewmodels,views,contracts}/
```

---

## 6. Navigation, routes and deep links (URL playbook)

The mobile app uses the **same path grammar** as web and desktop:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]

/app/market/products/{productId}            /app/market/cart
/app/game/map                                /app/game/encounters/{encounterId}
/app/avatar/sessions/{sessionId}             /app/avatar/avatars/{avatarId}
/app/notifications/preferences
Session-less (public experience):  /onboarding  /login  /register  /forgot-password  /reset-password  /verify-email
```

* Expo Router file layout mirrors this (route groups for layouts; `app/` vs `public`
  experiences; place the routes under a folder that yields the exact URL paths **(verify
  router `root` and typed-routes behavior)**). The router is an implementation
  detail, not the contract: the typed route table below is. A file-based
  convention is acceptable only while it cannot contradict that table — the
  reference kit ships a plain `App.tsx` entry and resolves every path against the
  table (`frontend-mobile/docs/adr/0001-expo-with-a-plain-app-entry.md`).
* **Universal/App Links and custom scheme:** `https://<domain>/app/...` and
  `<scheme>://app/...`, associated domains / `assetlinks.json` per variant.
* Route builders are generated from the shared route table; no hand-written paths.
* The **typed route table is the source of truth for the paths the app ships**: the
  optional `{subresource}` segment is part of the grammar, and a client whose table
  declares no subresource route **refuses a five-segment path as unknown** — it never
  guesses a route.
* Guards (`RequireAuth`, `RequireUserType`, `RequirePermission`) only shape UX (redirect to
  `/login`, `/403`, allowed home); **the server authorizes every call**.
* Deep-link/notification parameters are **untrusted**: validate IDs, never trigger
  state-changing actions from a link without UI confirmation, require a session.
* Cold-start deep links and notification taps are queued until auth/bootstrap completes,
  then routed (so a tap while signed out lands on login, then continues).

---

## 7. Shared platform features: onboarding, login, registration, forgot password

### 7.1 Backend contract (identity slice; session-less `/public` experience)

| Purpose | Endpoint (examples; follow `PLAYBOOK_URLS.md`) |
|---|---|
| Register | `POST /api/v1/public/auth/register` |
| Verify email / phone | `POST /api/v1/public/auth/verify` · `POST /api/v1/public/auth/verify/resend` |
| Login | `POST /api/v1/public/auth/login` (+ MFA challenge step) |
| Social/OIDC exchange | `POST /api/v1/public/auth/oidc/callback` (code + PKCE verifier) |
| Passkey | `POST /api/v1/public/auth/passkey/options` · `.../verify` |
| Refresh / logout | `POST /api/v1/public/auth/token/refresh` · `POST /api/v1/app/auth/logout` |
| Forgot password | `POST /api/v1/public/auth/password/forgot` |
| Reset password | `POST /api/v1/public/auth/password/reset` |
| Change password / sessions | `POST /api/v1/app/auth/password/change` · `GET/DELETE /api/v1/app/auth/sessions` |
| Account deletion | `DELETE /api/v1/app/account` (re-auth + grace period) |
| Consent | `POST /api/v1/app/account/consents` (versioned ToS/Privacy/marketing) |
| Device registration (push) | `POST /api/v1/app/devices` · `DELETE /api/v1/app/devices/{deviceId}` |

All requests carry `X-Client: mobile/<variant>/<version> (<os>; <model class>)`,
`X-Correlation-Id`, and `Idempotency-Key` where applicable. Errors use the shared
envelope and `SdkError` taxonomy.

### 7.2 Onboarding

* **3–5 screens maximum**, skippable, content from remote config (copy, images, order) with
  bundled fallback; shown once per install/account; resumable.
* **Value first, permissions later**: never ask for location, camera, mic or notifications on
  the first screen. Use **permission primers** at the moment of need (see 7.6).
* Variant-specific steps: Game (safety notice, age gate, choose starter), Market (country,
  language, interests), Avatar (create/choose avatar, voice consent, AI disclosure).
* Low-end: static images (WebP/AVIF), no heavy video/Lottie loops; respect "reduce motion".
* Funnel analytics (privacy-safe) per step; A/B via remote config.
* Accessible: screen-reader order, large-text support, RTL.

### 7.3 Registration

* Minimal fields (email or phone + password, or social), **password policy** shown inline
  (length-first policy, breached-password check server-side), show/hide password, paste allowed,
  password-manager friendly (`textContentType`, `autoComplete`).
* Capture **consent** with versioned documents (ToS, Privacy), marketing opt-in separate and
  unticked; **age gate** where required (Game especially; COPPA/age-appropriate design rules
  **(verify per market)**).
* Email/phone **verification** via link (universal link) or OTP code; resend with cooldown;
  unverified accounts have limited capabilities.
* **Username/handle availability** checks are rate-limited and don't leak account existence of emails.
* Abuse controls: rate limits, device attestation signals (Play Integrity / App Attest **(verify)**),
  CAPTCHA/risk challenge only when risk is high.
* After registration: restore intended deep link, start onboarding, request push at a good moment.

### 7.4 Login

* Methods: email/password; **Sign in with Apple** (required on iOS when other third-party
  social logins are offered, with equivalent privacy-preserving options **(verify guideline)**);
  Google; optional **passkeys** (platform authenticators) **(verify library maturity)**;
  enterprise SSO via **OIDC Authorization Code + PKCE in the system browser** (`expo-auth-session` +
  `expo-web-browser`), never an embedded WebView.
* **MFA**: TOTP/passkey/push-approval for sensitive roles and actions; step-up for payments,
  account deletion, password change.
* Throttling and lockout are server-side; the client shows generic errors
  ("check your credentials") and `Retry-After` hints; uniform timing.
* **Biometric app lock** (optional): `expo-local-authentication` gates the *app UI/tokens*
  (unlock to use the stored refresh token); biometrics never replace server authentication.
* **Session storage**: access token in memory; **refresh token in SecureStore**
  (Keychain/Keystore) with `AFTER_FIRST_UNLOCK_THIS_DEVICE_ONLY`-class accessibility on iOS and
  non-backed-up storage on Android; single-flight refresh, rotation with reuse detection (server),
  immediate local wipe on sign-out/suspension.
* **Multiple accounts/roles** (market seller vs buyer): active role switch resets caches and
  subscriptions; `{userType}` selects the experience only.

### 7.5 Forgot password and reset

| Step | Behavior |
|---|---|
| Request | `/forgot-password`: user enters email; **always show the same confirmation** (no account enumeration); rate-limit per IP/device/account; cooldown on resend |
| Delivery | Email contains a **short-lived, single-use link** (universal/app link to `/reset-password?token=…`) and/or a numeric code for in-app entry; link works if the app is installed, otherwise lands on a web fallback |
| Reset screen | Token validated server-side on submit; new password with policy feedback; confirm field optional; success → **all sessions revoked**, user signed in or sent to login; notify by email |
| Security | Token never logged or stored; clipboard/deep-link input treated as untrusted; reset tokens bound to the account and expire quickly; MFA required if enabled; passkeys remain valid |
| UX | Handles app cold start from the link, expired/used token states, offline, and keyboard avoidance; accessible error messaging |

### 7.6 Permission primers (all variants)

Show a short in-context explanation **before** the OS prompt, only when the feature is about to
be used; if denied, provide a settings deep link and a degraded path:

| Permission | Trigger moment | Degraded path |
|---|---|---|
| Notifications | After first valuable event (order placed, first catch, first avatar chat) or via onboarding opt-in | In-app inbox; periodic refresh |
| Location (Game) | When opening the map | Manual region selection / browse-only mode |
| Background location (Game) | Only if the user enables a feature that needs it | Foreground-only play |
| Camera | AR mode, barcode scan, selfie avatar | Non-AR mode, manual entry, preset avatars |
| Microphone | First voice interaction | Text chat |
| Photos/files | Review photos, avatar upload | Camera or skip |

### 7.7 Account lifecycle and compliance

* **In-app account deletion** (initiate, re-authenticate, grace period, confirmation) is
  required by the stores **(verify)** and by privacy law; deletion cascades server-side to
  devices/push tokens, avatar assets, location history and caches.
* **Data export** and consent management in settings; marketing opt-out honored in push.
* Terms/privacy version changes trigger re-consent flows.

---

## 8. Push notifications (all three variants)

### 8.1 Architecture

```text
App (expo-notifications) ──permission──► OS
   │ device token (FCM on Android, APNs on iOS via FCM or direct)
   ▼
POST /api/v1/app/devices {platform, token, provider, appVariant, appVersion, locale, tz, prefs}
   ▼
Backend `notifications` slice: device registry, preferences, templates, scheduler,
   dedupe/collapse, rate limits → provider adapters (FCM HTTP v1, APNs) → delivery receipts
```

Decisions:

* **Client library:** `expo-notifications` (permissions, channels/categories, listeners,
  scheduling, native device token via `getDevicePushTokenAsync`). Bare projects may use
  `@react-native-firebase/messaging` with a display library (Notifee, or a maintained fork;
  **verify maintenance**) for advanced Android UI.
* **Provider:** your backend sends through **FCM HTTP v1** (the legacy FCM endpoint was
  retired in 2024) and APNs; store the provider with each token because Expo push tokens and
  native tokens are different types. Using the Expo Push Service is acceptable for speed but adds
  a processor to your data map; document the choice.
* **Remote push needs a development build**: Expo Go cannot receive remote push on Android since
  SDK 53.
* **Android 13+ requires the `POST_NOTIFICATIONS` runtime permission**; without it notifications silently
  never appear.

### 8.2 Notification contract

```json
{
  "v": 1,
  "type": "market.order.shipped",
  "category": "transactional",
  "deepLink": "heimdall://app/market/orders/ord_123",
  "correlationId": "…",
  "collapseKey": "order:ord_123",
  "ttl": 86400,
  "priority": "high",
  "title": "…", "body": "…",
  "data": { "orderId": "ord_123" }
}
```

Rules:

* **Minimal, non-sensitive payloads** (lock screens are public): no PII, no tokens, no full
  addresses or message bodies; the app **fetches details after the tap**.
* `deepLink` follows the route table and is validated against an allowlist before navigation.
* Versioned types; unknown types are ignored safely and counted.
* **Data-only (silent) messages** are for sync hints, not reliable delivery; the OS and OEM battery
  managers may delay or drop them. Notification messages are used for user-visible alerts.
* `collapseKey`/`tag` to replace stale alerts; `ttl` per type (a "price drop" expires; an "order
  shipped" persists longer).

### 8.3 Categories and channels

| Category (Android channel / iOS category) | Examples | Default |
|---|---|---|
| `transactional` | Order status, payment issues, security alerts, password reset notices | On, high importance |
| `account-security` | New login, password changed | On, cannot be disabled |
| `game-events` | Nearby spawns, raids, event start | On (user-tunable), default importance |
| `social` | Friend requests, trades, mentions | On |
| `avatar` | Avatar session invites, scheduled sessions | On |
| `promotions` | Offers, price drops | **Off until opted in** |

Channels are created on first launch (Android 8+); users manage them in-app (preferences stored
server-side) and in OS settings; quiet hours and per-category toggles are supported.

### 8.4 Token lifecycle

* Register **after login** (not before), on every app start (to detect refresh) and on token
  change events; send `appVariant`, version, locale, timezone, and capability info.
* **Unregister on logout** and on account deletion; the server also prunes tokens on provider errors
  (`UNREGISTERED`, `NotRegistered`) and by inactivity.
* One token per install; reinstall yields a new token; the server de-duplicates by device ID.
* Permission state changes (granted/denied/provisional) are reported so the server stops
  sending to denied devices.
* iOS: standard, **provisional** (quiet delivery) as an option for early-opt-in; critical alerts
  only with the special entitlement **(verify)**.

### 8.5 Handling states

| App state | Behavior |
|---|---|
| Foreground | Show an in-app banner/toast via `Notifier`, update badges/inbox; don't duplicate OS alerts unless configured |
| Background | OS displays; tap → deep link → route after auth |
| Killed/cold | Tap launches app; initial notification retrieved; queued until bootstrap/auth done |
| Signed out | Show generic alert content; after login, route to the target or fall back to home |

### 8.6 Low-end and OEM realities

* Aggressive OEM battery managers (common on low-end Android) can kill background work and delay
  messages: use **high-priority FCM only for time-sensitive** alerts, keep payloads small, avoid
  relying on background JS, and offer a one-time in-app guide to battery settings where delivery
  fails repeatedly.
* Never depend on push for correctness: the app **reconciles on foreground** and via realtime/polling.
* Foreground services are used only where policy-allowed (see Game background location).

### 8.7 Server-side requirements (backend spec)

Device registry per user/tenant; preferences; template and i18n rendering; scheduling with timezone; per-user
rate limits and collapse; idempotent send jobs; delivery/open receipts feeding analytics; dead-token pruning; audit trail for
transactional and security notifications; kill switch per category. Implements the `notifications` slice following
`PLAYBOOK_BACKEND.md` (batch layer with Redis/Dragonfly coordination and idempotency).

---

## 9. Library selection (works on low- and mid-range phones)

### 9.1 Selection rules

1. **Prefer Expo SDK modules and libraries that declare New Architecture support** (legacy is gone).
2. **Native over JS-bridge-heavy**: choose libraries whose work happens on native/GPU/background threads (e.g., Filament,
   FlashList, MMKV) rather than the JS thread.
3. **Every dependency passes a weight check** on the Low pool: app size delta, cold-start delta, memory delta, and
   maintenance health (recent releases, open critical issues, bus factor, license). Record it in
   `docs/dependency-register.md`.
4. **No library without a fallback or a replacement plan** if it is unmaintained.
5. **Avoid CSS-in-JS runtimes and heavy UI kits** on low-end; prefer StyleSheet + tokens or a compile-time styling tool
   (**verify** impact on your pool).
6. **Pin versions** with the SDK's compatibility set (`expo install`, `expo-doctor`); upgrade SDKs deliberately. Pin `react`
   and `react-native` **exactly** (no range): a newer `react` patch can fail `expo-doctor` against the SDK's compatibility set, so
   a floating range breaks the build on a patch bump nobody chose. Note: Expo
   SDK 56 had a Hermes V1 memory regression affecting apps using Reanimated/worklets, fixed in later patch releases (SDK 57.0.9 and
   newer): never run an unpatched SDK on the Low pool.

### 9.2 Curated stack

| Area | Library (default) | Low/mid-end notes | Alternatives / cautions |
|---|---|---|---|
| Runtime | **React Native 0.88 + Expo SDK 58**, Hermes V1 | New Architecture only; Hermes V1 default | Verify patch level for known regressions |
| Navigation | **Expo Router** | File-based, deep linking built in | React Navigation underneath; a plain app entry + the typed route table is enough for a small app (section 6) |
| Lists | **@shopify/flash-list v2** | Recycling and virtualization; New Arch | FlatList only for tiny lists |
| Images | **expo-image** | Disk/memory cache, blurhash, WebP/AVIF, downscaling | Avoid un-sized remote images |
| Animation | **react-native-reanimated 4 + worklets**, gesture-handler | Use sparingly on Low: few simultaneous worklets; verify memory on the pool | RN `Animated` with native driver for simple fades |
| 2D/particles | **@shopify/react-native-skia** | GPU 2D; budget memory | Pre-rendered sprites on Low |
| Server state | **@tanstack/react-query** | Dedupe/caching reduces network and renders | — |
| Client state | **zustand** | Selector subscriptions | Avoid large context trees/Redux where unneeded |
| KV storage | **react-native-mmkv** | Sync JSI, very fast | AsyncStorage is slower |
| Secrets | **expo-secure-store** | Keychain/Keystore | Check value size limits **(verify)** |
| DB | **expo-sqlite** (+ Drizzle optional) | WAL, bounded size | WatermelonDB for very large sync sets |
| Forms/validation | **react-hook-form + zod** | Low re-render form model | — |
| Networking | `fetch` via the SDK; `@react-native-community/netinfo` | Retry/backoff, abort | Avoid chatty polling |
| Auth | **expo-auth-session**, **expo-web-browser**, **expo-apple-authentication**, **@react-native-google-signin/google-signin**, **expo-local-authentication**, **expo-crypto**; passkeys via a maintained native module **(verify)** | System browser + PKCE | No embedded login WebViews |
| Push | **expo-notifications** (+ FCM HTTP v1 server) | Native tokens; channels | `@react-native-firebase/messaging` + Notifee/fork for bare/rich Android UI **(verify maintenance)** |
| Maps (Game) | **@maplibre/maplibre-react-native** or **@rnmapbox/maps** | Native vector tiles; entities as GL layers; cache tiles; cap pitch/3D buildings | `react-native-maps` struggles with many markers |
| Location | **expo-location + expo-task-manager** | Adaptive accuracy, geofencing, foreground service rules | Fused/significant-change modes to save battery |
| Camera | **react-native-vision-camera** (frame processors) / **expo-camera** | Disable processors on Low | — |
| On-device ML | **react-native-fast-tflite** (TFLite + GPU/NNAPI/CoreML delegates) **(verify)** | Only on Mid/High; fallback to server/none | Keep models small, quantized |
| 3D (RN-native) | **react-native-filament** (glTF/GLB, native render thread, Metal/Vulkan/GLES, Bullet physics) | Best RN-native option for viewers and light scenes; confirm maintenance and 16 KB page-size support **(verify)** | — |
| 3D (JS-driven) | `@react-three/fiber/native` + `expo-gl` | GL calls on the JS thread, no JIT in Hermes: only for very light scenes on Mid/High | Don't use for the Low tier or heavy animation |
| 3D (engine) | **Godot** via `@borndotcom/react-native-godot` (LibGodot, own thread; use the Compatibility renderer for low-end) ; **Unity** via `@azesmway/react-native-unity` (AR Foundation; heavy) | Embed only in Game/Avatar variants that need it; on-demand asset packs | Check maintenance, build tooling and 16 KB alignment **(verify)** |
| WebGPU | `react-native-wgpu` | Experimental; not for production low-end | Revisit yearly |
| Audio | **expo-audio** (replaces expo-av) | Low-latency playback/recording | `react-native-audio-api` for graph-style audio **(verify)** |
| Video | **expo-video** | Hardware decode | — |
| Realtime voice | **@livekit/react-native** (WebRTC) or `react-native-webrtc` | Echo cancellation, low latency | Cloud STT/TTS streaming over WebSocket |
| STT/TTS | Cloud streaming for quality; **whisper.rn** (small models) for offline on Mid/High **(verify)**; `expo-speech` as system fallback | On-device models are heavy for Low | Lip-sync needs timing data (section 12) |
| Payments | **@stripe/stripe-react-native** (PaymentSheet, Apple Pay, Google Pay) | PCI scope minimized | Digital goods: StoreKit/Play Billing via `react-native-iap` or RevenueCat |
| Search | Backend-driven (Algolia/Typesense/Meilisearch) | Thin client, debounce, cached suggestions | — |
| i18n | **i18next + expo-localization** | Lazy-load locales | — |
| Observability | **@sentry/react-native** or Crashlytics; `react-native-performance` | Sample rates tuned for Low | — |
| Flags/config | Remote config (Firebase RC, GrowthBook, Statsig) + backend config | Device-tier overrides | — |
| Testing | **Vitest** (or Jest) for the framework-free layers, **@testing-library/react-native** for views, `node --test` for contract evals, **Maestro** (or Detox), **Reassure**, **Flashlight**, MSW | Real-device farm | Below the view layer there is usually nothing to render: a node environment is enough (section 17.1) |
| Asset tooling | **gltf-transform / gltfpack, toktx (KTX2/Basis), meshopt/Draco** | Mandatory in CI for 3D assets | — |

**Pinned baseline.** Start from the set the reference kit locks and verified against
Expo SDK 58 (`frontend-mobile/package.json`, `frontend-mobile/app/package.json`,
`frontend-mobile/MEMORY.md`): Expo **~58.0.24**, React **19.3.0** (exact),
React Native **0.88.1** (exact), TypeScript **~6.0.3**, Vitest **~5.1.0**,
zod **^4.6.7**, Node **≥ 24**, pnpm **10.x**. Copy them verbatim; move one at a
time with `expo install` + `expo-doctor`, and re-measure the Low-pool budgets
(section 3.2) in the same pull request.

### 9.3 Libraries and patterns to avoid on low-end

* WebView-hosted games or 3D (Phaser/three in a WebView run several times slower on Android than on desktop).
* Large animated-gradient/blur stacks, many simultaneous Lottie loops, full-screen blur on lists.
* Rendering game entities or product tiles as hundreds of absolutely-positioned Views.
* JS-thread physics/animation loops; per-frame JSON bridging; polling timers at high frequency.
* Unbounded image caches, uncompressed textures, 4K assets.
* Heavy state libraries triggering app-wide re-renders; context values that change every frame.

---

## 10. Variant A — Game (location-based 3D, Pokémon-GO-style)

### 10.1 Slices (mirror backend slices)

`map`, `spawns`, `encounters`, `catching`, `inventory`, `collection`, `events`, `social` (friends, trades),
`shop` (digital), `progression`, `safety`. Backend slice `game` (and `notifications`, `identity`) owns the
rules; permissions like `game.encounters.create`, `game.inventory.read`.

### 10.2 Server-authoritative design

* **The server owns**: spawn generation, encounter outcomes, RNG, inventory/economy, event schedules, anti-cheat
  decisions. The client sends **intents** (`startEncounter`, `throwBall` with minimal inputs) and renders results.
* **Anti-cheat and integrity**: server-side plausibility (speed limits, teleport detection, impossible travel,
  spawn proximity), rate limits, signed encounter tokens, replay protection; device attestation (Play Integrity /
  App Attest **(verify)**) as a **risk signal**; mock-location detection on Android as a signal, not a sole
  verdict; progressive penalties and appeals.
* **Location privacy**: precise coordinates are sensitive personal data: send only what gameplay needs, coarsen
  or fuzz in social views, never expose other players' exact positions, limit retention, explain in the primer and
  privacy policy.

### 10.3 Map and world rendering

* Use **MapLibre/Mapbox GL layers** (symbol/circle/fill-extrusion from GeoJSON sources) for spawns, POIs and the
  player marker: thousands of features cost almost nothing, whereas RN Views over a map don't scale.
* Tile strategy: vector tiles with an offline cache for the player's area; cap zoom/pitch; disable 3D buildings and
  heavy labels on Low; style simplified per tier.
* The "player avatar" and creatures on the map are **2D sprites/billboards** by default; 3D only in the encounter
  screen and optional 3D map mode on higher tiers.
* **Spawn sync** via viewport queries (`GET /api/v1/app/game/map/nearby?bbox=…&cursor=…`) plus realtime events
  (`game.spawn.created/expired`), snapshot + stream like the desktop playbook §8.

### 10.4 Location and battery

| Mode | Use | Notes |
|---|---|---|
| Foreground, high accuracy | Map open | Adaptive interval by speed; reduce when stationary |
| Foreground, balanced | Walking | Distance filter (e.g., tens of meters) |
| Geofences / significant-change | Spawn alerts, passive tracking | Cheapest; OS-driven |
| Background (optional) | Only for features that need it (e.g., adventure sync/step tracking) | Android foreground service with persistent notification; iOS "Always" permission requires strong justification and review; offer foreground-only play by default |

Rules: batch uploads, send deltas not streams, pause when stationary/low battery, respect thermal and
power-saver states, and prove battery impact on the Low pool each release. Background location use must be justified
in store declarations and onboarding **(verify store policies)**.

### 10.5 Encounter screen (3D)

* Pipeline: server returns an encounter spec (creature ID, level, modifiers, tokens); the client loads the
  creature model for the current **tier** (LOD/texture pack), plays idle/attack animations, handles throw input
  (gesture → intent), shows result from the server.
* **Tiers:** Low: pre-rendered sprite-sheet or turntable of the 3D model (flipbook), parallax background, no AR; Mid:
  low-poly 3D via the Filament adapter with baked lighting, 30 fps cap; High: richer 3D, optional AR.
* **AR mode** is **optional and capability-gated**: requires ARCore/ARKit support and enough performance; fall back to
  the non-AR encounter with a still/camera-less background. Implement AR through the Unity adapter (AR Foundation) or
  a custom native module (ARKit/ARCore via the Expo Modules API); ViroReact-style libraries need a maintenance check
  **(verify)**.
* Throw/catch mechanics run in the engine/scene layer at the scene's frame rate; the RN shell only receives events
  (`encounter.thrown`, `encounter.result`).
* Safety: in-app reminders to be aware of surroundings, speed warnings when moving fast (driving), cooldowns,
  parental controls and age-appropriate defaults.

### 10.6 Economy, IAP and compliance

* Digital items via StoreKit/Play Billing; server validates receipts and grants entitlements; idempotent.
* If loot-box-style mechanics exist, disclose odds and respect regional restrictions **(verify)**; spending limits and
  parental controls for minors.
* Moderation and reporting for any user-generated names/chat.

### 10.7 Game realtime

WebSocket (or SSE) for nearby events, raids, trades; reconnect with backoff; resync after gaps; pause in background and
rely on push for time-critical alerts; never trust client time or position. Protocol envelope and state machine as in
`PLAYBOOK_DESKTOP_QT.md` §8 (adapted to mobile lifecycle: socket drops when the app is suspended).

### 10.8 Performance specifics (Game)

* GPU/engine work off the JS thread; entity updates batched at ≤ 10–20 Hz for the map; throttle JS→native bridges.
* Asset delivery: base app + **on-demand packs** per region/creature set (Play Asset Delivery / iOS On-Demand
  Resources or your own CDN with signed manifests); prefetch on Wi-Fi.
* Memory ceilings per tier enforced by the governor; unload encounter assets on exit.
* Test on the Low pool for sustained 20-minute sessions: temperature, battery, frame time, memory.

---

## 11. Variant B — Market (Amazon-style marketplace)

### 11.1 Slices

`catalog`, `search`, `product`, `cart`, `checkout`, `payments`, `orders`, `tracking`, `reviews`, `wishlist`, `promotions`,
`recommendations`, `account` (addresses, payment methods), `seller` (seller tools: listings, inventory, orders, payouts),
`support` (returns, disputes). Backend slices: `catalog`, `orders`, `payments`, `sellers`, … with permissions like
`market.cart.update`, `market.orders.read`, `market.listings.create`.

### 11.2 Screens and flows

Home (personalized rails) → Search (suggestions, recents, scan) → Results (facets, sort, infinite scroll) → Product detail (gallery, variants,
availability, delivery estimate, reviews, Q&A, seller info) → Cart → Checkout (address, shipping, payment, review) → Order confirmation →
Orders/tracking → Returns/support. Seller mode: dashboard, listings (create/edit with photos), orders, messaging, payouts.

### 11.3 Performance and low-end

* **FlashList v2** for results and rails with fixed-aspect tiles; image placeholders (blurhash), `expo-image` with
  CDN-resized variants (request the exact size/format, WebP/AVIF), disk cache with quota.
* **Server-driven UI sparingly**: home layouts from a typed server response (rails of known types) so layouts change
  without releases but stay native and validated; no arbitrary remote code.
* Paginate with cursors; prefetch the next page near the end; cancel stale requests; debounce search input (e.g., 200–300 ms);
  cache suggestions and facets.
* Avoid heavy shadows/blur on cards; flatten view hierarchies; keep list item components memoized and light.
* Product gallery: progressive loading, pinch-zoom with gesture-handler/native image zoom; **no 3D/AR product view by default**; if
  offered, lazy-load a GLB with strict budgets and the Filament adapter only on Mid/High, with a 2D fallback.
* Skeletons instead of spinners; optimistic cart updates with rollback.
* Cold start: render the cached home immediately, then refresh.

### 11.4 Cart, checkout and payments

* **Server-authoritative pricing**: the client shows prices but the server computes totals, taxes, shipping and promotions at
  checkout; cart items revalidated before payment; price/stock changes shown clearly.
* **Idempotent checkout**: `Idempotency-Key` on order creation; safe retries; no double charges.
* **Stripe PaymentSheet** with Apple Pay/Google Pay for physical goods; the app never touches raw card data (PCI scope minimized, SAQ A-class
  **(verify with your QSA)**); 3-D Secure/SCA handled by the SDK; payment intent created server-side.
* **Digital goods** (subscriptions, in-app items) use the store billing APIs; physical goods and services may use external payment
  **(verify current store rules)**.
* Address entry with validation/autocomplete (provider SDK) and saved addresses; **secure screens**: block screenshots on payment screens
  where appropriate (Android `FLAG_SECURE`, iOS privacy overlays).
* Fraud/risk: server-side scoring, device signals, velocity limits; step-up auth for high-risk orders.

### 11.5 Search

Backend search engine (typo tolerance, facets, synonyms, ranking, personalization) exposed through
`GET /api/v1/app/market/search?q=…&filters=…&cursor=…`; the client handles suggestions, recents, voice (optional) and barcode scan
(`expo-camera`/VisionCamera) → product lookup. Search analytics feed ranking (privacy-safe).

### 11.6 Reviews, wishlist, notifications

Verified-purchase reviews, photo uploads (compressed on-device before upload, EXIF stripped), moderation and abuse reporting;
wishlist with price-drop alerts (`promotions` category, opt-in); order-status and delivery push (`transactional`).

### 11.7 Seller tools (if in scope)

Role-gated experience (`/app/market/seller/...` or `{userType}=partner`), listing creation with image pipeline, inventory, order
management, payouts onboarding (KYC through the payment provider's hosted flow), messaging; all authorization server-side.

### 11.8 Compliance specifics

Consumer-protection rules (pricing display, returns, delivery promises), marketplace obligations (seller transparency, illegal-content
reporting) **(verify per market)**, tax display, accessibility, privacy (marketing consent), PCI via provider, review authenticity.

---

## 12. Variant C — Avatar (3D, voice and movement)

### 12.1 Slices

`avatars` (creation, selection, customization), `sessions` (conversation/performance sessions), `voice` (capture, STT, TTS), `performance`
(lip-sync, gestures, emotions), `tracking` (optional camera-driven motion), `wardrobe` (items), `safety`.
Backend slice `avatar` (assets, entitlements, sessions) plus an AI/conversation service behind it; permissions like
`avatar.sessions.create`, `avatar.avatars.update`.

### 12.2 Avatar formats and sourcing

* **Open formats**: glTF/GLB, **VRM 1.0** (humanoid rig, standardized expressions), meshopt/Draco compression, KTX2 textures.
* **Provider-agnostic**: `AvatarProviderPort` with adapters (in-house artist-made library, VRoid/VRM imports, selfie-to-avatar vendors such as
  MetaPerson/Avaturn). **Ready Player Me shut down on 31 January 2026**, so design for vendor loss: store exported GLB/VRM in your own storage,
  keep a manifest of source/licence, never depend on a vendor runtime at play time.
* **Selfie-to-avatar** (optional): treat the photo and derived face geometry as **biometric-adjacent sensitive data**: explicit consent, process
  on-device or via a vetted processor with a DPA, delete the raw photo after generation, short retention, no secondary use **(verify legal basis
  per region, e.g., GDPR/BIPA-style rules)**.
* **Budgets per tier** (starting points): Low ≤ ~10k triangles, ≤ 25 bones, 1 material, 512 px textures (KTX2/ETC2/ASTC); Mid ≤ ~25k tris, ≤ 60
  bones, 1024 px; High ≤ ~50k tris, 2048 px. Facial expressions via ≤ 15–20 morph targets (visemes + basic emotions) on Low/Mid.

### 12.3 Rendering

* **Primary renderer**: `react-native-filament` (native thread) for avatar viewing/conversation; **Godot** adapter when richer scenes are needed;
  **2D fallback** on the Low tier (pre-rendered video/sprite avatar with audio-driven mouth frames or a lightweight Skia-based 2D rig).
* One avatar, one light setup, baked/simple lighting, no real-time shadows on Low/Mid; render scale and 30 fps cap from the governor; pause rendering
  when the app is backgrounded or the avatar is off-screen.
* Keep the scene's animation update on the **native/engine side**; RN sends high-level commands.

### 12.4 Voice pipeline

```text
Mic → AudioPort (echo cancel, noise suppress, VAD) ─► STT (cloud streaming | on-device, per tier/privacy) ─► text
text ─► backend conversation service (LLM + safety) ─► "performance script"
performance script ─► TTS (cloud neural voice with timing data | system TTS fallback) ─► audio + timeline
audio + timeline ─► PerformancePlayer (audio clock is master) ─► SceneEngine (visemes, gestures, emotion)
```

* **Latency budget** (conversational feel): capture→first partial transcript ≲ 300–500 ms, response start ≲ 1.0–1.5 s;
  stream everything (STT partials, LLM tokens, TTS chunks); barge-in support (interrupt the avatar when the user speaks).
* **Transport**: WebRTC (LiveKit or equivalent) for real-time audio on good networks; WebSocket streaming as the simpler
  fallback; HTTP for non-realtime features. Adaptive bitrate; graceful degradation to text chat.
* **Audio session management**: handle route changes (Bluetooth/headphones/speaker), interruptions (calls), background audio policy, and
  permissions; mute indicator; push-to-talk option.
* **Privacy**: show when the mic is live, never record by default; transcripts/audio retention per policy with user controls; voice data is
  sensitive (voiceprints): no speaker identification/cloning without explicit, revocable consent.
* **AI disclosure and safety**: clearly label the avatar as AI where it converses; content-safety filters server-side; report/block tools; age-appropriate
  modes; transparency obligations for AI systems may apply (EU AI Act timelines, **verify**).

### 12.5 Performance script (the contract that drives movement)

The backend emits a typed, versioned script; the client **plays it, it doesn't improvise**:

```json
{
  "v": 1, "id": "perf_…", "text": "…", "audio": { "url": "…", "codec": "opus", "durationMs": 4200 },
  "visemes": [ { "t": 0, "v": "sil" }, { "t": 120, "v": "aa", "w": 0.8 } ],
  "emotions": [ { "t": 0, "name": "happy", "w": 0.6, "rampMs": 300 } ],
  "gestures": [ { "t": 800, "name": "wave", "hand": "right" } ],
  "gaze": [ { "t": 0, "target": "camera" } ]
}
```

* **Lip-sync sources** (by quality/cost): (a) cloud TTS with viseme/alignment timestamps (best); (b) server-side phoneme alignment; (c)
  on-device amplitude/spectral estimation (cheap, lower fidelity) as the Low-tier fallback.
* **Gestures/emotions** come from a **fixed animation vocabulary** (named clips retargeted to the VRM/humanoid rig, blended with
  cross-fades); LLM output is constrained to this vocabulary (no free-form motion data from untrusted sources).
* The **audio clock is the master timeline**: visemes/gestures are scheduled against audio playback position; drift is corrected, late events dropped.
* Idle behaviors (blink, breathing, micro-movements, gaze) run locally and cheaply.

### 12.6 Optional camera-driven movement

* Face/pose tracking (head pose, blendshapes) is **Mid/High only**, opt-in, with the camera preview hidden or minimized, using on-device ML
  (TFLite/MediaPipe-class models via VisionCamera frame processors **(verify packages)**) at a low frame rate (e.g., 15–30 fps), mapped to
  VRM expressions. Native ARKit face tracking (iOS) can be added through a custom native module; no equivalent universal Android API.
* On Low devices or when thermal/battery constraints apply, disable tracking and use script-driven + idle animation only.
* Raw camera frames never leave the device; only derived parameters may be sent, with consent.

### 12.7 Sessions and entitlements

Session lifecycle (`POST /api/v1/app/avatar/sessions`), usage limits and entitlements enforced server-side, cost controls for STT/TTS/LLM, reconnect
and resume, push for scheduled sessions, wardrobe items as digital goods (IAP) with server-validated entitlements.

---

## 13. Scene engines, 3D assets and the asset pipeline

### 13.1 `SceneEnginePort` and tiers

| Tier | Adapter | When |
|---|---|---|
| **T0** | `TwoDFallbackAdapter` (sprites, flipbooks, Skia/expo-image) | Low devices, crash-safe mode, thermal emergency, unsupported GPU |
| **T1** | `FilamentAdapter` (`react-native-filament`) | Mid-range 3D viewers/encounters/avatars, simple scenes |
| **T2** | `GodotAdapter` (Compatibility renderer for low-end, Mobile renderer on capable GPUs) | Game scenes needing a real engine/physics/scripting |
| **T3** | `UnityAdapter` (AR Foundation) | AR-heavy high-end experiences only |

* Adapters implement the same commands/events; scene content is **data** (scene specs/assets), not logic in the shell.
* **Fallback chain on failure**: engine init error or repeated frame-time breach → step down a tier (persisted) and report.
* Engines run on **their own render thread**; the RN shell and engine communicate by **message passing** (commands in, events out), never per-frame bridging.
* Engines are loaded **lazily** when a scene screen opens and **disposed** on exit; keep one engine instance alive only while needed.
* Each engine package is its own optional module so variants without 3D don't ship it.

### 13.2 Engine notes

* **react-native-filament**: native Filament (physically based, Metal on iOS, Vulkan/OpenGL ES on Android) with its own render threads and
  Bullet physics; loads GLB; good fit for avatar and light creature scenes. Verify maintenance cadence and 16 KB page-size alignment before committing.
* **react-three-fiber/native + expo-gl**: convenient for teams with three.js skills, but WebGL calls are issued from the JS thread and Hermes has no
  JIT, so per-frame JS is expensive on low/mid devices; use only for light, mostly static scenes and never as the Low-tier solution.
* **Godot embedding**: runs on a separate thread and can render into RN views; the **Compatibility** (OpenGL ES 3) renderer is the right target for
  low-end Android; engine footprint is smaller than Unity's; verify export tooling, build times and library maintenance.
* **Unity as a library**: strong AR and tooling; larger binary and memory cost; integrates as a view with message passing; reserve for
  AR-first, high-tier experiences and keep it out of the Low path.
* **WebView 3D/Phaser**: not allowed on the Low/Mid path (large slowdowns on Android WebViews).

### 13.3 Asset pipeline (CI-enforced)

1. Source art (DCC) → export glTF/GLB/VRM.
2. **gltf-transform/gltfpack**: mesh simplification and **LOD chains**, meshopt/Draco compression, animation quantization, dedupe, prune.
3. **Textures → KTX2 (Basis Universal)** transcoded to ETC2/ASTC (Android) and ASTC/BC (iOS) at tiered resolutions; atlas where possible; mipmaps.
4. Validate with **glTF-Validator** and **budget checks** (triangles, bones, materials, texture sizes, draw calls) per tier; fail the build on violations.
5. Package into **versioned asset packs** with content hashes and a **signed manifest**; host on CDN; clients verify hash/signature.
6. Delivery: base pack in app; **on-demand packs** (Play Asset Delivery / iOS On-Demand Resources or CDN) per tier and region; resumable downloads, Wi-Fi preference, disk quota and eviction (LRU), integrity check, rollback to the previous pack.

### 13.4 Memory discipline

Texture memory dominates: use GPU-compressed formats, cap resolutions by tier, share materials, unload scenes when leaving, avoid duplicating
models in RN-side memory, watch iOS memory warnings and Android `onTrimMemory`, and keep peak process memory under the tier ceiling (leave
headroom for the OS low-memory killer).

---

## 14. Realtime, connectivity and background behavior

* Realtime follows the optional-transport model of `PLAYBOOK_DESKTOP_QT.md` §8: `RealtimeMode = auto | websocket | sse | polling | off`,
  snapshot + stream, at-least-once with dedupe by event ID, resume tokens, resync after gaps, per-topic authorization server-side.
* **Mobile specifics**: sockets are suspended/killed in background → reconnect on foreground; use **push as the wake-up channel**; use
  `NetInfo` for reachability and switch Wi-Fi/cellular; backoff with jitter; battery-aware (pause when the screen is off unless a feature requires it).
* **Background tasks**: use OS-sanctioned mechanisms only (`expo-task-manager`, background fetch with its limits, foreground services where
  justified); no polling loops; design for **no background execution** as the default.
* **Network quality adaptation**: detect slow/metered connections, lower image quality and disable prefetch, defer large asset packs to Wi-Fi.

---

## 15. Security, privacy and store compliance

### 15.1 Baseline (mobile)

Follow `PLAYBOOK_SECURITY.md` and **OWASP MASVS** (verification levels) with MASTG-style tests:

| Area | Controls |
|---|---|
| Storage | Secrets only in Keychain/Keystore (SecureStore); Android `allowBackup=false` (or targeted backup rules); no secrets in MMKV/SQLite/logs; encrypt sensitive local DB fields (SQLCipher or field-level keys in the keystore) |
| Network | TLS 1.2+; Android network security config (no cleartext, no user-CA trust in release unless needed); optional certificate pinning with a rotation plan (pin a backup key; kill switch) |
| Auth | Section 7; refresh rotation; biometric gate; step-up; session list/revoke |
| Code/binary | R8/ProGuard for native/Java layers; Hermes bytecode is not a secret store; no API keys or secrets in the bundle; license-restricted keys (Maps) are scoped and rotatable |
| Platform trust | Play Integrity API / App Attest as risk signals, evaluated server-side; root/jailbreak signals are advisory; never block solely on client-side checks |
| Deep links/IPC | Allowlisted routes, validated params, no exported components without need; intent/URL scheme hijack protection (App Links/Universal Links) |
| UI | `FLAG_SECURE`/privacy overlays on sensitive screens; clipboard minimization; hide content in app switcher where needed |
| Third-party SDKs | Inventory, minimal permissions, privacy manifests, vendor review (see `PLAYBOOK_AUDIT.md` §12) |
| Supply chain | Lockfile, frozen installs, release-age cooldown for new package versions, SCA, SBOM, signed builds, EAS secrets management, CI hardening (`PLAYBOOK_SECURITY.md` §11) |
| Native engines | Review engine/plugin sources and licenses; keep versions current; scan binaries |

### 15.2 Privacy

* Data minimization per variant; **location, microphone, camera, voice and face-derived data are sensitive**: explicit consent, purpose
  limitation, retention limits, deletion on account removal, regional rules (GDPR/CCPA/LGPD/others) **(verify)**.
* **Children/teens**: age gate and age-appropriate design for Game and Avatar; parental consent where required; restrict chat/voice and purchases by age.
* **Store privacy artifacts**: iOS privacy manifests and nutrition labels, Play Data safety forms: generated from the dependency register and kept in sync with actual SDK behavior.
* Analytics/telemetry: opt-out/consent as required; no precise location or content in analytics; hashed identifiers; retention limits.

### 15.3 Store policy checklist (verify each release)

| Topic | Requirement to confirm |
|---|---|
| Sign in with Apple | Offered when other third-party social logins exist (iOS) |
| Account deletion | In-app initiation and web/support path |
| Permissions rationale | Accurate usage strings; background location justification (Game) |
| Payments | Digital goods via store billing; physical goods via external processors; correct disclosures |
| UGC/AI content | Reporting, blocking, moderation, AI disclosure |
| Age rating | Questionnaire reflects chat, voice, purchases, location |
| Android targets | Target API level currency; edge-to-edge; 16 KB page size; foreground-service types; notification permission |
| Privacy forms | Data safety and privacy manifests match reality |
| Loot boxes | Odds disclosure/regional restrictions if applicable |

---

## 16. Observability

| Signal | Implementation |
|---|---|
| Crashes/ANRs | Sentry RN or Crashlytics with source maps/debug symbols uploaded privately; native crash handlers for engines; release/variant/tier tags |
| Performance | `react-native-performance` markers (startup phases, screen TTI), frame-drop counters, JS thread stalls, scene frame-time p50/p95, memory samples, thermal and battery state, governor decisions |
| Network | Request latency/error rates by endpoint and network type; realtime state transitions and lag; asset pack download success |
| Product | Funnels: onboarding → registration → verification → first value; checkout; encounter; session start; push opt-in and open rates |
| Tracing | `X-Correlation-Id` generated per user action; propagated through SDK/realtime/push payloads and shown on error screens |
| Diagnostics bundle | User-initiated, redacted: app/variant/version, device tier and capabilities, renderer/engine, governor profile history, permissions, push status, recent logs, recent errors with correlation IDs |
| Privacy | Sampling and consent aware; redaction filters; no PII/precise location in logs |

Server-side alerts watch crash-free rate per device class, startup time p95 on Low devices, checkout failures, push delivery failures, auth error spikes.

---

## 17. Testing strategy

Follows `PLAYBOOK_TESTS.md`; adds a `mobile` side:

```text
tests/<slice>/mobile/{unit,component,integration,contract,e2e,perf,a11y,security}/
tests/_platform/{architecture,device-perf,push,auth,store-readiness}/
```

### 17.1 Levels

| Level | Scope | Tooling |
|---|---|---|
| Unit | domain, application, ViewModels with fake ports | Vitest or Jest in a node environment — the domain and application layers hold no JSX and need no renderer; a ViewModel needs one only when it uses React hooks (`renderHook`, `@testing-library/react-native`), so prefer a plain state machine plus a hook in the views layer (section 4.1) |
| SDK/contract | SDK vs OpenAPI, notification payload schema, performance-script schema, route table parity, permission catalog | schema tests, MSW |
| Component | Views per ViewModel status; accessibility roles/labels | RNTL |
| Integration | ViewModel + SDK + SQLite/outbox, realtime transport conformance, push handling state machine (foreground/background/killed) with fakes | Vitest or Jest + fakes |
| **E2E** | Onboarding → register → verify → login → forgot → reset; permission denial paths; push tap deep links; market checkout; game encounter (mock GPS); avatar session (mock audio/script) | Maestro or Detox on emulators **and** real devices |
| Device performance | Cold start, list scroll (FlashList), scene frame time/memory/thermal on the Low/Mid pool | Flashlight (Android), Reassure (render regressions), Perfetto/Instruments traces, custom scene harness |
| Scene/asset | Asset budgets, GLB validity, tier fallback behavior, engine init failure injection | CI scripts, harness |
| Location/anti-cheat | Mock routes (walk, drive, teleport), poor GPS, permission changes | simulators, test server |
| Audio/voice | Latency measurements, barge-in, route changes, interruptions, lip-sync drift | harness with recorded audio |
| Push | FCM/APNs sandbox, token refresh, permission states, OEM-kill scenarios on pool devices, collapse/ttl behavior | device farm + test sender |
| Accessibility | TalkBack/VoiceOver flows, large text, contrast, reduce motion | manual + automated |
| Security | MASTG-style checks, storage inspection, deep-link fuzzing, TLS config, secrets scan, dependency/SBOM scan | MobSF-class tools, custom |
| Compatibility | OS versions, screen sizes, languages/RTL, offline/slow networks | matrix |
| Upgrade | Install N-1 → update → data/migration and token continuity | scripts |
| Store readiness | Permissions, privacy forms, listing assets, 16 KB alignment, target API | checklist/CI |

### 17.2 Rules

* **Tests run on the Low pool**, not only emulators or flagships.
* No production data; synthetic tenants/users; payment tests use provider test modes.
* Flaky tests are quarantined with owner and expiry (tests playbook).
* Every notification type and every route has a contract/e2e test.

### 17.3 CI matrix

Unit/integration on every PR; emulator e2e on PR (smoke) and nightly (full); **device-farm runs** nightly/pre-release; Low-pool performance suite
nightly and per release candidate; asset budget checks on every asset change.

### 17.4 Performance gates

Per release: cold start, TTI, list scroll jank %, scene p95 frame time, memory plateau, battery drain/hour (Game), crash-free rate on the Low pool.
Regression threshold (e.g., +10%) fails the build; results are stored as release evidence.

---

## 18. Build, release and operations

### 18.1 Builds and variants

* **EAS Build** profiles per variant × environment (`dev`, `staging`, `prod`); `APP_VARIANT` selects name, bundle IDs, icons, plugins, permissions,
  associated domains and push credentials.
* Android: AAB with per-ABI/density splits; arm64 primary; R8 shrinking; 16 KB-aligned native libs; on-demand asset packs for Game/Avatar.
* iOS: separate app records/capabilities per variant; privacy manifests; phased release.
* **Continuous Native Generation** (prebuild) keeps native projects reproducible; custom native code via the Expo Modules API/config plugins.

### 18.2 OTA updates (EAS Update)

* JS/asset-only changes; **never** change native dependencies or permissions via OTA; strict **runtime version** policy per variant.
* Channels per environment with staged rollout (percentage) and automatic rollback on health regression; Hermes bytecode diffing keeps updates small.
* Respect store rules on OTA usage; OTA is not a way to bypass review for functionality changes.

### 18.3 Release pipeline

| Stage | Gate |
|---|---|
| PR | Typecheck, lint, architecture rules, unit/component/contract tests, dependency/SBOM and secrets scans, asset budgets |
| Main | Variant builds (all three), emulator e2e smoke, size diff per variant, permission diff vs register |
| Nightly | Full e2e, device-farm run, Low-pool performance, soak (30-min sessions), push suite |
| Release candidate | All above + store-readiness checks, accessibility pass, security tests, evidence bundle |
| Release | Staged rollout (Play staged release, iOS phased release), health gates, remote kill switch ready |
| Post-release | Crash/ANR, startup p95 on Low, checkout/auth/push metrics; rollback plan (halt rollout, OTA fix, kill switch, higher version) |

### 18.4 Remote config and kill switch

Remote config (data only, schema-validated, signed or served from a trusted origin) controls: feature flags, device-tier overrides and blocklists,
quality profile defaults, onboarding content, realtime mode, push categories, minimum supported app version (force-upgrade screen),
and emergency kill switches per feature (AR, background location, voice, 3D, checkout provider).

### 18.5 Runbooks

Crash spike on a device model (blocklist + tier override), OEM push delivery failure, FCM/APNs outage or credential expiry, store rejection/appeal, bad OTA
(rollback), engine regression, asset pack corruption (rollback), payment provider incident, auth/IdP outage, location-policy review, privacy incident,
avatar-provider loss (switch provider), voice/LLM provider outage (degrade to text).

---

## 19. Governance, compliance and working with Claude Code

* **Ownership:** CODEOWNERS for `app.config.ts`, `eas.json`, native-module declarations, `packages/scene-engine/**`, push/auth adapters,
  `docs/permissions-register.md`, store metadata; security review for permission, SDK-inventory and auth/push changes.
* **Documents:** ADRs, dependency register, permissions register (each permission: why/variant/feature/fallback), device pool and tiers,
  support matrix, threat model, data-flow diagrams, privacy review, store submission checklist, runbooks.
* **Compliance mapping** (`PLAYBOOK_AUDIT.md`): SDLC controls via CI gates and reviews; vendor management for SDKs/providers (push, maps, payments,
  voice/LLM, avatar); privacy (location/voice/face data) and consent evidence; change management via staged rollouts; incident response runbooks; evidence
  bundle per release (builds, SBOM, test and performance reports, permission diffs, approvals). CRA applicability for distributed apps: assess with counsel
  **(verify)**.
* **Claude Code** (`PLAYBOOK_CLAUDE.md`): path-scoped rules for `packages/mobile-core/**`, `packages/feature-*/**`, `packages/scene-engine/**`;
  integrator role owns `app.config.ts`, native deps, lockfiles, CI; skills: `new-mobile-feature` (slice scaffold with ports, ViewModel, View, tests),
  `new-notification-type` (schema, category/channel, deep link, handler, test), `new-scene-asset` (pipeline + budgets), `device-perf-check`;
  specs list variants affected, permissions, tiers, budgets, push types, routes and test levels per AC.

---

## 20. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| One fat app containing all three products and all engines | Download size, memory and review surface on low-end devices |
| Assuming flagship GPUs/RAM; no Low-tier path | Crashes and unplayable frame rates for much of the market |
| Hard-coded device lists instead of capability detection + measurement | Stale, wrong, unmaintainable |
| 3D via WebView or per-frame JS-thread rendering for the Low tier | Order-of-magnitude slowdowns |
| Game entities/products as hundreds of positioned RN Views | JS/UI thread overload |
| Client-authoritative game state or prices | Cheating/fraud |
| Asking for all permissions at first launch | Denials, store rejection |
| Push payloads with sensitive data; push as the only sync path | Privacy leaks, missed state |
| Depending on a single avatar/voice vendor runtime | Shutdown risk (e.g., Ready Player Me) |
| Storing tokens in AsyncStorage/MMKV/logs | Credential theft |
| Embedded login WebViews, custom crypto | Phishing and weak security |
| Unbounded image/asset caches, uncompressed textures | OOM kills |
| OTA changes to native deps/permissions | Crashes and policy violations |
| Reanimated-everything on Low devices without measurement | Memory/CPU regressions |
| Account enumeration in login/forgot flows | Privacy and abuse |
| Ignoring OEM battery killers for push | Silent delivery failures |
| No device-farm/Low-pool performance testing | Unproven claims |
| Shipping an unpatched SDK with known memory regressions | Field crashes |

---

## 21. Definition of done and checklists

### 21.1 Feature DoD

- [ ] Spec lists variant(s), slice, routes, permissions, push types, device-tier behavior, budgets and test levels per AC.
- [ ] ViewModels tested with fake ports; all statuses covered; shared with other clients where applicable.
- [ ] Low-tier path implemented and verified on the Low pool; governor behavior defined.
- [ ] Uses the shared SDK; no hand-written paths/DTOs; error taxonomy handled; correlation IDs shown.
- [ ] Permissions requested in context with primer and degraded path; register updated.
- [ ] Accessibility verified (TalkBack/VoiceOver, large text, reduce motion); strings are keys; RTL checked.
- [ ] Offline/slow-network behavior defined; outbox/idempotency for mutations.
- [ ] Telemetry and redaction verified; no sensitive data in logs/analytics/push.

### 21.2 Release checklist

- [ ] Variant builds reproducible; size, permission and dependency diffs reviewed; SBOM and scans green.
- [ ] Unit, contract, e2e (emulator + real devices), accessibility, security suites green.
- [ ] Low/Mid pool performance within budgets (start, scroll, scene fps, memory, battery, crash-free).
- [ ] Push verified end to end (permission states, foreground/background/killed, deep links, token lifecycle).
- [ ] Auth flows verified: register, verify, login, MFA/step-up, forgot, reset, sign-out, deletion.
- [ ] Store checklist verified (Sign in with Apple, deletion, privacy forms, target API, 16 KB alignment, permissions text).
- [ ] Staged rollout, remote kill switch and rollback plan ready; runbooks current.

### 21.3 Architecture acceptance checklist

- [ ] Expo/RN current SDK, New Architecture, Hermes V1, patched for known regressions.
- [ ] Three variants from one monorepo with per-variant native deps, permissions, bundle IDs and EAS profiles.
- [ ] Layered architecture enforced (domain/application/viewmodels/adapters/views) with shared ViewModels and SDK.
- [ ] Route table mirrors the URL playbook; deep links validated; guards are UX only.
- [ ] Shared onboarding, login, registration, forgot/reset password, session security and account deletion in `mobile-core`.
- [ ] Push: expo-notifications + FCM v1/APNs, channels/categories, token lifecycle, deep-link contract, OEM guidance.
- [ ] `DeviceTier`, measured benchmark, `QualityGovernor`, remote overrides, performance modes.
- [ ] `SceneEnginePort` with 2D fallback, Filament, Godot, Unity adapters; lazy load/dispose; fallback chain.
- [ ] Asset pipeline with KTX2, LODs, budgets, signed manifests, on-demand packs.
- [ ] Game: server-authoritative, map via GL layers, location/battery modes, anti-cheat signals, AR optional.
- [ ] Market: server-authoritative pricing, idempotent checkout, Stripe PaymentSheet, FlashList, CDN-resized images.
- [ ] Avatar: open formats, provider port, voice pipeline with barge-in, performance-script contract, camera tracking optional.
- [ ] Observability, remote config, kill switch, diagnostics bundle.
- [ ] Device-lab testing with Low/Mid pool and CI performance gates.

---

## Appendix A — Variant-aware `app.config.ts` (sketch)

```ts
import type { ExpoConfig } from 'expo/config';

const VARIANTS = {
  game:   { name: 'Heimdall Go',     slug: 'heimdall-game',   id: 'com.example.heimdall.game',
            plugins: ['./plugins/with-maplibre', './plugins/with-godot', 'expo-location'],
            permissions: ['ACCESS_FINE_LOCATION', 'ACCESS_COARSE_LOCATION', 'POST_NOTIFICATIONS'] },
  market: { name: 'Heimdall Market', slug: 'heimdall-market', id: 'com.example.heimdall.market',
            plugins: ['@stripe/stripe-react-native', 'expo-camera'],
            permissions: ['CAMERA', 'POST_NOTIFICATIONS'] },
  avatar: { name: 'Heimdall Avatar', slug: 'heimdall-avatar', id: 'com.example.heimdall.avatar',
            plugins: ['react-native-filament', 'expo-audio', '@livekit/react-native-expo-plugin'],
            permissions: ['RECORD_AUDIO', 'CAMERA', 'POST_NOTIFICATIONS'] },
} as const;

const v = VARIANTS[(process.env.APP_VARIANT ?? 'market') as keyof typeof VARIANTS];

export default (): ExpoConfig => ({
  name: v.name, slug: v.slug, scheme: v.slug,
  ios:     { bundleIdentifier: v.id, associatedDomains: [`applinks:${process.env.LINK_DOMAIN}`] },
  android: { package: v.id, permissions: [...v.permissions], blockedPermissions: [] },
  plugins: ['expo-router', // only when the variant uses the file-based router (section 6)
            'expo-secure-store', 'expo-notifications', ...v.plugins],
  extra:   { variant: process.env.APP_VARIANT, apiBaseUrl: process.env.API_BASE_URL },
  runtimeVersion: { policy: 'appVersion' },
  updates: { url: process.env.EAS_UPDATE_URL },
});
```

## Appendix B — Device tier and quality governor (sketch)

```ts
type DeviceTier = 'low' | 'mid' | 'high';

async function resolveTier(cap: DeviceCapabilityPort, bench: BenchmarkPort, rc: RemoteConfig): Promise<DeviceTier> {
  const forced = rc.tierOverride(await cap.modelId());           // remote override wins
  if (forced) return forced;
  const hint = await cap.hintTier();                              // RAM, OS level, low-RAM flag, GPU probe
  const measured = await bench.lastOrRun();                       // p95 frame time of a reference scene
  return combine(hint, measured);                                 // measurement can only lower or confirm the hint
}

class QualityGovernor {
  constructor(private sensors: Sensors, private scene: SceneEnginePort, private profile: QualityProfile) {}
  tick(): void {
    const s = this.sensors.snapshot();                            // frame p95, thermal, memory pressure, battery saver
    if (s.frameP95 > this.profile.frameBudget || s.thermal >= 'serious' || s.memoryWarning) this.stepDown();
    else if (s.healthyForMs > 30_000) this.stepUp();              // hysteresis
    this.scene.setQuality(this.profile);
  }
}
```

## Appendix C — Notification routing (sketch)

```ts
function handleTap(n: IncomingNotification, nav: Navigator, session: SessionPort) {
  const link = parseDeepLink(n.data.deepLink);                    // allowlist of routes, validated params
  if (!link) return nav.go(routes.home());
  if (!session.isAuthenticated()) return queueAfterLogin(link);   // resume after bootstrap/login
  nav.go(link);                                                   // never perform state changes from the payload
}
```

## Appendix D — Auth flow summary

```text
Onboarding → Register (consent, age gate) → Verify (link/OTP) → Login → [MFA/step-up] → Home
Forgot: /forgot-password (generic response) → email link/code → /reset-password (token) → sessions revoked → Login
Sign-out: revoke server-side, wipe tokens, unregister device (push), clear caches per policy
Delete account: re-auth → confirm → grace period → server cascades → local wipe
```

## Appendix E — Asset budget table (starting points)

| Asset | Low | Mid | High |
|---|---|---|---|
| Character/creature triangles | ≤ 5–10k | ≤ 15–25k | ≤ 40–60k |
| Bones | ≤ 25 | ≤ 60 | ≤ 75+ |
| Materials per model | 1 | 1–2 | 2–4 |
| Texture size (KTX2, GPU-compressed) | 512 | 1024 | 2048 |
| Morph targets (face) | 10–15 | 15–20 | 20–52 |
| Scene draw calls | ≤ 50 | ≤ 100 | ≤ 200 |
| Concurrent animated models | 1–2 | 3–5 | 6–10 |
| Real-time shadows/post | off | simple | on |
| Peak scene memory | ≤ 300 MB total process | ≤ 450 MB | ≤ 700 MB |

## Appendix F — Support matrix template

```markdown
# Support matrix (per release)
| Platform | Min OS | ABIs | Notes |
| Android | <API> | arm64-v8a (+armeabi-v7a if needed) | 16 KB page-size aligned; target API <N> |
| iOS | <min> | arm64 | Xcode <N>, privacy manifest |
Device tiers: Low (≤3 GB RAM) · Mid (4–6 GB) · High (≥8 GB). Device pool: <list>
Known limitations per tier: <AR availability, background location, voice features>
```
