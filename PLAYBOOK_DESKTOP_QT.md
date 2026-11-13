# PLAYBOOK_DESKTOP_QT.md — Cross-platform Qt desktop application playbook

Architecture and engineering contract for building, shipping and operating a
**Qt desktop application for Windows, Linux and macOS** that:

* runs **fast on old hardware** (low-end CPUs, little RAM, slow disks, weak or
  broken GPUs) while running on **current operating systems**;
* connects to the platform backend through the shared **SDK** and optionally
  through **Server-Sent Events (SSE)** and/or **WebSocket** for live updates;
* is built with the same **hexagonal + vertical-slice** architecture, **MVVM**
  presentation and spec/test-driven workflow as the rest of the product family;
* is packaged, signed, updated, administered and audited at enterprise grade.

It builds on and must stay consistent with:

| Playbook | What this file takes from it |
|---|---|
| `PLAYBOOK_URLS.md` | Route/permission model; `{userType}` never authorizes; `module.resource.action` permissions; deep-link paths |
| `PLAYBOOK_BACKEND.md` | Slices, ports/adapters, tenant isolation, idempotency, outbox events, deployment modes (§3–§8) |
| `PLAYBOOK_FRONTEND.md` | MVVM + SDK, error taxonomy, `202` long operations, deep links (§3–§6) |
| `PLAYBOOK_SECURITY.md` | Authentication, authorization, crypto, supply chain, logging (§4–§14) |
| `PLAYBOOK_TESTS.md` | Test pyramid, authz matrix, tenant isolation, evidence (§1–§10) |
| `PLAYBOOK_AUDIT.md` | Control library, evidence, CRA/SOC 2/ISO mapping (§4–§14) |
| `PLAYBOOK_CHROME_EXTENSION.md` | Enterprise policy, kill switch, diagnostics bundle, release governance patterns reused here |
| `PLAYBOOK_CLAUDE_CODE.md` | SDD + TDD workflow, roles and kit for AI-assisted development |

The runnable, machine-checkable companion is
[beautifulqt](https://github.com/Facta-Dev/beautifulqt): three Agent Skills
(write, review, port), a drop-in clang-tidy/clang-format config and a compiled
example.

The rules that carry the whole file:

1. **Keep the GUI thread sacred.** Nothing that can block (network, disk, parsing,
   crypto, big loops) runs on it. Responsiveness on a 10-year-old dual-core is the
   acceptance test.
2. **Budget first, features second.** Every release is measured against a
   low-end reference machine; regressions fail CI.
3. **No GPU assumption.** The UI must work with software rendering. GPU paths are
   an optimization with a tested fallback.
4. **Domain and application know nothing about widgets, QML, sockets or the OS.**
   They talk through ports.
5. **The backend is the source of truth.** The desktop holds a cache (projection)
   and an outbox; it never invents authority.
6. **Realtime is optional and degradable.** The app is fully correct with
   polling or no streaming at all; SSE/WebSocket only make it fresher.
7. **Slices mirror the backend.** Same names, same permissions, same contracts.
8. **The local machine is untrusted.** Secrets live in the OS keystore, updates
   are signed, plugins and DLLs load only from trusted locations.
9. **Administrators can control it.** Managed policy, kill switch, update
   channels, diagnostics, audit trail.
10. **Everything ships signed, reproducible and with evidence.**

> Verification note: this playbook reflects the Qt and platform landscape as of
> November 2026. Qt 6.12 was released on 30 September 2026 as an LTS with five
> years of maintenance; 6.8 LTS remains supported until October 2029. Items
> marked **(verify)** depend on exact Qt/OS versions and must be re-checked in
> the Qt documentation and the platform vendors' documentation before relying
> on them. Treat numeric budgets as starting points to calibrate on your own
> reference hardware.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Qt version | **Qt 6.12 LTS** for new projects; Qt 6.8 LTS acceptable for conservative fleets | Five-year maintenance; align with the product's own support window. Qt 6.12 states it was prepared for EU CRA requirements for its part (apps still need their own compliance) |
| D2 | Language | **C++20** (C++17 minimum) for domain, application, adapters | Predictable performance and memory on old machines. PySide6/Python and QML-only JS logic increase startup, memory and CPU; not recommended under this performance target |
| D3 | UI technology | **Qt Widgets** as the default for data-dense/enterprise screens; **Qt Quick** only for isolated, motion-heavy surfaces, restricted to a software-safe subset (see 2.2) | Widgets render on the CPU (raster) with no GPU dependency and behave predictably on old drivers |
| D4 | Presentation pattern | **MVVM** with C++ ViewModels (`QObject` + `Q_PROPERTY`/bindable properties, `QAbstractItemModel` for lists) usable from Widgets **and** QML | Same ViewModel contract regardless of View technology |
| D5 | Build system | **CMake** (Presets, Ninja), dependency manager with locked baseline (**vcpkg** manifest or **Conan** lockfile) | Reproducible builds, SBOM source |
| D6 | Linking | **Dynamic** Qt (LGPL-compatible) unless a commercial license allows static linking | Licensing; smaller updates for shared runtimes; verify module licenses |
| D7 | Backend API | REST/JSON via the generated **C++ SDK** that mirrors `PLAYBOOK_URLS.md`; optional streaming over SSE/WebSocket | Same contract as web/mobile/extension; gRPC only if measurements justify it, behind the same SDK port |
| D8 | Local storage | **SQLite** (WAL) via a single storage adapter; optional **SQLCipher**/field encryption for Restricted data | Embedded, fast on old disks |
| D9 | Auth | OIDC Authorization Code + PKCE in the **system browser** with loopback or custom-scheme redirect; tokens in OS keystore | No embedded password UI; matches `PLAYBOOK_SECURITY.md` §4 |
| D10 | Updates | Signed updates with channels and staged rollout; admin can disable/defer | Section 14 |
| D11 | Crash reporting | Crashpad/Breakpad-class minidumps via a crash handler process, opt-in by policy | Section 13 |
| D12 | Embedded web content | **Avoid Qt WebEngine** in the main app; if unavoidable, isolate it in a separate optional component | WebEngine adds a Chromium-sized footprint and memory, which breaks low-end budgets |
| D13 | Architectures | x86-64 and ARM64 on Windows, macOS (universal) and Linux | Windows on Arm and Apple Silicon are mainstream; no 32-bit |
| D14 | CPU baseline | Compile for baseline x86-64 (no AVX/AVX2 requirement); use runtime dispatch for SIMD libraries | Old CPUs |
| D15 | TLS backend | Decide explicitly per OS (see 11.6) | Windows 10's native TLS stack lacks TLS 1.3 **(verify)** |

---

## 2. Platform and performance model

### 2.1 Support matrix policy

* **OS support = intersection** of (a) the platforms the chosen Qt version supports and
  (b) OS versions still receiving vendor security updates, plus any contractual
  extended-support carve-outs. Re-derive it at every Qt upgrade from the Qt "supported
  platforms" page **(verify)**; do not hard-code it in this playbook.
* "Old computers with new operating systems" is the target profile:
  modern OS builds on legacy hardware.
* Publish the matrix in `docs/support-matrix.md` with: OS and minimum build,
  architectures, display servers (X11/Wayland), minimum hardware, known
  limitations, end-of-support dates.

### 2.2 Reference hardware ("the floor")

Define and keep real or virtual machines for these profiles (adjust to your fleet data):

| Profile | CPU | RAM | Storage | Graphics | Display |
|---|---|---|---|---|---|
| **Floor** | 2 cores, ~2 GHz, no AVX assumption | 4 GB | 5400 rpm HDD (or throttled IO) | Old integrated GPU / **software rendering** | 1366×768, 100% DPI |
| **Typical** | 4 cores | 8 GB | SATA SSD | Integrated GPU | 1920×1080, 125% DPI |
| **High-DPI** | 4+ cores | 8+ GB | SSD | Integrated/discrete | 4K, 200% DPI, multi-monitor |

Every performance budget below is measured on the **Floor** profile. If the app is
smooth there, it is smooth everywhere.

### 2.3 UI technology choice: Widgets vs Qt Quick

| Criterion | Qt Widgets | Qt Quick (QML) |
|---|---|---|
| Rendering | CPU raster; no GPU required | Scene graph on RHI: Direct3D on Windows, Metal on macOS, OpenGL on Linux by default; a software backend exists with limitations |
| Old GPU/driver risk | None | Real (driver bugs, missing features); needs fallback logic (2.4) |
| Dense tables/trees/forms | Excellent, mature model/view | Possible, more tuning needed |
| Animation/fluid effects | Limited | Excellent on capable GPUs |
| Startup/memory | Low | Higher (QML engine, scene graph) |
| Native look | Native styles | Native-like styles, more theming work |
| Accessibility | Mature on all three OSes | Good, requires care |

**Policy:** Widgets-first. Use Qt Quick only where motion/custom drawing is core
value, keep its effects within the software-safe subset (no `ShaderEffect`, minimal
`layer.enabled`, no blur/shadow effects, simple clipping), and ship the Compatibility
Mode (2.4). A single window may mix both, but avoid per-frame GPU work on lists.

**Qt Canvas Painter** (production-ready in Qt 6.12) and **QRhiWidget** are GPU-accelerated
paths: use them only for optional visualizations and keep a CPU path (**verify** support
and behavior on your floor GPUs).

### 2.4 Rendering fallback (no-GPU guarantee)

Requirements:

1. A **Compatibility Mode** setting (user and managed policy) that forces CPU rendering
   and disables non-essential animation and transparency.
2. **Automatic safe mode**: on startup record `startup_in_progress=true`; clear it after
   the first window is shown and a frame is rendered. If the flag is still set at the next
   launch (crash/hang during init), start in Compatibility Mode and report it.
3. **Renderer selection** happens before the first window is created (Qt Quick: the
   graphics API must be chosen before the first `QQuickWindow`). Chain, per OS:
   Windows: D3D11 → OpenGL → software; macOS: Metal → software; Linux: OpenGL → software
   (Vulkan optional **(verify)**). Honor the override environment variables for support
   (`QSG_RHI_BACKEND`, `QT_QUICK_BACKEND=software`) and a CLI flag `--software-rendering`.
4. **Block-list** by GPU vendor/driver string, persisted from field telemetry/diagnostics.
5. Widgets-only windows need no switch, which is why Widgets is the default.
6. CI runs the full UI suite under software rendering (see 15).

### 2.5 Performance budgets (Floor profile; starting points)

| Metric | Budget |
|---|---|
| Cold start to first usable window (HDD) | ≤ 3 s; ≤ 1.5 s on SSD |
| Warm start | ≤ 1 s |
| Time to interactive with cached data (stale-while-revalidate) | ≤ 1 s after window |
| GUI thread block (any single task) | ≤ 16 ms typical, **≤ 50 ms hard limit**; watchdog logs violations |
| Scroll/resize of main list/table | Smooth (≥ 30 fps on Floor, 60 fps on Typical) with 100k rows (virtualized) |
| Input latency (click→visual response) | ≤ 100 ms |
| Idle CPU | ≈ 0% (no timers firing without need; < 0.5% sample average) |
| Idle RAM after warm-up | ≤ 150 MB (Widgets) / ≤ 250 MB (with Quick) |
| Memory growth over 8 h soak | ≈ 0 (no unbounded caches/leaks) |
| Realtime burst (1,000 events/s) | UI updates coalesced to ≤ 30 Hz; CPU stays bounded; no UI stall |
| Installer size / install footprint | Tracked per release; prune unused Qt modules and plugins |
| Battery impact (laptops) | No background polling when hidden beyond policy; timers coalesced |

Budgets are enforced by an automated benchmark job (see 15.4) with a regression
threshold (for example +10% fails).

### 2.6 Engineering rules for performance

**Threading and the event loop**

* The GUI thread runs Views, ViewModels and trivial orchestration only.
* I/O and CPU-bound work runs through `QtConcurrent`/`QThreadPool`/`QFuture` or
  dedicated worker threads; results return to the GUI thread via queued signals
  or `QFuture::then(context, …)`.
* One `QNetworkAccessManager` per thread that uses it (it is reentrant but tied to
  its thread's event loop); one SQLite connection per thread; long-lived realtime
  connections get their **own** manager (see 8.3).
* Never call `processEvents()` to "keep the UI alive"; never use blocking waits
  (`waitForReadyRead`, `QThread::sleep`, `QFuture::result()` on the GUI thread).
* Add a **GUI-thread watchdog**: a heartbeat timer on the GUI thread and a monitor
  thread; stalls over the budget are logged with a stack/metadata (and counted in
  telemetry).

**Models and views**

* Large data uses custom `QAbstractItemModel`s with **lazy fetching**
  (`canFetchMore/fetchMore`), stable row identity, batched change signals
  (`beginInsertRows` ranges, one `dataChanged` per range), and uniform row heights
  (`setUniformItemSizes`) where possible.
* Filtering/sorting of large sets happens in the application/data layer (SQL or
  indexed in-memory) and returns pages; avoid `QSortFilterProxyModel` on very large
  datasets, or run it incrementally/asynchronously.
* Custom delegates paint with cached `QPixmap`/pre-laid-out text; no layout
  computation in `paint()`; no per-paint allocations.
* Prefer `QStringView`/`QByteArrayView`, `reserve()`, move semantics and implicit
  sharing; avoid copying big containers; use `std::span`.

**Rendering and assets**

* Vector icons (SVG) cached to pixmaps per scale; no runtime-scaled huge images;
  decode images on worker threads at display size.
* Avoid `QGraphicsEffect`, translucent top-level windows and per-pixel alpha, large
  stylesheets (QSS) on hot widgets, and frequent palette/style changes.
* Minimize repaint regions (`update(rect)`), disable updates during bulk changes,
  throttle progress/animation to what is visible.
* Limit fonts and font fallback chains; avoid text elision recomputation per frame.

**Startup**

* Do the minimum before the first window: create the composition root, show UI from
  the **local cache**, then connect to the network and refresh in the background.
* Lazy-load features/plugins; initialize slices on first navigation; defer crash-handler
  upload, update checks, telemetry and realtime connect until after first paint.
* Keep the binary lean: only needed Qt modules and platform/image-format/style plugins;
  strip symbols (keep separate debug files), enable LTO; avoid static initializers with
  I/O; compile QML ahead of time (`qt_add_qml_module`, QML caching/compiler) if QML is
  used.

**Data and IO**

* SQLite: WAL mode, `synchronous=NORMAL`, prepared statements, batch writes in
  transactions, appropriate indexes, bounded page cache, writes on one writer thread;
  avoid random IO patterns on HDDs.
* Streaming JSON for big payloads or a fast parser with runtime SIMD dispatch; never
  parse megabytes of JSON on the GUI thread.
* Bounded caches (`QCache`/LRU) with size accounting; disk cache with quota and
  eviction; compress cold data if CPU permits.

**Power and background behavior**

* Pause or slow non-essential work when the window is hidden/minimized or the OS
  signals low power; resume and reconcile on show.
* Use coarse timers (`Qt::CoarseTimer`) where precision isn't needed; coalesce wakeups.

### 2.7 Compatibility hygiene for old machines

* Test on the Floor profile every release, including **first-run on a cold disk cache**.
* Do not require instruction sets beyond the baseline in your own code; verify third-party
  libraries dispatch at runtime.
* Ship needed runtimes (MSVC redistributable, etc.) in the installer; support older
  graphics drivers by never relying on features without a fallback.
* Linux: build on the **oldest supported distribution** (glibc baseline) to run on newer
  ones; test X11 and Wayland, and a software-rendering Mesa configuration.
* Respect OS accessibility, reduced-motion and high-contrast settings.

---

## 3. Architecture: hexagonal + vertical slices + MVVM

### 3.1 Layers

```text
┌──────────────────────────────── desktop app (one process) ─────────────────────────────────┐
│ INBOUND ADAPTERS (drive the app)                   OUTBOUND ADAPTERS (driven by the app)   │
│ Views (Widgets/QML) + ViewModels                   SDK/HTTP · Realtime (SSE/WS/poll)       │
│ Realtime events · OS events (tray, deep links,     SQLite storage · OS keystore            │
│ single-instance IPC, power/network) · timers       Notifications · Updater · Telemetry     │
│ CLI flags                                           Policy reader · File system            │
│                 │                                              ▲                           │
│                 ▼                                              │                           │
│           APPLICATION (use cases, ports, policy, sync, outbox) ─┘                           │
│                 │                                                                          │
│                 ▼                                                                          │
│           DOMAIN (entities, value objects, rules, domain events)                           │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Contains | May depend on | Must not depend on |
|---|---|---|---|
| **domain** | Entities, value objects, invariants, domain events, pure services | `shared-kernel`, C++ std; QtCore **value types** only (`QString`, `QDateTime`, `QUuid`, `QList`, …) if the ADR allows | `QObject`, signals/slots, QtGui/Widgets/Quick/Network/Sql, OS APIs |
| **application** | Use cases, **ports**, policy resolver, sync engine, outbox, permission checks | domain, `shared-kernel`, `contracts`, **QtCore** (`QObject`, `QFuture`, `QTimer`) | QtGui/Widgets/Quick/Network/Sql/WebSockets, platform adapters |
| **adapters/out** | SDK client, realtime transports, SQLite, keystore, notifications, updater, telemetry, policy sources | application ports, `platform`, Qt Network/Sql/WebSockets/etc. | views, viewmodels |
| **adapters/in** | ViewModels (as presentation adapters), realtime event handlers, OS event handlers | application, contracts | other slices' internals, SQL/HTTP |
| **presentation/views** | Widgets or QML | ViewModel types, `ui-kit` | SDK, storage, network, application directly |
| **platform** | Per-OS code behind interfaces (`#ifdef` lives only here) | OS APIs, Qt Core/GUI | domain/application |
| **contracts** | DTOs, error codes, event envelope, policy schema, route table | `shared-kernel` | everything else |

Rules:

* **`#ifdef Q_OS_*` appears only under `platform/`** (and build files).
* ViewModels are the only presentation code that talks to use cases.
* Enforce with a dependency checker (include-graph rules via CMake target link
  rules + `include-what-you-use`/custom script or clang-tidy checks): each layer is a
  separate CMake target and **links only to allowed targets**, so violations fail at link
  time, not just in review.

### 3.2 Ports (examples)

```cpp
// application/ports/lead_repository.h  — owned by the slice, domain language
template <class T> using Result = tl::expected<T, AppError>;   // or std::expected (C++23) per ADR

class LeadRepositoryPort {
public:
  virtual ~LeadRepositoryPort() = default;
  virtual QFuture<Result<Lead>>     byId(LeadId id) = 0;
  virtual QFuture<Result<LeadPage>> list(LeadQuery q) = 0;
  virtual QFuture<Result<Lead>>     updateStage(LeadId id, Stage s, Version expected, IdempotencyKey key) = 0;
};

class SessionPort    { /* current(), signIn(), signOut(), permissions() */ };
class PolicyPort     { /* effective(), onChanged(cb) */ };
class RealtimePort   { /* start(config), stop(), setTopics(set), signals: stateChanged, eventReceived, resyncRequired */ };
class NotifierPort   { /* notify(Notice) */ };
class ClockPort      { /* now() */ };
class SecretStorePort{ /* get(key), set(key,value), remove(key) */ };
```

Ports return `QFuture<Result<T>>` (errors as values, never exceptions across layers).
Exceptions are not used for control flow; build with exceptions enabled only if Qt/deps
require it, and never let them cross a `QObject` slot boundary.

### 3.3 MVVM in Qt

* **ViewModel** = `QObject` with `Q_PROPERTY` (preferably bindable properties via
  `Q_OBJECT_BINDABLE_PROPERTY`), invokable commands (`Q_INVOKABLE`/slots) and a
  `status` enum: `Loading | Ready | Empty | Forbidden | NotFound | Offline | Error |
  PolicyBlocked`. Lists expose `QAbstractItemModel`.
* **View** = Widgets or QML binding to the ViewModel. Widgets Views connect signals
  explicitly (or via small binding helpers); QML Views bind declaratively. Views contain
  no business conditions beyond "show this status".
* ViewModels depend on use cases/ports, a `Navigator` port and a `Notifier` port, never on
  widgets or windows. Navigation is by **route** (section 6.2), not by window pointers.
* One ViewModel per screen or significant component; no global mutable singletons;
  construction through the composition root.
* ViewModels are tested without any GUI (`QSignalSpy`, fake ports).

### 3.4 Vertical slices and the backend mapping

Desktop slices mirror backend slices one-to-one:

| Desktop slice | Backend slice | URL module | Permission prefix | Realtime topic prefix | Test folder |
|---|---|---|---|---|---|
| `funnels` | `funnels` | `funnels` | `funnels.*` | `funnels.` | `tests/funnels/desktop/` |
| `projects` | `projects` | `projects` | `projects.*` / `campaigns.*` | `projects.` | `tests/projects/desktop/` |
| `tenants` (admin) | `tenants` | `tenants` | `tenant.*` | `tenants.` | `tests/tenants/desktop/` |
| `usage`, `billing` (admin) | same | same | `billing.*` | `usage.` | … |
| `identity` | `identity` | (session) | (n/a) | `session.` | … |

Each slice contains `domain/ application/ adapters/{in,out}/ presentation/{viewmodels,views}/ contracts/`.
Cross-slice access goes only through the other slice's `contracts/` (same rule as the backend).

### 3.5 Shared artifacts vs shared code

The web/mobile/extension clients share TypeScript ViewModels and SDK; a C++ client cannot.
What is shared across all clients is the **contract**, and parity is enforced by tests:

| Shared artifact | Source of truth |
|---|---|
| OpenAPI / JSON Schemas (DTOs, errors, realtime envelope) | Backend repo; **generate** the C++ DTOs and SDK stubs from them |
| Permission catalog (`module.resource.action`) | Backend; exported as JSON; compile-time/enum generated |
| Route table (`/{userType}/{module}/…`, deep links) | Shared JSON route table → generated route builders in each client |
| Design tokens | `packages/tokens` → generated QSS/palette/QML constants |
| i18n message keys | Shared key catalog; per-client translations |
| Acceptance criteria | Specs (SDD) → each client's tests cite the same AC IDs |

A **parity suite** runs the same scenario fixtures against each client's SDK/ViewModel
layer to catch divergence in status handling, error mapping and permission gating.

### 3.5.1 Embedded browser temptation

Do not reuse the web app via an embedded browser to "save work" in this client: it breaks the
Floor-profile budgets and the security model. If a screen must be web-based, open it in the
system browser.

---

## 4. Repository layout

```text
apps/desktop/
├── CMakeLists.txt  CMakePresets.json  vcpkg.json (or conanfile.py + lockfile)
├── cmake/                        # hardening.cmake, qt.cmake, versioning.cmake, deploy.cmake, sanitizers.cmake
├── src/
│   ├── app/                      # composition root: main.cpp, bootstrap, DI container, lifecycle, single-instance
│   ├── shared-kernel/            # Result, ids, errors, strong types, time, span helpers
│   ├── contracts/                # generated DTOs, error codes, realtime envelope, policy schema, route table
│   ├── platform/                 # ONLY place for OS-specific code
│   │   ├── common/  windows/  macos/  linux/
│   │   │   keystore  notifications  autostart  tray  paths  power  network_info  protocol_handler  file_assoc
│   ├── infrastructure/
│   │   ├── http/                 # QNAM transport, retry, proxy, TLS config
│   │   ├── realtime/             # sse_client, websocket_client, polling_client, selector, state machine
│   │   ├── storage/              # sqlite connection pool, migrations, outbox, cache
│   │   ├── crypto/               # AES-GCM helpers, key mgmt (via OS keystore)
│   │   ├── telemetry/            # logging, redaction, ring buffer, crash, metrics, watchdog
│   │   ├── updater/              # manifest verify, download, apply, rollback
│   │   └── policy/               # registry/plist/etc/ policy sources + resolver
│   ├── features/<slice>/
│   │   ├── domain/ application/ adapters/{in,out}/ presentation/{viewmodels,views}/ contracts/
│   └── ui-kit/                   # shared widgets, styles/themes, icons, delegates, dialogs
├── resources/                    # icons (SVG), themes, translations (.ts), fonts (subset)
├── packaging/
│   ├── windows/ (WiX/MSI, MSIX, NSIS/IFW)  macos/ (app bundle, dmg, pkg, entitlements)
│   └── linux/ (AppImage, deb, rpm, flatpak, desktop file, appstream metainfo)
├── tools/                        # codegen (OpenAPI→C++), license report, symbol upload, perf harness
packages/
├── sdk-cpp/                      # generated + thin hand-written C++ SDK (no GUI deps)
├── tokens/  contracts/           # shared with other clients
tests/<slice>/desktop/{unit,integration,contract,e2e,perf,security,a11y,visual}/
tests/_platform/{architecture,desktop-perf,installer,updater,policy,realtime,security}/
docs/{adr,support-matrix.md,admin-guide.md,runbooks/,permissions-register.md,data-flows.md}
```

Each layer is its own CMake target (`desktop_domain`, `desktop_application`, …) with explicit
`target_link_libraries` that encode the dependency rules.

---

## 5. Backend integration: SDK and contracts

### 5.1 C++ SDK (`packages/sdk-cpp`)

* **Mirrors the URL playbook:** `sdk.app().funnels().leads().get(id)` ↔
  `GET /api/v1/app/funnels/leads/{leadId}`; `sdk.admin().tenants().users().list(tenantId)` ↔
  `GET /api/v1/admin/tenants/{tenantId}/users`. Methods: `list/get/create/update/remove` plus
  verb methods (`suspend`, `launch`). Paths are built in exactly one place.
* Generated from the backend **OpenAPI** (use an OpenAPI generator targeting C++/Qt or a custom
  generator) so DTOs and paths cannot drift; hand-written code is limited to transport,
  interceptors and ergonomics.
* **No GUI dependencies**; depends on QtCore/QtNetwork only; returns `QFuture<Result<T, SdkError>>`.
* **Transport interceptors** (same order as `PLAYBOOK_FRONTEND.md` §3.2): auth (Bearer, single-flight
  refresh, replay once), tenant (`X-Tenant-Id` from the session), correlation
  (`X-Correlation-Id`), client info (`X-Client: desktop/<version> (<os>; <arch>)`), idempotency
  (`Idempotency-Key` for mutations), retry (backoff + jitter for network/429/5xx on idempotent
  calls), error mapping (envelope → `SdkError`).
* **Errors** reuse the shared taxonomy: `unauthenticated | forbidden(requiredPermission) | not-found |
  conflict | validation(fields) | rate-limited | network | server(correlationId) | upgrade-required`.
* **Versioning:** the SDK targets one API version; the desktop declares a minimum supported API and
  the backend can answer `upgrade-required` with a typed error that the app surfaces and the updater
  handles.
* **Network settings:** one `QNetworkAccessManager` for request/response traffic (HTTP/2 enabled
  where the server supports it), system proxy by default, bounded redirect policy, explicit timeouts
  (`QNetworkRequest::setTransferTimeout`), response size limits, compression accepted, conditional
  requests (`ETag`/`If-None-Match`) for cacheable lists.
* **HTTP/1.1 connection limit:** a `QNetworkAccessManager` runs up to six parallel requests per
  host/port on desktop, so any long-lived stream must not share the request manager (see 8.3).

### 5.2 Authorization on the client

* The session carries `permissions[]` (same strings as the backend). `can("funnels.leads.update")`
  only shapes the UI. **The backend decides**; every ViewModel handles `forbidden` and shows
  `requiredPermission`.
* `{userType}` (`/app`, `/admin`, `/partner`) selects the **experience** (navigation shell, windows,
  menus), never access. A user with the admin experience still has each call authorized by token, tenant,
  RBAC and ownership server-side.
* Multi-role users have an **active role**; switching resets caches, realtime subscriptions and
  permission projections.

### 5.3 Long operations

Operations answered with `202 Accepted` (for example `POST /api/v1/app/projects/{projectId}/launch`)
are tracked as **operation resources** (`.../runs/{runId}`). The app persists `runId`, polls with
backoff or receives progress events through realtime, survives restarts, and never blocks a thread
waiting.

### 5.4 Offline and outbox

* The local DB is a **projection cache**: server data with `updatedAt/version` and a "stale" marker; it is
  never an authority.
* Mutations the user performs while offline go to an **outbox** (SQLite): `{id, type, payload,
  idempotencyKey, attempts, nextRunAt, status}`; the sync worker sends them in order per entity with
  backoff; conflicts (`409`/version mismatch) surface as typed results that the ViewModel presents for
  resolution.
* Outbox handlers are idempotent by key, matching the backend's idempotency contract
  (`PLAYBOOK_BACKEND.md` §5/§6).
* A schema/version gate prevents old clients from writing new-format data; outbox items carry the
  client version.

### 5.5 Backend additions required for desktop and realtime

These must be added to the backend playbook/specs (new spec per slice) before client work begins:

| Addition | Contract |
|---|---|
| Realtime capabilities | `GET /api/v1/app/realtime/capabilities` → supported transports, heartbeat interval, max message size, resume window, topic catalog version |
| SSE stream | `GET /api/v1/app/realtime/stream?topics=…` (`text/event-stream`), supports `Last-Event-ID` |
| WebSocket | `GET /api/v1/app/realtime/socket` (upgrade), subprotocol `heimdall.v1` |
| Admin variants | `/api/v1/admin/tenants/{tenantId}/realtime/…` for cross-tenant monitoring (admin permission checked server-side) |
| Device/session registration (optional) | `POST /api/v1/app/devices` (client version, OS, arch, channel) for audit and targeted notices |
| Update manifest | Static signed manifest on a CDN (preferred) or `GET /api/v1/public/desktop/updates/{channel}/{os}/{arch}` (`/public` = session-less) |
| Min-version gate | `upgrade-required` error with `minVersion`, `downloadUrl` hints |
| Topics | `{module}.{resource}[.{id}]`, authorized per topic with the same `module.resource.read` permissions |

Backend placement follows the hexagon: realtime is an **inbound adapter** per slice
(`adapters/in/realtime`) fed by the **outbox/event bus** (Redis Streams) through a `realtime`
deployment mode that fans events out per tenant, with backpressure and per-connection limits
(`PLAYBOOK_BACKEND.md` §5, §7.2, §8).

---

## 6. Navigation, routes and deep links

### 6.1 Route model

The desktop uses the **same path grammar** as the web and mobile clients:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]

/app/funnels/leads/{leadId}
/app/projects/{projectId}/funnels/{funnelId}
/admin/tenants/{tenantId}/users
/admin/tenants/{tenantId}/billing
```

* A `Navigator` port accepts a route (from the generated route table), resolves the screen
  (ViewModel + View), and manages history/back/forward and multi-window behavior.
* Route builders are **generated** from the shared route table; no hand-written path strings in
  Views.
* Guards (`RequireUserType`, `RequirePermission`) redirect to sign-in, a "not permitted" view or the
  allowed home; they only shape UX.

### 6.2 Shell per `{userType}`

* `/app`: employee shell (funnels, leads, projects). `/admin`: admin shell (separate navigation,
  admin guard). `/partner`: partner shell. Shells are separate modules loaded on demand to keep
  startup small for users who never see admin screens (performance, not security).

### 6.3 Deep links and OS integration

* Custom protocol (`heimdall://app/funnels/leads/{leadId}`) and, where useful, associated web URLs,
  registered per OS (registry/URL scheme, `Info.plist`, `.desktop` MIME handler).
* Incoming deep links, file-open events and second-launch arguments arrive through the
  **single-instance IPC** (`QLocalServer`/`QLockFile`), are parsed by a strict parser (allowlist of
  routes, size limits, no commands), mapped to a route and handled by the `Navigator`.
* Deep-link parameters are **untrusted input**: validate IDs, never execute actions without UI
  confirmation, and require the authenticated session.

---

## 7. Authentication and session

* **OIDC Authorization Code + PKCE** in the **system browser**; redirect via a loopback listener
  (`127.0.0.1`, random port, single-use state) or a registered custom scheme. Qt Network Auth can help;
  confirm PKCE support in your Qt version **(verify)**, otherwise implement it in the adapter.
  No embedded login webview.
* **Token storage**: access token in memory (refresh on demand); **refresh token in the OS keystore**
  through a `SecretStorePort` adapter (Windows Credential Manager/DPAPI, macOS Keychain, Linux Secret
  Service/libsecret or KWallet; a vetted wrapper library such as QtKeychain **(verify license and
  maintenance)**). If no secure store exists (headless Linux), degrade to an explicit "sign in each
  session" mode rather than plaintext files.
* Refresh rotation with reuse detection (server side), single-flight refresh in the SDK, immediate
  revoke on sign-out/suspension, token cleared from memory on lock/sleep if policy requires.
* Sign-out wipes tokens, caches (per policy), subscriptions and outbox decisions (the outbox is kept
  only if the user confirms, since it contains unsent user data).
* Enterprise SSO: IdP issuer/client ID from managed policy; support device-bound or conditional-access
  signals through the IdP, not custom device fingerprinting.
* Session expiry and policy changes surface as ViewModel statuses (`signed-out`, `policy-blocked`).

---

## 8. Realtime: optional SSE and WebSocket

### 8.1 Principles

1. **Optional and degradable**: `RealtimeMode = auto | websocket | sse | polling | off` (managed policy
   > user setting > default `auto`). With `off` or on failure the app works via refresh/polling.
2. **Snapshot + stream**: state is obtained through normal API calls; the stream carries **change
   notifications** (and small payloads). After any gap, the app **resyncs** affected resources.
3. **At-least-once delivery**: events have monotonic IDs and are **deduplicated**; handlers are idempotent;
   ordering is guaranteed only per entity/topic.
4. **Tenant and permission scoped**: the server authorizes every subscription; the client never assumes
   a topic is allowed.
5. **Backpressure**: the client coalesces bursts and renders at a bounded rate.

### 8.2 Components

```text
RealtimePort (application)  ◄── RealtimeCoordinator (adapter)
                                   ├── TransportSelector (capabilities + policy + health)
                                   ├── SseTransport        (QNetworkAccessManager streaming)
                                   ├── WebSocketTransport  (QWebSocket)
                                   ├── PollingTransport    (ETag / long poll / since-cursor)
                                   └── NullTransport       (off / offline)
Event pipeline: transport → envelope parser (schema, size cap) → dedupe (LRU of ids)
                → topic router → slice handlers (adapters/in/realtime) → use cases → ViewModels
```

### 8.3 Connection state machine

```text
Disabled ─start→ Connecting ─ok→ Live ─error/timeout→ Reconnecting ─backoff→ Connecting
                         └─fail (policy/transport)→ Degraded(polling)
Live ─stop→ Disconnected;  any ─policy off/kill switch→ Disabled
```

* **Backoff** exponential with full jitter (for example 1 s → 60 s cap), reset after a healthy period;
  honor server `retry:` hints (SSE) and `Retry-After`.
* **Heartbeat**: server sends comments/pings every 15–25 s; client declares the connection dead after
  ~2 missed intervals and reconnects. Set explicit transfer timeouts longer than the heartbeat; Qt
  applies a default transfer timeout to requests **(verify)**, so streaming requests must override it,
  and newer Qt versions changed TCP keepalive defaults **(verify)** (a connection can be terminated
  after about two minutes of inactivity).
* **Resume**: SSE `Last-Event-ID` header; WebSocket `resumeToken` in the subscribe message. If the
  server answers `resync-required` (window exceeded), emit `resyncRequired(topics)` and refetch.
* **Network changes and sleep/wake**: observe reachability (`QNetworkInformation` **(verify)**), OS
  sleep/resume and network switches; drop and re-establish rather than trusting half-open sockets.
* **Visibility**: when the app is hidden/minimized (policy-configurable), pause the stream or lower
  the rate and reconcile on show.
* **Dedicated connection**: the stream uses its **own** `QNetworkAccessManager`/`QWebSocket` so it
  never consumes one of the six HTTP/1.1 slots used by API calls; prefer HTTP/2 where available.

### 8.4 SSE transport (client side)

Qt has no first-party SSE client class that this playbook relies on **(verify for your Qt version)**;
implement an adapter with a **spec-compliant parser**:

* Request: `GET` with `Accept: text/event-stream`, `Cache-Control: no-cache`, `Authorization: Bearer …`
  (desktop can set headers, unlike browser `EventSource`), `Last-Event-ID` when resuming, HTTP/2
  allowed, cache bypass, timeout overridden, no response buffering limit issues (consume `readyRead`
  incrementally).
* Parser: UTF-8, optional BOM, line terminators `\n`, `\r`, `\r\n`; fields `event`, `data` (multiple
  lines joined with `\n`), `id`, `retry`; comment lines (`:`) as heartbeat; dispatch on blank line;
  `id` persisted as last-event-id; unknown fields ignored; enforce **max event size** and **max line
  length**; reject invalid UTF-8.
* Resilience: reconnect on EOF/error with the state machine above; detect proxies that buffer streams
  (no data despite heartbeats → fall back to WebSocket/polling and report in diagnostics).
* Security: TLS only; token in header, never in the URL; redact in logs.

### 8.5 WebSocket transport

* Qt WebSockets (`QWebSocket`): connect with a request carrying `Authorization`, subprotocol
  `heimdall.v1`, origin/client headers; set maximum incoming frame/message sizes; use app-level
  heartbeat messages in addition to protocol pings.
* Messages: JSON envelope `{v, type, id, ts, topic, payload}` with a command set
  (`subscribe`, `unsubscribe`, `ack`, `ping`, `auth.refresh`) validated by schema; unknown types ignored
  and counted.
* Token refresh while connected: send `auth.refresh` or reconnect with the new token before expiry.
* Compression, fragmentation and per-message limits depend on the Qt version **(verify)**; keep payloads
  small (notifications + IDs), fetch details via REST.

### 8.6 Polling fallback

Cursor-based `GET …?since=<cursor>` or ETag conditional requests at an adaptive interval (faster while
the window is active and recent activity exists, slower when idle/hidden), with jitter and server-driven
`Retry-After`. It is the **correctness baseline**; streams are an optimization.

### 8.7 Applying events

* Events are **notifications first**: a handler invalidates/refreshes the affected cache entries and
  updates the model with minimal diffs; large state changes trigger a refetch.
* UI updates are **batched** on the GUI thread (timer at ≤ 30–60 Hz), coalescing by entity; row-level
  `dataChanged` over ranges; no per-event repaint storms.
* Ordering/gaps: keep a per-topic sequence if the server provides one; on a gap, resync.
* Permission changes arrive as events (`session.permissions.changed`) and trigger a permission refresh
  and a ViewModel re-evaluation.

### 8.8 Server contract summary (for the backend spec)

Envelope: `{ id, type, tenantId, topic, ts, v, correlationId, payload }` (payload minimal; PII avoided).
Delivery: at-least-once, resume window ≥ N minutes, per-tenant partitioning, per-connection rate and
size caps, authz per topic at subscribe **and** on each push for revocable permissions, immediate
termination on suspension/kill switch, connection limits per user/tenant, metrics on lag.

---

## 9. Local storage and data

* **SQLite** with migrations (`PRAGMA user_version`), forward-only, tested against fixtures of every
  released schema; backup before migrating; integrity check on startup after crash; **reset-and-resync**
  path when corruption is detected (the server holds truth).
* **Per-tenant/per-user databases** (or tenant-scoped tables with `tenant_id` first in indexes) so
  tenant switch and sign-out can purge cleanly.
* **Encryption**: OS disk encryption is baseline; for Restricted data use SQLCipher or application-level
  AES-GCM with keys in the OS keystore; never store secrets or tokens in `QSettings`/plain files.
* **Retention**: `DataRetentionDays` (policy) enforced by a background job; purge on sign-out and on
  tenant removal; temp files in the app-private directory with restrictive permissions.
* **Paths** via `QStandardPaths` (config, cache, data, logs); never write next to the executable;
  portable-mode is an explicit, documented build option.
* **Settings**: `QSettings` for non-sensitive preferences; policy-locked keys are read-only in the UI.
* **Concurrency**: one writer, multiple readers; WAL; no DB access from the GUI thread.

---

## 10. Platform integration (per OS, behind `platform/`)

| Capability | Windows | macOS | Linux |
|---|---|---|---|
| Secret store | Credential Manager / DPAPI | Keychain | Secret Service (libsecret) / KWallet |
| Notifications | Native toast (or tray message fallback) | `UNUserNotificationCenter` (requires proper bundle/signing) | freedesktop D-Bus notifications |
| Tray/menu bar | Notification area | Menu bar extra (+ global menu bar for the app menu) | StatusNotifier/AppIndicator (not guaranteed on all desktops; provide a fallback) |
| Autostart | Registry Run key / Task (per policy) | Login item / LaunchAgent | XDG autostart `.desktop` |
| Deep links/file assoc | Registry | `Info.plist` | `.desktop` + MIME |
| Single instance | Mutex/`QLockFile` + `QLocalServer` | same | same |
| High DPI | Per-monitor v2 | Retina | X11/Wayland scaling quirks, test fractional scaling |
| Dark mode/contrast | Follow OS color scheme and high contrast | Follow appearance | Follow desktop settings; test GNOME and KDE |
| Display server | n/a | n/a | **X11 and Wayland** (global shortcuts, window positioning and screen capture differ) |
| File dialogs | Native | Native | Desktop portal/native where available |
| Power/network events | WM messages/`QNetworkInformation` | NSWorkspace/`QNetworkInformation` | logind/NetworkManager via D-Bus (best-effort) |
| Accessibility | UI Automation | NSAccessibility | AT-SPI |

Rules:

* Platform adapters implement ports (`NotifierPort`, `SecretStorePort`, `AutostartPort`…) and are
  selected at the composition root.
* Every platform feature has a **graceful fallback** and is covered by a capability probe (for example
  tray unavailable on a Wayland desktop → in-app notification center).
* Use the native style for each OS by default; custom theming goes through tokens and must keep
  contrast/accessibility; test dark/light/high-contrast.
* Window state restoration (geometry, screen) must handle monitors that disappeared.
* Keyboard shortcuts use `QKeySequence::StandardKey` and platform conventions (`Ctrl` vs `Cmd`).
* Internationalization with Qt Linguist (`tr`), ICU-backed formatting, RTL layouts, pseudo-locale
  tests; no string concatenation for sentences.
* **Accessibility is mandatory:** roles, names, focus order, keyboard-only operation, screen-reader
  checks on all three OSes, high-contrast, respect for reduced motion.

---

## 11. Security

Aligned with `PLAYBOOK_SECURITY.md`; desktop-specific additions below.

### 11.1 Threat model (desktop)

| Threat | Controls |
|---|---|
| Local malware reading files/memory | OS keystore for secrets, no secrets on disk, minimal in-memory lifetime, no tokens in logs/crash dumps |
| Binary/library tampering, DLL/plugin hijacking | Load libraries/plugins only from the app directory (`QCoreApplication::setLibraryPaths` restricted; Windows safe DLL search order and `SetDefaultDllDirectories`), signed binaries, hardened runtime on macOS |
| Malicious or tampered update | Signed manifests and binaries, HTTPS, rollback protection (monotonic versions), staged rollout, key custody |
| Malicious deep link / IPC | Strict parser, route allowlist, UI confirmation, authenticated session, local IPC with OS-level permissions |
| Credential theft / phishing | System browser + PKCE, no embedded login, MFA via IdP |
| MITM / TLS interception | TLS validation on, no `ignoreSslErrors`, enterprise CA via OS store or policy, optional pinning for the update channel only |
| Hostile server content | Schema validation, size limits, escape on render (rich text in `QLabel`/`QTextBrowser` can render HTML; use plain text or sanitized subsets), no remote code or plugins |
| Insecure realtime | Header-based auth, short tokens, per-topic authz server-side, schema-validated messages, max sizes |
| Supply chain (Qt, vcpkg/Conan deps) | Pinned baseline, SBOM, cooldown for new versions, review, signed artifacts, provenance |
| Information leak via diagnostics | Redaction, user-initiated export, `TelemetryLevel` |
| Memory-safety bugs in C++ | Section 11.3 profile |

### 11.2 Controls

* TLS 1.2+ (prefer 1.3), certificate validation always on; never connect to unexpected hosts: the SDK
  restricts hosts to the policy/tenant allowlist.
* Secrets never in the binary, resources, QSettings, logs or crash dumps; tenant provider keys never
  reach the desktop (same rule as the extension).
* Restrict file permissions on app data directories; wipe temp files; clear clipboard-held secrets
  where the app copies them (with timeout).
* Disable debugging aids in release builds (no `QT_QPA` debug env honoring beyond a documented support
  flag; no QML debugger port; no remote inspector).
* **No Qt WebEngine in the main app** (attack surface and size), or isolate it.
* Local IPC (`QLocalServer`) is restricted to the current user; messages are schema-validated and
  authenticated by a per-session secret when sensitive.
* Command-line arguments and environment variables are untrusted when launched via protocol handlers.
* Auto-update trust root is separate from the code-signing identity where possible.

### 11.3 C++ secure coding profile (complements `PLAYBOOK_SECURITY.md` §16)

* **Language discipline:** C++20, RAII, `std::unique_ptr`/`QObject` parent ownership (no naked `new`
  without an owner), `std::span`/views over raw pointers, no manual `malloc/free`, no `strcpy`-family,
  bounds-checked access in hot paths via `at()`/asserts in debug, `[[nodiscard]]` on results, no
  C-style casts.
* **Qt hygiene:** define `QT_NO_CAST_FROM_ASCII`, `QT_NO_CAST_TO_ASCII`, `QT_NO_URL_CAST_FROM_STRING`,
  `QT_NO_FOREACH`, `QT_DISABLE_DEPRECATED_UP_TO` (to the minimum Qt you target); use new-style
  `connect` with explicit connection types; document thread affinity; avoid `deleteLater` races by
  parenting; disconnect/guard lifetimes with context objects.
* **Compiler/linker hardening:** stack protector, `_FORTIFY_SOURCE`, PIE/ASLR, RELRO + BIND_NOW,
  CET/CFI where available (Linux/clang), MSVC `/GS /guard:cf /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA
  /sdl`, macOS hardened runtime; warnings as errors (`/W4`, `-Wall -Wextra -Wconversion` selectively).
* **Analysis:** clang-tidy (bugprone, cert, cppcoreguidelines subsets), cppcheck, CodeQL/Semgrep,
  compiler analyzers (`/analyze`), Coverity/commercial if available.
* **Dynamic:** ASan/UBSan (Linux/macOS, MSVC ASan), TSan for threading, **fuzzing** (libFuzzer/AFL) of
  every parser: SSE, JSON envelope, deep-link parser, update manifest, SQLite migration inputs, file
  imports.
* **Memory/ownership of secrets:** zero secret buffers when done (`explicit_bzero`/`SecureZeroMemory`),
  avoid copying secrets into `QString` where avoidable.
* **Banned:** `system()`/`popen` with interpolated input, `QProcess::startDetached` with a shell string,
  `ignoreSslErrors`, `QLibrary` loading from user-controlled paths, `eval`-like facilities in QML
  (`Qt.createQmlObject` with remote strings, `Qt.include`), loading QML/JS from the network.

### 11.4 Code signing and trust

| OS | Requirement |
|---|---|
| Windows | Authenticode signing of all executables, DLLs and installers with a timestamp; signing keys in an HSM/cloud signing service (private keys for publicly trusted code-signing certificates must be hardware-protected); SmartScreen reputation planning; sign MSI/MSIX |
| macOS | Developer ID signing with **hardened runtime**, minimal entitlements, **notarization** and stapling, correct `Info.plist` privacy strings |
| Linux | Signed repositories (apt/rpm) with GPG; detached signatures for AppImage/tarballs; Flatpak via signed remote |

Signing happens in a **dedicated CI job** with isolated credentials and manual approval for releases;
signing keys are never on developer machines.

### 11.5 Privacy

Data minimization, purpose limitation, user-visible disclosure of what leaves the machine, telemetry
governed by policy, retention limits, purge on sign-out, data-subject requests handled by the backend
(`PLAYBOOK_AUDIT.md` §13). Crash dumps are opt-in and scrubbed.

### 11.6 TLS backend ADR

Qt uses platform TLS backends (for example Schannel on Windows, Secure Transport on Apple platforms,
OpenSSL on Linux) **(verify per Qt version)**. On older Windows builds the native stack may not offer TLS
1.3. Choose explicitly:

| Option | Pros | Cons |
|---|---|---|
| Native backends everywhere | Uses OS certificate store and enterprise trust, smaller footprint | Possible TLS 1.2-only on older Windows |
| Bundle OpenSSL backend on Windows | Uniform TLS 1.3 behavior | Bigger package, you own OpenSSL patching, verify enterprise CA handling |

Whatever the choice: keep TLS 1.2 acceptable on the server for the support window, track OpenSSL/CVE updates,
and document enterprise CA and TLS-inspection behavior.

---

## 12. Enterprise capabilities

### 12.1 Managed configuration (policy)

Policies override user settings; the UI shows locked controls as "managed by your organization".

| Source | Windows | macOS | Linux |
|---|---|---|---|
| Admin-set | Registry `HKLM\Software\Policies\<Vendor>\<Product>` (with **ADMX/ADML** templates; Intune/GPO) | Configuration profile / managed preferences (`/Library/Managed Preferences`) via `QSettings` native format **(verify)** | `/etc/<product>/policy.json` (+ optional dconf) |
| User-level | `HKCU` / app config | app preferences | `~/.config/<product>` |

Precedence: **managed > user > defaults**. A pure `PolicyResolver` (application layer) validates
values against a schema and exposes the effective configuration.

Recommended policy surface (mirrors the extension):

| Policy | Type | Effect |
|---|---|---|
| `TenantUrl` / `ApiBaseUrl` | string | Backend endpoint (validated against allowlisted patterns) |
| `SsoMode`, `IdpIssuer`, `ClientId` | string | SSO configuration |
| `UpdatesEnabled`, `UpdateChannel`, `UpdateWindow` | bool/enum/object | Update control, deferral |
| `UpdateMirrorUrl` | string | Internal update mirror |
| `RealtimeMode` | `auto\|websocket\|sse\|polling\|off` | Realtime transport control |
| `ProxyMode`, `ProxyUrl`, `CustomCaBundle` | enum/string/path | Network environment |
| `TelemetryLevel` | `off\|errors\|full` | Diagnostics governance |
| `CrashReportingEnabled` | bool | Crash dumps |
| `KillSwitch` | bool | Disable network features / force read-only |
| `FeatureFlags` | map | Central feature toggles |
| `DataRetentionDays` | int | Local retention |
| `AutostartMode` | enum | Start at login |
| `ForceSignInOnStartup`, `LockOnIdleMinutes` | bool/int | Session policy |
| `CompatibilityMode` | enum | Force software rendering/reduced effects |
| `LogLevel` | enum | Local log verbosity |
| `AllowUserSettings` | bool | Lock preferences |
| `SupportContact` | string | Shown in diagnostics UI |

### 12.2 Deployment

| Need | Mechanism |
|---|---|
| Silent install/uninstall | MSI properties (Windows), `.pkg` via MDM (macOS), `.deb/.rpm` via package managers/config management (Linux) |
| Per-machine vs per-user | Per-machine default for managed fleets; per-user installer optional |
| Update control | Policy-driven channel/window; internal mirror; ability to pin a version |
| Fleet visibility | Version, channel, OS, policy hash reported with sessions (subject to `TelemetryLevel`) |
| Store channels (optional) | Microsoft Store/MSIX, Mac App Store (sandbox constraints), Flathub/Snap, with separate test passes |
| Offline/air-gapped | Offline installers and update bundles with signature verification |
| Proxy/TLS inspection | System proxy by default, PAC/authenticated proxies (Negotiate/NTLM) tested **(verify per OS)**, enterprise CA support |

### 12.3 Kill switch, flags and remote config

Same model as the extension: policy `KillSwitch` plus signed remote config (data only, schema-validated, with
bundled safe defaults) can disable features or force read-only; minimum-version enforcement uses the
`upgrade-required` error and the updater. Rollback = ship a higher version with the fix + kill switch for
containment.

### 12.4 Support and audit

* **Diagnostics bundle** (user-initiated, redacted): app/Qt/OS/GPU/driver info, renderer in use,
  compatibility mode state, policy snapshot, effective config, realtime state and recent transitions,
  queue/outbox counts, storage sizes, recent logs and error events with correlation IDs, GUI-stall stats.
* **Audit events** to the backend (if policy allows): sign-in/out, policy applied, update applied,
  data export, kill-switch state. Redacted and rate-limited.
* **Runbooks** (section 15.6) and an admin guide are release artifacts.

---

## 13. Observability

| Signal | Implementation |
|---|---|
| Logging | `QLoggingCategory` per slice/layer, structured JSON lines via a custom message handler, redaction filter, rotating files with size caps, ring buffer for diagnostics |
| Crash reporting | Out-of-process crash handler (Crashpad/Breakpad/Sentry Native class), minidumps with symbols uploaded privately, PII scrubbing, opt-in by policy, release/channel tags |
| Metrics | Startup phases, time-to-interactive, GUI stalls (count, p95), frame/paint times, model update latency, SDK latency/error rates, realtime state/uptime/reconnects/lag, queue depth, memory, DB timings, update outcomes |
| Tracing | `correlationId` created at the user action, sent in API/realtime headers and logs |
| Health | Internal health projection (policy valid, session valid, API reachable, realtime state, storage OK) shown in diagnostics |
| Privacy | Telemetry level from policy; hosts hashed/truncated; no content in telemetry unless essential and allowed |

Server-side alerts watch error rates per client version, `upgrade-required` rates, update failures and
realtime reconnect storms.

---

## 14. Packaging, signing and updates

### 14.1 Build and packaging matrix

| OS | Artifacts | Notes |
|---|---|---|
| Windows | `windeployqt`-assembled folder → **MSI (WiX)** for enterprise, EXE or MSIX for consumers, ZIP portable (optional) | x64 and ARM64; bundle the MSVC runtime; sign everything; winget manifest optional |
| macOS | `.app` (universal2 or per-arch), `macdeployqt`, **notarized** `.dmg` and `.pkg` (MDM) | Hardened runtime, minimal entitlements, Sparkle-class updater or custom |
| Linux | AppImage, `.deb`, `.rpm`, Flatpak, tarball | Build on the oldest supported distro; bundle only what is needed; desktop file + AppStream metainfo; x86-64 and ARM64 |

### 14.2 Build rules

* CMake Presets per OS/arch/config; Ninja; compiler cache (ccache/sccache); locked dependency baseline
  (vcpkg manifest baseline or Conan lockfile); Qt installed from a pinned, verified source.
* **Reproducible builds** as far as practical (fixed toolchains in containers/images, `SOURCE_DATE_EPOCH`,
  deterministic archives); record hashes.
* Release builds: `-O2`, LTO, stripped binaries with **separate symbol files** archived and uploaded to the
  crash service; no debug code or development environment variable hooks.
* Generate the **third-party license report** and **SBOM** (CycloneDX/SPDX) in CI; include Qt notices and
  LGPL obligations (dynamic linking, relinking ability, license texts, source offer where required).
  Check each Qt module's license; some add-on modules are GPL/commercial only **(verify)**.

### 14.3 Update system

* **Channels:** `canary` (internal), `beta` (pilot), `stable`. Staged rollout by percentage/cohort with
  automatic halt on health regression.
* **Manifest:** signed (for example Ed25519) JSON listing version, channel, platform/arch, URL, hashes,
  size, minimum OS, release notes, `minSupportedVersion`, rollout cohort. Clients verify signature, hash and
  **monotonic version** before applying; HTTPS only.
* **Mechanism:** a vetted framework (Qt Installer Framework maintenance tool, Sparkle/WinSparkle-class,
  MSIX/Store updates) or a minimal custom updater with a separate **updater helper process**; apply on
  restart/idle; keep the previous version for **rollback**; verify OS signatures after download.
* **Delta updates** optional; full-package fallback always available.
* **Admin control:** policy to disable, defer, pin or point to an internal mirror.
* **Failure handling:** interrupted downloads resume; failed apply rolls back; telemetry reports outcomes.
* **No update without trust:** unsigned or downgrade manifests are rejected and reported.

### 14.4 Release channels and cadence

Align with the Qt LTS window and OS release cycles: test against new OS betas, schedule Qt patch upgrades
regularly (security), and maintain a documented end-of-support policy per app version.

---

## 15. Testing strategy

Follows `PLAYBOOK_TESTS.md`; adds a `desktop` side next to `backend` and `frontend`:

```text
tests/<slice>/desktop/{unit,integration,contract,e2e,perf,security,a11y,visual}/
tests/_platform/{architecture,desktop-perf,installer,updater,policy,realtime,security}/
```

### 15.1 Levels

| Level | Scope | Tooling (examples) |
|---|---|---|
| **Unit** | domain, application use cases with fake ports; pure helpers | Qt Test, GoogleTest or Catch2 |
| **ViewModel** | states/commands with fake ports, all statuses; `QSignalSpy`; no GUI | Qt Test |
| **Adapter integration** | SQLite (migrations from every released schema), SDK against a mock server, SSE/WebSocket transports against a test server (reconnect, resume, duplicates, out-of-order, heartbeat loss, proxy buffering, slow consumer), keystore adapters per OS | Qt Test + local test servers/containers, network fault injection |
| **Contract** | C++ SDK vs OpenAPI; realtime envelope vs schema; policy schema; route table parity; permission catalog parity | schema tests, shared fixtures |
| **Parity** | Same scenario fixtures across clients (status handling, error mapping, permission gating) | shared fixtures |
| **Component/View** | Views per ViewModel status | `QTest` GUI events, `QT_QPA_PLATFORM=offscreen`, screenshot compare |
| **E2E** | Critical journeys on real OS builds (sign-in, list/detail, edit, offline→online, realtime update, kill switch, update) | Qt Test GUI, OS accessibility-driven automation, Squish-class tools |
| **Accessibility** | Roles, names, focus order, keyboard-only, contrast on each OS | Accessibility Insights (Windows), Accessibility Inspector (macOS), Orca/Accerciser (Linux) |
| **Visual** | Layout per DPI (100/125/150/200%), themes (light/dark/high-contrast), locales/RTL, pseudo-locale | screenshot baselines |
| **Performance** | Section 2.5 budgets on the Floor profile | `QBENCHMARK`, startup harness, memory soak, GUI-stall counters |
| **Security** | Fuzzing (SSE/JSON/deep-link/manifest), IDOR and tenant isolation through SDK, authz matrix parity, TLS validation, DLL/plugin path tests, IPC tests, secrets-in-logs/dumps scans | libFuzzer, custom |
| **Installer/Updater** | Clean install, upgrade from N-1/N-2, uninstall, repair, silent install flags, rollback, signature/notarization verification, tamper and downgrade attempts | VM images per OS |
| **Policy** | Registry/plist/JSON policies applied; locked UI; invalid values ignored; kill switch; `RealtimeMode` behavior | scripted per OS |
| **Resilience** | Offline, flaky network, API outage, realtime storms, sleep/wake, disk full, DB corruption, clock skew | fault injection |
| **Architecture** | Layer targets, `#ifdef` only in `platform/`, forbidden APIs, include rules | scripts/clang-tidy |

### 15.2 CI matrix

* OS/arch: Windows 10 and 11 (x64; ARM64 if supported), Ubuntu LTS releases and one RPM-based distro
  (x64; ARM64), macOS current and N-1 (Apple Silicon and Intel).
* Display: Windows native, Linux X11 **and** Wayland, macOS; software rendering mode on all.
* Compilers: MSVC, GCC, Clang; Qt: pinned LTS plus an upcoming-version canary job.
* **Floor-profile runner:** constrained VM/container (2 vCPU, 4 GB, throttled IO, software GL) running the
  performance suite and the UI smoke suite.

### 15.3 Test rules

* No test touches production services or real tenant data; synthetic fixtures only.
* UI tests wait on observable conditions, never fixed sleeps; flaky tests are quarantined with an owner and
  expiry (see tests playbook).
* Every realtime transport has the **same conformance suite** (a transport that fails it cannot ship).
* Every parser has a fuzz target and a corpus of past bugs.

### 15.4 Performance gates

The perf job records startup phases, GUI stalls, memory after soak, and realtime burst CPU; compares with the
last release baseline; fails the build on regression thresholds; stores results as release evidence.

### 15.5 Evidence

Test reports, perf results, fuzz run summaries, SBOM, signing/notarization logs, installer test logs and
approvals feed the **release evidence bundle** (`PLAYBOOK_TESTS.md` §10, `PLAYBOOK_AUDIT.md` §8).

---

## 16. CI/CD and release governance

| Stage | Gate |
|---|---|
| Pre-commit | format, clang-tidy on changed files, secrets scan |
| PR | build all OS/arch (matrix subset), unit/ViewModel/contract/architecture, SAST, dependency scan, perf smoke |
| Main | full matrix, integration, e2e smoke, sanitizer builds, fuzz (time-boxed), SBOM, license report |
| Nightly | full e2e, Floor-profile perf, soak, installer/upgrade, realtime conformance, extended fuzz, Qt canary |
| Release candidate | all above + signing (isolated job, approval), notarization, update manifest dry run, accessibility audit, evidence bundle |
| Release | staged rollout (canary → beta → stable), health gates, automatic halt |
| Post-release | monitoring of crash/stall/update metrics, rollback plan (higher version + kill switch) |

Runbooks (required): broken release/rollback, update failure wave, signing key compromise, notarization/SmartScreen
issues, crash spike, GPU/driver regression (blocklist + Compatibility Mode), realtime outage (force polling), policy
misconfiguration, IdP outage, vulnerability response (below).

### 16.1 Vulnerability handling and CRA

A desktop app distributed to customers is a **product with digital elements** for EU purposes: plan
accordingly **(confirm applicability with counsel)**. Reporting obligations for manufacturers apply from
11 September 2026 and most other obligations from 11 December 2027. Required capabilities:

* coordinated vulnerability disclosure policy and contact (`security.txt`-style page);
* SBOM and dependency monitoring (Qt, third-party libs) with fix SLAs;
* ability to ship **security updates** for the declared support period (align with Qt LTS maintenance);
* internal process to report actively exploited vulnerabilities and severe incidents within the
  required timelines;
* secure-by-default configuration, documented hardening, and conformity documentation;
* Qt 6.12's CRA readiness covers Qt's part; your application still needs its own compliance work.

---

## 17. Governance and compliance

* **Ownership:** CODEOWNERS for `cmake/**`, `src/platform/**`, `src/infrastructure/{updater,policy,crypto}/**`,
  `packaging/**`, signing workflows, `docs/permissions-register.md`; security review for those paths.
* **Documents:** ADRs (D1–D15 and later), support matrix, threat model, data-flow diagrams, privacy review,
  admin guide, runbooks, license/SBOM reports.
* **Mapping to SOC 2 / ISO 27001 / ISO 9001** (`PLAYBOOK_AUDIT.md` §4): secure SDLC (CI gates, reviews,
  analysis, fuzzing), change management (release approvals, staged rollout), cryptography/key custody
  (signing keys in HSM, update trust root), vulnerability management (SBOM, scanning, SLAs), logging
  (telemetry, audit events), vendor management (Qt, dependencies, crash service), availability (kill switch,
  rollback, offline mode), privacy (telemetry policy, retention), quality (KPIs below).
* **Quality KPIs:** crash-free sessions, GUI-stall rate, startup time p50/p95 on Floor, update success rate,
  realtime uptime, ANR-like freeze reports, support tickets per release, defect escape rate.
* **Periodic:** quarterly review of the support matrix, GPU blocklist and dependency baseline; annual pen test
  of the desktop client and updater; after each OS major release a compatibility pass.

---

## 18. Working with Claude Code on this project

Per `PLAYBOOK_CLAUDE_CODE.md`:

* A path-scoped rule for `apps/desktop/**` with the layer rules, "no `#ifdef` outside `platform/`", "GUI thread
  never blocks", "no embedded browser", realtime rules, and links to this playbook (not `@`-imported).
* **Role policy:** the integrator edits `cmake/**`, `packaging/**`, signing workflows, dependency baselines and
  generated contracts; security reviewer checks permission/policy/updater/crypto changes; hooks and settings stay
  human-owned.
* **Skills:** `new-desktop-feature` (slice scaffold: ports, ViewModel, View, tests), `new-realtime-topic`
  (schema, handler, resync rule, conformance test), `new-policy-key` (schema, ADMX/plist/JSON, UI lock, test),
  `perf-regression` (run Floor harness, diff budgets).
* **Specs** include: contexts/threads touched, ports added, permissions, policy keys, realtime topics, performance
  impact against budgets, accessibility notes, per-OS considerations, and test levels per AC.

---

## 19. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| Blocking calls on the GUI thread | Freezes on old machines |
| `processEvents()` loops, `sleep`, `QFuture::result()` on GUI thread | Re-entrancy bugs and stalls |
| Embedded Chromium/WebEngine for "easy" screens | Memory/startup/attack-surface blowout |
| Qt Quick effects (`ShaderEffect`, heavy layers, blur) without fallback | Broken or slow on old GPUs |
| GPU-only features without a software path | App unusable on weak/old graphics |
| One `QNetworkAccessManager` shared by API calls and a long-lived stream | Starves HTTP/1.1 slots |
| Realtime as the only data path | Stale/incorrect when the stream fails |
| Treating events as the state | Missed events corrupt the cache; use snapshot + stream |
| Unbounded caches/queues/log files | Memory/disk growth on small machines |
| `#ifdef Q_OS_*` scattered through business code | Untestable, platform drift |
| Business logic in widgets/QML | Duplicated, untested |
| Secrets in `QSettings`, files, logs, dumps | Local theft |
| `ignoreSslErrors`, unsigned updates, update over HTTP | Takeover |
| Tokens in URLs (SSE/WS query strings) | Leaks in logs/proxies |
| Loading plugins/DLLs from user-writable or CWD paths | Hijacking |
| Hand-written API paths/DTOs | Contract drift |
| Ignoring Wayland/HiDPI/RTL/accessibility | Field failures |
| No Floor-profile testing | Performance claims unproven |
| Installer without silent mode/policies | Not enterprise-deployable |
| Static Qt linking under LGPL without legal review | License violation |

---

## 20. Definition of done and checklists

### 20.1 Feature DoD

- [ ] Spec covers layers, ports, permissions, routes, realtime topics, policy keys, perf impact, a11y, per-OS notes.
- [ ] Domain/application tested with fake ports; ViewModel statuses covered; no GUI-thread blocking.
- [ ] Uses the generated SDK; no hand-written paths/DTOs; error taxonomy handled (`forbidden`, `offline`, …).
- [ ] Works with realtime on, degraded (polling) and off; resync path tested.
- [ ] Works in Compatibility Mode (software rendering) and meets Floor-profile budgets.
- [ ] Keyboard/screen-reader accessible; HiDPI and dark/high-contrast verified; strings are translation keys.
- [ ] Policy-aware (locked controls, kill switch); telemetry with correlation ID and redaction.
- [ ] Storage changes have migrations and fixtures; no secrets persisted outside the keystore.
- [ ] Parser/IPC inputs have fuzz targets; security tests updated.

### 20.2 Release checklist

- [ ] Reproducible build; hashes recorded; SBOM and license report generated.
- [ ] Sanitizer, static analysis, fuzz and dependency scans green.
- [ ] Unit/integration/contract/e2e/a11y/visual suites green on the CI matrix; Floor perf within budget.
- [ ] Installer install/upgrade/uninstall/silent tests green on clean VMs; policies verified.
- [ ] Signed (Authenticode/Developer ID/GPG), notarized (macOS), signatures verified post-build.
- [ ] Update manifest signed; rollback tested; staged rollout and halt criteria configured.
- [ ] Admin guide, support matrix, changelog and permission/policy changes documented.
- [ ] Canary soak complete; crash/stall/update metrics healthy.

### 20.3 Architecture acceptance checklist

- [ ] Qt LTS version pinned; C++20; CMake presets; locked dependency baseline.
- [ ] Layers are separate CMake targets with enforced dependencies; `#ifdef` only in `platform/`.
- [ ] Widgets-first UI; Qt Quick restricted and with Compatibility Mode and safe-mode fallback.
- [ ] MVVM with C++ ViewModels; Views contain no business logic; navigation by route.
- [ ] GUI-thread watchdog; async ports (`QFuture<Result<T>>`); no blocking on the GUI thread.
- [ ] Slices mirror backend slices; generated DTOs/SDK; permission catalog and route table shared.
- [ ] Realtime port with SSE, WebSocket, polling and off; state machine, resume, dedupe, resync, backpressure.
- [ ] SQLite storage with migrations, outbox, retention, encryption for Restricted data.
- [ ] OIDC + PKCE in system browser; secrets in OS keystore; policy-controlled configuration and kill switch.
- [ ] Signed, notarized, hardened builds; signed update channel with rollback; CRA/vulnerability process defined.
- [ ] Observability: structured logs, crash reporting, metrics, diagnostics bundle.
- [ ] Test pyramid including perf-on-Floor, installer/updater, policy, realtime conformance and fuzzing.

---

## Appendix A — CMake skeleton (illustrative)

```cmake
cmake_minimum_required(VERSION 3.25)
project(heimdall_desktop LANGUAGES CXX VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)       # LTO

find_package(Qt6 6.12 REQUIRED COMPONENTS Core Gui Widgets Network Sql WebSockets LinguistTools)

include(cmake/hardening.cmake)                           # per-compiler flags (Appendix B)

add_library(desktop_domain STATIC ...)                   # links: shared_kernel only
add_library(desktop_application STATIC ...)              # links: desktop_domain, Qt6::Core
add_library(desktop_infrastructure STATIC ...)           # links: application, Qt6::Network, Qt6::Sql, Qt6::WebSockets
add_library(desktop_platform STATIC ...)                 # OS specifics via if(WIN32)/APPLE/UNIX
add_library(desktop_presentation STATIC ...)             # links: application, Qt6::Widgets, ui_kit
add_executable(heimdall WIN32 MACOSX_BUNDLE src/app/main.cpp)
target_link_libraries(heimdall PRIVATE desktop_presentation desktop_infrastructure desktop_platform)

target_compile_definitions(heimdall PRIVATE
  QT_NO_CAST_FROM_ASCII QT_NO_CAST_TO_ASCII QT_NO_URL_CAST_FROM_STRING QT_NO_FOREACH
  QT_DISABLE_DEPRECATED_UP_TO=0x060800)
```

## Appendix B — Hardening flags (illustrative)

```text
MSVC compile:  /W4 /WX /sdl /GS /guard:cf /permissive- /Zc:__cplusplus
MSVC link:     /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /guard:cf /CETCOMPAT
GCC/Clang:     -Wall -Wextra -Wconversion(-selective) -fstack-protector-strong -fstack-clash-protection
               -D_FORTIFY_SOURCE=2 (or 3 where supported) -fPIE -fcf-protection
Linker (ELF):  -pie -Wl,-z,relro,-z,now -Wl,-z,noexecstack
macOS:         hardened runtime at codesign; minimal entitlements; library validation on
Sanitizer CI:  -fsanitize=address,undefined (and thread in a separate job)
Baseline CPU:  no -march=native; default x86-64 baseline; runtime dispatch in SIMD libs
```

## Appendix C — Realtime port and state (sketch)

```cpp
struct RealtimeEvent {
  QString id;        // monotonic per stream (resume token)
  QString type;      // "funnels.lead.updated"
  QString tenantId;
  QString topic;     // "funnels.leads.<id>"
  qint64  ts = 0;
  int     v  = 1;
  QByteArray payload;   // small JSON, schema-validated by the transport/router
};

class RealtimePort : public QObject {
  Q_OBJECT
public:
  enum class State { Disabled, Disconnected, Connecting, Live, Reconnecting, Degraded };
  Q_ENUM(State)
  virtual void start(const RealtimeConfig&) = 0;    // transport from policy + capabilities
  virtual void stop() = 0;
  virtual void setTopics(const QSet<QString>&) = 0; // sync with visible screens
signals:
  void stateChanged(State s);
  void eventReceived(const RealtimeEvent& e);       // already deduplicated
  void resyncRequired(const QSet<QString>& topics); // gap/overflow/resume rejected
};
```

## Appendix D — SSE parser rules (conformance checklist)

```text
- Decode UTF-8 (strip leading BOM); lines end with \n, \r or \r\n
- A line starting with ':' is a comment (heartbeat)
- field:value (single optional space after ':') — fields: event, data, id, retry
- 'data' lines accumulate joined by '\n'; dispatch on empty line; empty data → no event
- 'id' sets last-event-id (ignore if it contains NUL); 'retry' (digits only) sets reconnection delay
- Unknown fields ignored; enforce max line length and max event size
- On reconnect send Last-Event-ID; honor retry; stop on HTTP 204 (server says do not reconnect)
- Treat non-200 or wrong Content-Type as failure → fallback path
```

## Appendix E — Renderer safe-mode logic (sketch)

```cpp
// main.cpp, before creating any window
const bool crashedLastTime = settings.value("startup/inProgress", false).toBool();
const bool forceSoftware   = policy.compatibilityMode() == Compat::Force
                          || cli.has("--software-rendering")
                          || crashedLastTime
                          || gpuBlocklist.matches(probeGpuInfo());
settings.setValue("startup/inProgress", true);

if (forceSoftware) {
  qputenv("QT_QUICK_BACKEND", "software");            // Qt Quick surfaces only; Widgets unaffected
  appState.setCompatibilityMode(true);                // disables non-essential animation/transparency
}
// ... create QApplication, composition root, first window ...
QObject::connect(firstWindow, &MainWindow::firstFrameRendered, [&]{
  settings.setValue("startup/inProgress", false);     // healthy start confirmed
});
```

## Appendix F — Policy schema (JSON form; same keys in ADMX/plist)

```json
{
  "type": "object",
  "properties": {
    "TenantUrl":        { "type": "string" },
    "SsoMode":          { "type": "string", "enum": ["oidc", "local"] },
    "UpdatesEnabled":   { "type": "boolean" },
    "UpdateChannel":    { "type": "string", "enum": ["stable", "beta", "canary"] },
    "RealtimeMode":     { "type": "string", "enum": ["auto", "websocket", "sse", "polling", "off"] },
    "TelemetryLevel":   { "type": "string", "enum": ["off", "errors", "full"] },
    "CompatibilityMode":{ "type": "string", "enum": ["auto", "force", "off"] },
    "KillSwitch":       { "type": "boolean" },
    "FeatureFlags":     { "type": "object", "additionalProperties": { "type": "boolean" } },
    "DataRetentionDays":{ "type": "integer", "minimum": 0 },
    "AllowUserSettings":{ "type": "boolean" }
  },
  "additionalProperties": false
}
```

## Appendix G — Example vertical slice flow: update a lead's stage (desktop)

```text
View (LeadDetailView) → ViewModel (LeadDetailViewModel::moveToStage)
  → use case MoveLeadStage (checks can('funnels.leads.update') as UX hint, builds IdempotencyKey)
  → LeadRepositoryPort::updateStage → SDK: PATCH /api/v1/app/funnels/leads/{leadId}
     (Authorization, X-Tenant-Id, X-Correlation-Id, Idempotency-Key) → backend authorizes (RBAC, tenant, ownership)
  → Result<Lead> → ViewModel status Ready / Forbidden(requiredPermission) / Conflict / Offline(outbox)
  ← Realtime event "funnels.lead.updated" (topic funnels.leads.<id>) → dedupe → handler → cache refresh
     → coalesced model update (≤ 30 Hz) → View repaints the affected row only
Tests: unit (policy/domain), ViewModel statuses, adapter integration (SDK mock), realtime conformance,
       parity fixture vs web/mobile, authz-matrix row, Floor-profile list-scroll perf.
```

## Appendix H — Support matrix document template

```markdown
# Support matrix (generated per release)
| OS | Min build | Archs | Display servers | Status | End of support |
| Windows 10 | 22H2 | x64 | n/a | Supported until <date> | <date> |
| Windows 11 | 23H2+ | x64, arm64 | n/a | Supported | |
| macOS | <N-2> | arm64, x86-64 | n/a | Supported | |
| Ubuntu LTS | 22.04, 24.04 | x64, arm64 | X11, Wayland | Supported | |
Minimum hardware (Floor): 2 cores, 4 GB RAM, software rendering supported.
Known limitations: <tray on some Wayland desktops, global shortcuts on Wayland, ...>
Qt version: 6.12 LTS (maintenance until <date>)
```
