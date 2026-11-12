# PLAYBOOK_CHROME_EXTENSION.md — Enterprise-grade Chrome extension playbook (TypeScript, Manifest V3)

Architecture and engineering contract for building, shipping and operating a
**Chrome extension at enterprise grade**: strict TypeScript, layered
(hexagonal) architecture, MVVM UIs, a typed SDK, hardened security,
administrator-controlled configuration, observable and testable, released
through a governed pipeline.

It extends `PLAYBOOK_FRONTEND.md` (MVVM + SDK, extension section),
`PLAYBOOK_URLS.md` (route/permission model), `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_TESTS.md`, `PLAYBOOK_AUDIT.md` and `PLAYBOOK_CLAUDE.md`. It is
project-agnostic: replace `<product>`, `<tenant>` and `<api>` with your own.

The rules that carry the whole file:

1. **Manifest V3 only.** Event-driven service worker, no remote code, no
   persistent background page.
2. **The service worker is the brain; everything else is a client of it.**
   It owns the session, the SDK, storage writes, queues and policy. UIs and
   content scripts are thin.
3. **The page is hostile.** Content scripts and any message from a web page
   are untrusted input; tokens never reach them.
4. **Layers do not leak.** `domain` and `application` never touch `chrome.*`
   or the DOM; platform access goes through ports.
5. **State is durable and the worker is mortal.** Nothing important lives in
   a global variable; every handler is resumable and idempotent.
6. **Least privilege, provably.** Every permission has a justification, an
   owner and a test; host access is narrow and admin-restrictable.
7. **Administrators are first-class users.** Configuration, rollout, kill
   switches, diagnostics and audit are designed in, not bolted on.
8. **Everything is typed at the boundary.** Messages, storage, policy and API
   payloads are validated schemas.
9. **Releases are boring.** Reproducible builds, permission diffs, staged
   rollout, rollback by kill switch.

> Verification note: the Chrome extension platform changes frequently. Facts
> below come from Chrome's documentation and engineering practice as of
> November 2026; items marked **(verify)** are version- or limit-dependent and
> must be re-checked against <https://developer.chrome.com/docs/extensions>
> and the Chrome Web Store developer policies before relying on them. Numeric
> limits (storage quotas, timers, review times) change; keep them as
> configuration constants with a source link, not as magic numbers.

---

## 1. Decisions up front

| Decision | Choice | Rationale |
|---|---|---|
| Manifest version | **MV3** (MV2 is no longer a supported path for new or existing listings) | Platform requirement |
| Language | **TypeScript**, `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` | Compile-time safety at every boundary |
| Build tool | **WXT** (Vite-based; maintained, cross-browser, generates manifest from conventions) as default; **plain Vite + CRXJS** if you need full control of the Vite config; avoid Plasmo for new projects (reported to be in maintenance mode) | Keeps the build a replaceable adapter; architecture below is tool-independent |
| UI | React + TypeScript, **MVVM** (shared ViewModels from the frontend playbook), Tailwind or CSS modules | Same pattern as web/mobile |
| Validation | **zod** (or valibot) schemas for messages, storage, policy, API | Avoid libraries that compile validators with `eval`/`new Function` under CSP (see 13.4) |
| API access | Shared **SDK** package, executed **only in the service worker** | Single place for tokens, retry, tenant headers |
| IPC | Typed RPC + events over `chrome.runtime` messaging/ports | One contract, validated both sides |
| State | SW as source of truth; `storage.local/session/sync/managed` + IndexedDB by purpose | Survives worker termination |
| Remote code | **Never**. Remote *data* (config, selectors) only, schema-validated and signed | Store policy + security |
| Distribution | Chrome Web Store (public or private/unlisted) **and** enterprise force-install via policy; optional self-hosted CRX feed | Enterprise reach |
| Config for admins | `storage.managed` with a JSON Schema + `ExtensionSettings` guidance | Policy over user choice |
| Browsers | Chrome first; Edge works (Chromium); Firefox/Safari through the build adapter if required | Section 19 |

---

## 2. Platform model

### 2.1 Execution contexts

| Context | Purpose | DOM | Lifetime | Can call SDK/API? | Trust level |
|---|---|---|---|---|---|
| **Service worker** (`background`) | Event router, session, SDK, jobs, policy, orchestration | No | Event-driven; terminated when idle | **Yes (only here)** | Trusted |
| **Popup** | Quick actions | Yes | Destroyed on close | Through SW | Trusted (extension origin) |
| **Side panel** | Persistent workspace per tab or global | Yes | While open | Through SW | Trusted |
| **Options page** | Settings, diagnostics | Yes | While open | Through SW | Trusted |
| **Offscreen document** | DOM/audio/clipboard/parsing tasks the worker can't do | Yes (hidden) | Independent of the worker | Through SW | Trusted |
| **Content script** (isolated world) | Read/augment host pages | Page DOM | Page lifetime | **No** | **Untrusted surroundings** |
| **Injected main-world script** | Access page JS objects (rare) | Page | Page lifetime | **No** | **Hostile** |
| **Web-accessible page/iframe** | Embeddable extension UI in a page | Yes | Frame lifetime | Through SW | Trusted code, hostile embedder |
| **Native messaging host** (optional) | Desktop integration | n/a | Process | n/a | Separate trust domain |

### 2.2 Service worker lifetime (design consequences)

Documented behavior **(verify limits)**: Chrome terminates an idle extension
service worker after about 30 seconds without events or extension API calls;
a single event handler can run roughly five minutes at most; a single fetch
response wait is also bounded (about 30 seconds). Consequences enforced by
this playbook:

* **Register every event listener synchronously at the top level** of the
  worker entry file. Listeners added after an `await` or inside a callback miss
  the event that woke the worker.
* **No state in module globals** that must outlive an event. Persist it
  (`storage.session` for volatile, `storage.local`/IndexedDB for durable).
* **No `setTimeout`/`setInterval` for durations that matter.** Use
  `chrome.alarms` (minimum period is bounded, currently 30 seconds **(verify)**).
* **Long work** is chunked into resumable steps with persisted checkpoints, or
  moved to an offscreen document, or delegated to the backend.
* **No keep-alive hacks** (pinging ports to stay alive). If a flow needs
  minutes of continuous execution, redesign it.
* **Dynamic `import()` is not available in service workers**; bundle
  statically and keep the worker small for fast cold start **(verify)**.
* **No DOM, `window`, `XMLHttpRequest`, `localStorage`.** Use `fetch`,
  `chrome.storage`, IndexedDB, `navigator.locks`.
* The worker may start **cold on any event**, including while the user is
  mid-flow; every handler must rebuild its dependencies from storage.

### 2.3 Storage areas

| Area | Persistence | Typical limits **(verify)** | Use for |
|---|---|---|---|
| `storage.session` | In memory; cleared on browser restart/extension reload | ~10 MB | Access tokens, caches, ephemeral flow state; keep access level to trusted contexts |
| `storage.local` | Disk, per profile | ~10 MB (more with `unlimitedStorage`) | Settings, queues, last-known projections |
| `storage.sync` | Synced across the user's devices | ~100 KB total, small per-item and rate limits | Tiny user preferences (and only if policy allows sync) |
| `storage.managed` | Read-only, from admin policy | Schema-defined | **Enterprise configuration** |
| IndexedDB | Disk, large | Quota-based | Large datasets, offline queues, diagnostics ring buffer |
| `navigator.locks` | n/a | n/a | Cross-context mutual exclusion |

---

## 3. Architecture: hexagonal layers + MVVM + SDK

### 3.1 Layers

```text
┌──────────────────────────────── extension ────────────────────────────────┐
│  INBOUND ADAPTERS (drive the app)             OUTBOUND ADAPTERS (driven)  │
│  runtime messages · alarms · commands         storage · identity · tabs   │
│  context menus · webNavigation · action       scripting · sidePanel       │
│  content-script DOM observers · UI Views      offscreen · notifications   │
│              │                                 downloads · native msg     │
│              ▼                                          ▲                 │
│        APPLICATION (use cases, ports, policy)  ─────────┘                 │
│                 │            SDK port ──► [SDK] ──► backend API           │
│                 ▼                                                         │
│        DOMAIN (entities, value objects, rules, domain events)             │
└───────────────────────────────────────────────────────────────────────────┘
```

| Layer | Contains | May import | Must not import |
|---|---|---|---|
| **domain** | Entities, value objects, invariants, domain events, pure services | `shared-kernel` | `chrome`, DOM, React, SDK, zod-less I/O |
| **application** | Use cases, **ports** (interfaces), policy resolver, job definitions, orchestration | domain, shared-kernel, `contracts` | adapters, `chrome`, DOM |
| **adapters/in** | RPC handlers, alarm/command/menu/navigation listeners, content-script observers, UI Views | application, contracts, platform | other adapters' internals |
| **adapters/out** | Storage, identity, tabs, scripting, notifications, offscreen, SDK client, clock, logger | application ports, platform, SDK | domain internals beyond the port types |
| **platform** | Thin typed, promise-based wrappers over `chrome.*` (the only place `chrome` is referenced) | `chrome` types | application/domain |
| **contracts** | Message schemas, DTOs, error codes, policy schema types | zod, shared-kernel | everything else |
| **ui (View)** | React components, layouts, styling | ViewModel hooks, `ui-*` kit | SDK, `chrome`, storage |
| **viewmodels** | Presentation state/commands | SDK port types, `core` ports | `react-dom`, `chrome`, router libs |

Enforce with `dependency-cruiser` or `eslint-plugin-boundaries` plus
`no-restricted-globals` for `chrome`/`browser` outside `platform/`. A
violation fails CI.

### 3.2 Ports (examples)

Ports are owned by the application layer, named in domain language:

```ts
export interface SessionPort      { current(): Promise<Session | null>; signIn(): Promise<void>; signOut(): Promise<void>; }
export interface PolicyPort       { get(): Promise<EffectivePolicy>; onChange(cb: () => void): Unsubscribe; }
export interface KeyValueStore<T> { get(): Promise<T>; update(fn: (v: T) => T): Promise<T>; }
export interface TabsPort         { active(): Promise<TabInfo | null>; send<M>(tabId: number, m: M): Promise<void>; }
export interface InjectorPort     { ensureInjected(tabId: number): Promise<void>; }
export interface JobQueuePort     { enqueue(job: JobSpec): Promise<void>; drain(budgetMs: number): Promise<void>; }
export interface NotifierPort     { notify(n: Notice): Promise<void>; }
export interface ClockPort        { now(): number; }
export interface LoggerPort       { log(level: Level, msg: string, ctx?: Record<string, unknown>): void; }
export interface ApiPort          { /* generated from the SDK: leads, funnels, ... */ }
```

### 3.3 Composition roots

Each execution context has one composition root that wires ports to adapters;
business code never instantiates infrastructure:

| Context | Root | Wires |
|---|---|---|
| Service worker | `entrypoints/background.ts` | Full container: storage, SDK, session, policy, queue, tabs, injector, RPC server |
| Popup / side panel / options | `entrypoints/<surface>/main.tsx` | RPC **client** adapters as ports for ViewModels |
| Content script | `entrypoints/content/*.content.ts` | DOM adapters, RPC client, site adapters |
| Offscreen | `entrypoints/offscreen/main.ts` | The specific capability + RPC client |

### 3.4 Vertical slicing

Follow the product's slice names: `src/features/<slice>/{domain,application,adapters,ui,viewmodels}`.
Cross-slice access only through each slice's `contracts/`. Cross-cutting
infrastructure (platform, messaging, storage, policy, telemetry) lives in
`src/platform/` and `src/shared-kernel/`.

---

## 4. Directory structure

```text
apps/extension/
├── wxt.config.ts                     # build adapter: manifest, env, per-channel config
├── tsconfig.json                     # strict; path aliases; types limited per folder
├── entrypoints/                      # THIN: composition roots only (framework conventions)
│   ├── background.ts
│   ├── content/                      # one file per matched site family
│   │   └── <site>.content.ts
│   ├── popup/{index.html, main.tsx}
│   ├── sidepanel/{index.html, main.tsx}
│   ├── options/{index.html, main.tsx}
│   └── offscreen/{index.html, main.ts}
├── public/_locales/<lang>/messages.json   # chrome.i18n
├── public/icons/…
├── managed-schema.json               # storage.managed schema (admin policy)
├── src/
│   ├── shared-kernel/                # ids, Result, errors, time, branded types
│   ├── contracts/                    # zod: messages, DTOs, policy, storage schemas, error codes
│   ├── platform/                     # ONLY place that touches chrome.*
│   │   ├── runtime.ts  storage.ts  tabs.ts  scripting.ts  identity.ts
│   │   ├── alarms.ts   sidepanel.ts  offscreen.ts  notifications.ts  locks.ts
│   ├── messaging/                    # typed RPC client/server, ports, sender validation
│   ├── storage/                      # typed stores, schema versions, migrations
│   ├── policy/                       # managed policy reader + resolver (admin > user)
│   ├── telemetry/                    # logger, redaction, ring buffer, error reporter, metrics
│   ├── security/                     # sender allowlist, URL/host matchers, sanitizers, CSP helpers
│   ├── jobs/                         # persisted queue, retry/backoff, scheduler
│   └── features/
│       └── <slice>/
│           ├── domain/  application/  adapters/{in,out}/  contracts/  ui/  viewmodels/
packages/                              # shared with web/mobile (see frontend playbook)
├── sdk/            # typed API client (no React, no chrome)
├── core/           # ports + route builders + permission helpers
├── viewmodels/     # shared ViewModels
├── ui-web/         # design system (DOM)
└── tokens/
tests/<slice>/frontend/{unit,component,integration,contract,e2e/extension,a11y,security,performance}/
tests/_platform/{architecture,security,policy,resilience,performance}/
docs/{adr,permissions-register.md,data-flows.md,admin-guide.md,runbooks/}
```

Rules: entrypoints stay under ~50 lines each; everything else is imported
from `src/`. Slice names match backend slices, URL `{module}` and permission
prefixes (`funnels.leads.read`).

---

## 5. Manifest and permission engineering

### 5.1 Manifest rules

* Generated from code (WXT config) per **channel** (dev, staging, prod) with
  distinct names, icons, `key` (extension IDs) and API base URLs. **Never edit
  the generated manifest by hand.**
* Mandatory fields: `manifest_version: 3`, `name`/`description` via
  `__MSG_…__`, `default_locale`, `version` (up to four dot-separated integers)
  and optional `version_name`, `minimum_chrome_version` (the oldest Chrome you
  support under enterprise channels), `key` (stable ID, see 5.4),
  `content_security_policy`, `storage.managed_schema`, `incognito`
  (decide explicitly: `not_allowed` is the safe default for enterprise).
* Declare **only** the entrypoints used; every `content_scripts` entry has the
  narrowest `matches`, `all_frames: false` unless justified, explicit
  `run_at`, and an explicit `world`.
* `web_accessible_resources` only for assets a page genuinely needs, with
  specific `matches` (and dynamic URLs where supported **(verify)**).
* `externally_connectable` only for the product's own origins. Omit if the web
  app talks to the extension through a content-script bridge instead.

### 5.2 Permission design

| Principle | Practice |
|---|---|
| Minimal | Start with `storage`, and add APIs only when a use case needs them |
| Optional first | Features that need extra APIs or hosts use `optional_permissions` / `optional_host_permissions` and request at runtime with a user gesture |
| Narrow hosts | `host_permissions` limited to the product API origin(s); customer sites via `activeTab`, programmatic injection, or `optional_host_permissions` driven by admin policy |
| No `<all_urls>` by default | If unavoidable, justify in the register, gate behind policy, and expose admin `runtime_blocked_hosts` guidance |
| Justified | Each permission appears in `docs/permissions-register.md`: why, which feature, data touched, owner, review date, alternative considered |
| Diffed | CI fails on any permission/host change without an updated register entry and a CODEOWNERS security review (permission increases can disable the extension until the user accepts, or require admin re-approval) |

Typical permission map:

| Capability | Permission | Notes |
|---|---|---|
| Settings, queues | `storage` | `unlimitedStorage` only if proven necessary |
| Scheduled work | `alarms` | Replaces timers |
| Side panel UI | `sidePanel` | `open()` requires a user gesture |
| Programmatic injection | `scripting` + host access or `activeTab` | Preferred over broad static matches |
| SPA navigation signals | `webNavigation` | Narrow with URL filters |
| Offscreen tasks | `offscreen` | One document at a time **(verify)**; choose the specific `reason` |
| Identity/SSO | `identity` | `launchWebAuthFlow` with PKCE |
| Notifications | `notifications` (optional) | |
| Downloads/diagnostics export | `downloads` (optional) | |
| Header changes | `declarativeNetRequest` | Static rulesets reviewed like code |
| Passive network observation | `webRequest` (non-blocking) | Blocking `webRequest` is reserved for policy-installed extensions **(verify)** |

### 5.3 Content Security Policy

* Keep the default extension-pages policy (`script-src 'self'; object-src
  'self'`); never add `unsafe-eval` or `unsafe-inline` (not allowed anyway).
* Consider adding `connect-src` limited to the API origin(s) for extension
  pages (and verify it also constrains the service worker in your Chrome
  versions) and `base-uri 'none'`.
* WebAssembly requires `wasm-unsafe-eval`; add it only if a wasm module is
  essential, and review the module's provenance.

### 5.4 Stable extension ID

Pin the ID with the manifest `key` (public key) per channel. Needed for:
OAuth redirect URIs (`https://<id>.chromiumapp.org/`), `externally_connectable`
allowlists, enterprise policies keyed by ID, and native messaging allowlists.
Keep the private key under custody (HSM/KMS or offline vault) with documented
recovery; losing it, or leaking it, is an incident (runbook in 18.6).

---

## 6. Messaging and IPC

### 6.1 Principles

* **One typed protocol** in `contracts/`; both ends validate with the same
  schemas. No ad-hoc `sendMessage` calls in feature code.
* **Request/response (RPC)**, **events** (SW → UIs/tabs), and **streams**
  (long-lived `runtime.connect` ports, for progress). Ports must tolerate
  disconnects, since the worker can terminate.
* Envelope: `{ v, id, type, correlationId, payload }`; `v` is the protocol
  version; unknown versions/types are rejected with a typed error.
* **Errors are values** (`Result<T, RpcError>`), never thrown across contexts.
  Error codes align with the SDK's `SdkError` kinds.
* Encapsulate the platform's callback quirks (`sendResponse` + `return true`
  for async replies) in one transport adapter.

### 6.2 Sender validation (mandatory)

Every `onMessage`/`onConnect` handler first checks:

| Check | Rule |
|---|---|
| `sender.id` | Equals `chrome.runtime.id` (reject other extensions unless explicitly allowlisted) |
| Origin/URL | Extension pages: `chrome-extension://<id>/…`; content scripts: `sender.tab` present and `sender.url` matches the content-script allowlist; web pages (`onMessageExternal`): exact allowlisted origins only |
| Frame | `sender.frameId` expected (top frame vs iframe) per message type |
| Capability | A **message permission table** maps each `type` to allowed sender classes (`ui`, `content`, `external`). Content scripts can call only a small subset (for example `lead.capture`, never `session.get`) |
| Payload | Parse with the zod schema; reject unknown fields; size-limit |

Never accept commands from a page that would change policy, tokens or
settings. Treat every content-script message as untrusted data from a
compromised renderer.

### 6.3 Projections and subscriptions

The SW publishes **read projections** (session summary, queue status, policy
summary). UIs fetch them via RPC and subscribe through `storage.onChanged`
(simple) or a port (streams). UIs never reconstruct state from raw storage
keys.

### 6.4 Page ↔ extension bridging

If the product's web app needs to talk to the extension:

* Prefer `externally_connectable` with exact origins; otherwise a content-script
  bridge using `window.postMessage` with a per-page **nonce handshake**, strict
  `event.origin`/`event.source` checks and a tiny allowed message set.
* The bridge never exposes tokens or generic RPC; it exposes named, validated
  commands.

---

## 7. State, storage and migrations

### 7.1 Typed stores

All storage goes through typed adapters (`KeyValueStore<T>`): zod schema,
default value, **schema version**, namespaced keys (`<slice>:<name>`), and
size accounting. Rules:

* **Single writer**: mutate durable state in the service worker. If another
  context must write, it asks the SW.
* **Atomic read-modify-write** under `navigator.locks.request(key, …)`.
* **Never store secrets in `storage.local`** (not encrypted at rest by the
  extension platform). Access tokens: `storage.session`; long-lived credentials:
  avoid on-device; if unavoidable, wrap with WebCrypto using a non-extractable
  key held in IndexedDB and accept the residual risk in the threat model.
* **Quota guard**: stores check size before write; eviction policy for caches;
  alerts via telemetry when usage exceeds a threshold.
* **Retention**: clear user data on sign-out, tenant switch and when policy
  requires; `runtime.setUninstallURL` for off-boarding surveys/cleanup guidance.

### 7.2 Migrations

* Each store has `version` and a list of pure migration functions
  `(vN) → (vN+1)`.
* Run in `runtime.onInstalled` (`reason: update`) **and** lazily on first read
  (belt and braces: the SW may start without `onInstalled`).
* Migrations are idempotent, tested against **fixtures of every released
  version**, and never delete data without a backup key for one release cycle.
* Downgrades are unsupported: store a `minReaderVersion` to avoid old code
  misreading new data.

### 7.3 Policy-over-user precedence

Effective settings = `managed policy` > `user settings (local/sync)` >
`defaults`. A pure `PolicyResolver` in the application layer computes the
effective configuration and flags which user controls are locked. UIs render
locked controls read-only with a "managed by your organization" notice.

---

## 8. Service worker engineering

### 8.1 Entry file contract

```text
1. Synchronously, at top level: build container (cheap), register ALL listeners.
2. Listener bodies: await container deps lazily, rebuild state from storage, run use case.
3. Never start unawaited long work; schedule via alarms/queue.
```

### 8.2 Lifecycle events

| Event | Responsibilities |
|---|---|
| `runtime.onInstalled` (`install`, `update`) | Run migrations; create alarms and context menus (they persist across restarts but must be created once); set uninstall URL; first-run onboarding; inject into already-open matching tabs (`scripting.executeScript`); apply policy |
| `runtime.onStartup` | Reconcile queues, refresh policy and session projections |
| `runtime.onUpdateAvailable` | Defer reload until idle; enterprise: honor admin update window |
| `storage.onChanged` (managed) | Recompute effective policy, notify UIs and tabs, apply kill switches |
| `alarms.onAlarm` | Drain job queue, refresh tokens, poll config, flush telemetry |
| `webNavigation.*`, `tabs.*` | Narrowly filtered; feed content-script lifecycle (SPA route changes) |
| `commands`, `contextMenus`, `action` | Thin handlers calling use cases |

### 8.3 Jobs, queues and alarms

* Persisted **job queue** (storage/IndexedDB): `{id, type, payload, attempts,
  nextRunAt, idempotencyKey}`; handlers are idempotent; exponential backoff with
  jitter; dead-letter store with admin-visible counts.
* A single alarm (e.g., every minute or the minimum allowed) drains due jobs
  within a **time budget** (e.g., 20 s) and re-arms; heavier work goes to the
  backend.
* **Resumability**: long flows checkpoint after each step; on cold start the
  queue picks up where it left off.
* **Concurrency control**: one drain at a time (lock); per-job-type caps.
* **Offline**: failures classified as retryable (network, 5xx, 429) vs
  terminal; queue survives browser restarts.

### 8.4 Offscreen documents

Use only for what the worker cannot do (DOM parsing, audio, clipboard, canvas)
with the precise `reason` and a clear justification string. Create on demand,
close when done, and treat as a stateless capability behind a port
(`HtmlParserPort`). Communicate through the typed RPC; they have a lifetime
independent of the worker. At most one may exist at a time **(verify)**; use a
dispatcher inside it for multiple tasks.

### 8.5 Resilience

* `self.addEventListener('unhandledrejection'|'error')` report to telemetry
  (redacted) without crashing silently.
* Circuit breaker around the API; fail closed for security decisions
  (unknown policy → most restrictive).
* **Extension-update orphaning**: after an update or reload, old content scripts
  lose their extension context (`Extension context invalidated`). The SW
  re-injects on `onInstalled`, and content scripts detect invalidation
  (`chrome.runtime.id` undefined / port disconnect) and **tear down cleanly**.

---

## 9. Content scripts and page integration

### 9.1 Injection strategy

| Strategy | Use when |
|---|---|
| Declarative (`content_scripts` in manifest) | Fixed, small, well-known host set |
| **Programmatic** (`scripting.executeScript`, `registerContentScripts`) | Hosts come from admin policy/tenant config; `activeTab` flows; need conditional injection |
| MAIN world | Only to read page-defined JS objects; keep to a minimal, audited shim that talks via `postMessage` |

Inject the smallest bundle possible; split by site family; lazy-load heavy UI
only after the user acts.

### 9.2 Structure of a content script

```text
content entry (composition root, <50 lines)
  ├── Guards: matches policy? kill switch? extension context valid?
  ├── Site adapters: detect page type, observe DOM, emit domain events
  ├── Rpc client: send validated events to the SW (never tokens, never secrets)
  └── UI mount (optional): Shadow DOM root + React, closed mode where possible
```

* **Site adapter pattern**: `SiteAdapter { id; matches(url); observe(ctx): Disposable }`.
  Selectors and heuristics live in versioned adapter modules; unit-tested with
  saved HTML fixtures. Page changes are the #1 maintenance cost: add telemetry
  for "adapter failed to match" to detect breakage early.
* **Remote selector config** is allowed as **data**, never code: JSON fetched
  by the SW, validated by schema, signed (or served from a trusted origin with
  integrity metadata), cached with TTL, with a bundled fallback.
* **SPA navigation**: use `webNavigation.onHistoryStateUpdated` (SW) or the
  Navigation API in the page to re-run adapters; debounce `MutationObserver`s;
  disconnect observers on `pagehide` and teardown.
* **Performance**: avoid layout thrash, cap observed subtree, no polling loops,
  budget per page (see 16).
* **Isolation**: UI in Shadow DOM; no global CSS; no `innerHTML` with page or
  extracted data; use `textContent`/safe templating; sanitize any HTML with an
  allowlist sanitizer; never evaluate page-provided strings.
* **Never trust the DOM**: text, attributes and URLs read from the page are
  attacker-controlled; validate and size-limit before sending to the SW.
* **Never expose secrets to the page**: no tokens, tenant keys, internal URLs
  in the content-script world, `window`, or injected UI.
* **CORS**: content-script network requests are subject to the page's
  restrictions; **route all API calls through the SW**.
* **Frames**: set `all_frames` only when needed; check `frameId` in messages.

---

## 10. UI surfaces (popup, side panel, options)

* **MVVM** exactly as in `PLAYBOOK_FRONTEND.md`: Views render; ViewModels hold
  state and commands; the SDK port is implemented by an RPC adapter that calls
  the SW. The same ViewModels run in web, mobile and the extension.
* **Routing**: in-memory router with the same paths as the web app
  (`/app/funnels/leads/:leadId`); `{userType}` never authorizes; guards shape
  the UI only; the backend decides.
* **Popup**: ephemeral; do not hold state; fetch projections on open; keep it
  under a few hundred ms to first paint.
* **Side panel**: persistent surface for workflows; use `sidePanel.setOptions`
  for per-tab or per-site panels and `setPanelBehavior` for open-on-click;
  `sidePanel.open()` needs a user gesture; handle tab switches by re-reading the
  active-tab projection.
* **Options page**: settings (only unlocked ones), policy status, account and
  tenant info, **diagnostics export**, version and channel info, links to the
  admin guide.
* **Statuses**: every ViewModel exposes `loading | ready | empty | forbidden |
  not-found | offline | error | policy-blocked`.
* **i18n**: all strings are message keys via `chrome.i18n` (`_locales`), no
  literals; plan for RTL and text expansion.
* **Accessibility**: keyboard-first, focus management on panel open/close,
  WCAG 2.2 AA, `prefers-reduced-motion`, high-contrast; axe checks in CI.
* **Theming**: follow `prefers-color-scheme`; tokens shared with web.
* **Error boundaries** per surface; show the `correlationId` and a
  "copy diagnostics" action.

---

## 11. Networking, authentication and SDK integration

### 11.1 SDK in the service worker

The SDK (`packages/sdk`) is created once in the SW container with ports:
`transport` (fetch with timeout/abort), `tokenStore` (`storage.session`),
`clock`. UIs and content scripts never import the SDK; they call RPC use cases.
Per the frontend playbook the SDK mirrors the URL contract
(`sdk.app.funnels.leads.get(id)` ↔ `GET /api/v1/app/funnels/leads/{id}`) and
adds `Authorization`, `X-Tenant-Id`, `X-Correlation-Id`, `Idempotency-Key`,
retry and error mapping.

### 11.2 Authentication

| Flow | Use |
|---|---|
| **OIDC with PKCE via `chrome.identity.launchWebAuthFlow`** | Default for enterprise SSO; redirect `https://<id>.chromiumapp.org/`; state/nonce validated; no embedded password forms |
| `chrome.identity.getAuthToken` | Google-account-only products |
| Sign-in through the web app | Extension receives a short-lived, audience-restricted exchange code through a verified channel |

Rules: access tokens in `storage.session` (trusted contexts only), short TTL,
refresh handled by the SW with single-flight; **prefer a backend that issues
extension-scoped, revocable tokens** over holding third-party refresh tokens
on the device; sign-out revokes server-side and clears all stores; tenant
switch purges caches; handle `401` by one refresh attempt then a `signed-out`
projection. Device/session binding and anomaly detection are server-side.

### 11.3 Network rules

* Only the SW calls the backend; every call has a timeout, abort signal,
  correlation ID and bounded retries on idempotent operations.
* API origins are constants from build config **and** validated against policy
  (`TenantUrl` must match an allowed pattern; reject plain HTTP and unexpected
  hosts).
* TLS errors are never bypassed. Respect corporate proxies and custom roots
  (the browser handles them; test behind a proxy).
* `declarativeNetRequest` rulesets (if any) are static, reviewed, and minimal;
  dynamic rules come only from validated policy/config.
* Never fetch and execute remote scripts; validate every remote JSON with
  schemas and size limits.
* Rate-limit client-side (queue) to respect backend quotas.

---

## 12. Enterprise capabilities

### 12.1 Admin configuration: `storage.managed`

Declare a **JSON Schema** (`managed-schema.json`) referenced by
`storage.managed_schema`. Chrome validates it strictly: the top-level must be an
object without `additionalProperties`, and each schema needs `$ref` or exactly
one `type`; if the schema is invalid Chrome won't load the extension, and values
that don't conform aren't published. Administrators set values through
policy (`3rdparty.extensions.<id>`); verify at `chrome://policy`. The extension
reads them with `chrome.storage.managed` and reacts to `storage.onChanged`.
It is the extension's job to **enforce** the policy.

Recommended policy surface:

| Policy | Type | Effect |
|---|---|---|
| `TenantUrl` / `ApiBaseUrl` | string | Backend endpoint (validated against allowlist) |
| `SsoMode`, `IdpIssuer`, `ClientId` | string | SSO configuration |
| `AllowedHosts` | string[] | Sites where content features run (drives dynamic injection and optional host requests) |
| `BlockedHosts` | string[] | Sites where nothing runs |
| `FeatureFlags` | object<string,bool> | Enable/disable features centrally |
| `KillSwitch` | boolean | Disable all page interaction and sync |
| `TelemetryLevel` | enum (`off|errors|full`) | Governs diagnostics and analytics |
| `LogLevel` | enum | Local log verbosity |
| `UpdateWindow` | object | When to apply pending updates |
| `ForceSignInOnStartup` | boolean | Session policy |
| `DataRetentionDays` | integer | Local cache retention |
| `AllowUserSettings` | boolean | Lock or unlock user-level preferences |
| `SupportContact` | string | Shown in diagnostics UI |

Schema and policy examples: Appendix B.

### 12.2 Installation and update policy

| Need | Mechanism |
|---|---|
| Force-install silently | `ExtensionInstallForcelist` (`<id>;<update_url>`) or `ExtensionSettings` with `installation_mode: force_installed` and `update_url` |
| Pin to toolbar | `ExtensionSettings.toolbar_pin` |
| Restrict permissions/hosts | `ExtensionSettings` (`blocked_permissions`, `runtime_blocked_hosts`, `runtime_allowed_hosts`) |
| Block other extensions | `ExtensionInstallBlocklist` / allowlist |
| Private distribution | Chrome Web Store **private/unlisted** (visible to a domain or group) or a **self-hosted update feed** (signed CRX + update manifest) |
| Staged rollout | Roll out the policy by organizational unit; use CWS staged publishing and trusted testers for canaries |
| Update timing | Honor browser auto-update; optionally defer reload via `onUpdateAvailable` + `UpdateWindow` |

Document for admins (in `docs/admin-guide.md`): required network allowlists
(Web Store/update endpoints, API origins, IdP), supported Chrome versions
(include Extended Stable), proxy/PAC and TLS inspection notes, example policy
for Windows (registry/GPO/ADMX), macOS (plist/MDM), Linux (managed policy
JSON), Google Admin console and Chrome Browser Cloud Management, how to
verify (`chrome://policy`, `chrome://extensions`), and troubleshooting.

### 12.3 Kill switch, remote config and incident control

* **Kill switch** from two sources: managed policy (`KillSwitch`) and signed
  remote config (data). When on: stop injection, clear in-memory state,
  disable network except config polling, show a banner.
* **Per-feature flags** and **per-tenant overrides** via remote config (data
  only, schema-validated, signed, with bundled safe defaults).
* **Minimum version enforcement**: backend can reject outdated clients with a
  typed `upgrade-required` error that the UI surfaces and the SW handles by
  checking for updates (`runtime.requestUpdateCheck`).
* **Rollback** = ship a higher version containing the revert (versions only go
  up) **plus** the kill switch for immediate containment.

### 12.4 Identity, device and multi-profile

* Support multiple Chrome profiles: state is per profile; never assume a
  single instance.
* Tie the extension session to the corporate identity (SSO), not the browser
  profile name.
* Incognito: decide per policy (`incognito` manifest key; default `not_allowed`
  for enterprise unless required).
* ChromeOS-only enterprise APIs (`chrome.enterprise.*`) are out of scope unless
  you target managed ChromeOS **(verify)**.

### 12.5 Supportability and audit

* **Diagnostics bundle** (options page, "Export diagnostics"): version, channel,
  manifest hash, permissions granted, policy snapshot (secrets redacted),
  effective config, storage usage, queue/DLQ stats, recent logs (ring buffer),
  last errors with correlation IDs. Generated locally, user-initiated.
* **Audit events** to the backend (if policy allows): sign-in/out, policy
  applied, feature toggled, data export, kill switch state, permission grants.
  Redacted, correlated, rate-limited.
* **Version and health reporting**: the SW reports version, channel, Chrome
  version and policy hash with each session refresh (so admins and support can
  see fleet state), subject to `TelemetryLevel`.
* **Admin documentation and runbooks** are release artifacts (section 18).

---

## 13. Security engineering

### 13.1 Threat model (extension-specific)

| Threat | Examples | Controls |
|---|---|---|
| **Hostile web page** | Spoofed messages, DOM tampering, stealing injected UI data, clickjacking | Sender validation, message permission table, Shadow DOM, validated bridge, no secrets in content world |
| **Compromised content script / renderer** | Calls privileged RPC | Content-script capability subset; SW enforces; sensitive actions need UI-confirmed user gesture in a trusted context |
| **XSS in extension pages** | Untrusted data rendered in popup/side panel | No `innerHTML`, sanitization, strict CSP, Trusted Types where practical, escape on render |
| **Token/session theft** | Malware, other extensions, devtools | `storage.session`, short TTL, server-side revocation, no tokens in logs |
| **Supply-chain compromise** | Malicious dependency, build tool, CI | Lockfile, pinned versions with release-age cooldown, SCA, provenance, reproducible build, signed artifacts |
| **Store account takeover / ownership transfer** | Malicious update pushed | 2-step verification, group publisher with least privilege, publish approval, monitoring of listing changes, kill switch |
| **Over-broad permissions** | Abuse if compromised | Permission register, optional permissions, admin host restrictions |
| **Malicious remote config** | Tampered selectors/flags | Signature verification, schema validation, bundled fallback, no executable content |
| **Update hijack (self-hosted)** | Tampered CRX or feed | Signed CRX, HTTPS update URL, pinned key, integrity checks, monitored feed |
| **Data exfiltration** | Capturing page data beyond purpose | Data minimization, allowlisted hosts, local redaction, purpose limitation, audit |
| **Malicious/insider admin config** | Pointing `TenantUrl` at attacker | Validate against allowlisted patterns baked into the build or signed config |
| **Side-channel via logs/telemetry** | PII leak | Redaction, `TelemetryLevel`, retention |

### 13.2 Controls checklist

* MV3 CSP with no `unsafe-eval`; no `eval`, `new Function`, string timers; CI
  bundle scanner fails on them (including in dependencies).
* **No remotely hosted code**; no dynamic script injection by URL.
* `chrome.runtime.onMessage` handlers all pass through the validation wrapper.
* Allowlist of origins for `externally_connectable` and bridges; no wildcards.
* HTML sanitization (DOMPurify-style allowlist) for any rich text; prefer text.
* No secrets in the bundle, manifest, source maps shipped to the store, or
  logs.
* Anti-tamper for configuration: validate `TenantUrl`/hosts against patterns.
* **Least-privilege APIs** per context: content scripts can't call
  privileged APIs by design (only `runtime.sendMessage` and a few others).
* Rate-limit and size-limit all inbound messages.
* Use `crypto.subtle`/`crypto.getRandomValues`, never `Math.random`, for
  security-relevant values.
* Dependency hygiene: minimal dependency count; review new packages (maintainer
  history, install scripts, size); disable lifecycle scripts by default;
  cooldown for fresh releases (e.g., 7–14 days) with a security-fix bypass;
  `npm audit`/OSV in CI; SBOM per release.
* Build integrity: CI builds from a clean checkout; the zip hash is recorded;
  artifacts signed/attested; the exact zip tested is the one published.

### 13.3 Secrets and keys

| Item | Handling |
|---|---|
| Manifest `key` private part / CRX signing key | KMS/HSM or offline vault; documented custody; rotation/recovery plan |
| Chrome Web Store API credentials | CI secret manager, scoped, rotated; publishing requires an approval gate |
| API client secrets | **None in the extension** (public client + PKCE) |
| Tenant provider keys | Never on the device |
| Sentry/telemetry DSN | Public-by-design but rate-limited and scoped; no PII |

### 13.4 Tooling gotchas under CSP

* Validators that compile code at runtime (some JSON-schema engines, template
  engines, `lodash.template`) violate CSP: use zod/valibot or precompiled
  standalone validators.
* Development builds may use `eval`-based source maps; production builds must
  not. Verify with a CSP-violation check in e2e.
* Third-party UI libraries that inject inline `<script>` or use
  `document.write` fail; vet them before adoption.

### 13.5 Security testing

Authz/permission tests of the message table (each type × each sender class);
fuzzing of message payloads and sender metadata; hostile-page e2e (spoofed
`postMessage`, DOM-injection, prototype pollution attempts); XSS payload suite
on all UIs and injected UI; CSP report/violation assertions; manifest
permission diff; dependency/bundle scans; periodic independent penetration test
of the extension and its backend integration.

---

## 14. Privacy and Chrome Web Store compliance

* **Single purpose**: the extension has one narrow, clearly described purpose;
  features outside it need a separate extension or strong justification.
* **Limited use / data minimization**: collect only what the feature needs;
  no selling data, no unrelated profiling; user-visible disclosure of what is
  collected, why and where it goes.
* **Privacy policy** linked in the listing and in the options page; the Web
  Store privacy/data-use declarations must match actual behavior (audit them
  each release).
* **Permission justifications** written for each permission in the listing
  (reuse the register).
* **No remote code**, no obfuscated code (minified is fine, obfuscation is not).
* **Consent and controls**: explicit onboarding explaining data access; admin
  policy can restrict (`TelemetryLevel`, `AllowedHosts`); user can pause
  features per site.
* **Data lifecycle**: retention limits, purge on sign-out/uninstall guidance,
  data-subject requests handled by the backend (see audit playbook).
* **Enterprise contracts**: processor role, sub-processor list, DPA; the
  extension reveals which hosts it reads and which backend it talks to in the
  admin guide.
* **Reviews and enforcement**: expect manual review for broad host
  permissions; keep a runbook for rejections, appeals and emergency
  takedowns; keep contact info current (publisher account).

---

## 15. Observability and diagnostics

| Signal | Implementation |
|---|---|
| **Logging** | `LoggerPort` with levels, structured JSON, redaction, correlation IDs shared with the backend; local **ring buffer** in IndexedDB (bounded); remote logging only per `TelemetryLevel` |
| **Errors** | Global handlers in every context; error reporter (e.g., Sentry-compatible) with PII scrubbing, release and channel tags, sourcemaps uploaded privately (not shipped in the store zip) |
| **Metrics** | SW cold-start time, event-to-response latency, RPC latency/error rate, queue depth/age/DLQ, content-script injection time and adapter match rate, storage usage, memory samples, update success |
| **Tracing** | `correlationId` generated at the edge of an action (UI click, content event) and propagated through RPC, SDK headers and logs |
| **Health** | `health` RPC and projection: policy valid, session valid, API reachable, adapters OK, versions |
| **Alerts** | Server-side alerts on spikes of `adapter-failed`, `rpc-error`, `upgrade-required`, update failures per version |
| **Privacy** | No page content or URLs in telemetry unless essential and allowed; hash/ truncate hosts; admin can set `off` |

---

## 16. Performance and reliability budgets

Set budgets in config and fail CI on regression (use measured baselines):

| Area | Budget (starting points) |
|---|---|
| SW bundle size | Small enough for cold start of a few tens of ms of parse; track KB per release |
| SW cold start to first handled event | Track p50/p95; investigate regressions |
| Content script size (per site family) | Keep minimal (tens of KB gzip); lazy-load UI |
| Injection cost on page | Negligible main-thread time; no layout thrash; observers throttled |
| Popup time to interactive | Under a few hundred ms |
| Memory | Stable over a 24-hour soak; no growth from observers/ports |
| Storage | Bounded; alerts at thresholds |
| RPC round trip | p95 tracked per message type |
| Battery/CPU | No background polling beyond alarms; idle SW |

Reliability: handle SW restarts at any point, orphaned content scripts after
updates, offline mode, API outages (degraded UI with cached projections),
quota exhaustion, corrupted storage (reset path with user/admin message),
policy changes mid-flow, and multi-profile/multi-window usage.

---

## 17. Testing strategy

Follows `PLAYBOOK_TESTS.md` with extension specifics.

| Level | Scope | Tooling (examples) |
|---|---|---|
| Unit | domain, application use cases with fake ports; pure functions (policy resolver, migrations, matchers) | Vitest/Jest |
| ViewModel | states and commands with fake RPC | `renderHook` |
| Platform adapters | `chrome.*` wrappers against a fake browser | `@webext-core/fake-browser` or hand fakes |
| Contract | message protocol (both ends use the same schemas), SDK vs OpenAPI, managed schema vs examples, storage schema versions | schema tests, MSW |
| Component / a11y | Views per status; axe | RTL, axe |
| **Integration** | SW + storage + queue with fake timers; migration from every released fixture | Vitest |
| **E2E (real Chrome)** | Load the built extension in a persistent context with the extension flags; drive popup/side panel/options; hostile test pages; content scripts on fixture sites; OAuth against a mock IdP | Playwright (extension-capable, new headless mode **(verify)**) |
| **SW lifecycle** | Terminate the worker mid-flow (via devtools protocol or restart) and assert resumption | Playwright + CDP |
| **Policy** | Launch Chrome with a managed policy file; assert `storage.managed` values, locked UI, kill switch, host restrictions, invalid values ignored | Playwright + OS policy directory |
| Security | message permission table, fuzzing, XSS payloads, CSP violations, bundle scan for `eval`, manifest permission diff | Custom + Semgrep |
| Performance | cold start, injection cost, memory soak | Playwright traces, custom harness |
| Upgrade | Install vN-1, populate data, update to vN, verify migrations and orphan handling | Scripted e2e |
| Cross-version | Min supported Chrome, Stable, Beta; Edge | CI matrix |
| Store readiness | Manifest lint, listing assets, privacy declarations check | Script + checklist |

Rules: tests never use real customer data or real IdP/production endpoints;
fixtures for host pages are static HTML snapshots; e2e keeps to critical
journeys (sign-in, capture, admin policy, kill switch, update); every message
type has a contract test and a sender-class permission test.

---

## 18. Build, release and operations

### 18.1 Tooling and configuration

* Package manager with lockfile and frozen installs; Node LTS; TypeScript
  strict; ESLint (type-aware) + dependency-cruiser; Prettier; Vitest;
  Playwright.
* Build per channel: `dev` (local), `staging` (internal), `canary`
  (pilot group), `stable` (all). Each has its own extension ID, name suffix,
  icon badge, API base URL and telemetry project.
* `wxt.config.ts` (or equivalent) generates the manifest from typed config;
  **manifest and permission snapshots** are committed and diffed in PRs.
* Source maps: generated, uploaded to the error tracker privately, excluded
  from the published zip.

### 18.2 CI pipeline

| Stage | Gate |
|---|---|
| Install | Frozen lockfile; lifecycle scripts disabled by default; registry allowlist |
| Static | Typecheck, lint, architecture rules, `chrome` global restriction, forbidden-API scan (`eval`, remote scripts) |
| Unit/integration/contract | Must pass; coverage floor on domain/application |
| Build | Reproducible build; record zip hash; SBOM |
| Manifest checks | Permission/host diff vs. register; CSP check; `minimum_chrome_version`; locale completeness |
| Security | SAST, SCA, secrets scan, bundle scan |
| E2E | Real Chrome (min + stable), policy and upgrade suites on release branches |
| Package | Zip (store) and optional signed CRX (self-hosted); attestation |
| Publish | **Manual approval gate**, then upload via the Chrome Web Store API to the target channel (canary/trusted testers → stable with staged percentage) |
| Post-release | Monitor error rate, adapter match rate, update adoption; auto-alert to rollback criteria |

### 18.3 Versioning

* Semantic versioning mapped to Chrome's numeric `version` (for example
  `MAJOR.MINOR.PATCH.BUILD`); `version_name` carries the human/prerelease
  label.
* Storage migrations and protocol versions are versioned independently.
* A changelog (user-facing and admin-facing: permission changes, policy
  additions, deprecations) ships with every release.

### 18.4 Rollout and rollback

1. Internal channel (staging ID) for every merge to main.
2. **Canary** to a pilot group (trusted testers / pilot OU) for a fixed soak.
3. **Staged** public/enterprise rollout in increasing percentages with
   automated health checks.
4. **Rollback**: publish a fixed higher version; use the **kill switch** or
   feature flags for immediate containment; for self-hosted feeds, point the
   update manifest to the fixed build.
5. Permission increases are batched, announced in the admin changelog ahead of
   time, and never bundled with urgent fixes.

### 18.5 Environments and data

Staging and canary never use production tenant data; test tenants are
synthetic. Production synthetic monitoring uses a dedicated test tenant with
read-only operations.

### 18.6 Runbooks (required)

Store rejection/takedown and appeal · compromised publisher account · leaked
signing key or manifest key · malicious dependency discovered · broken release
(rollback + kill switch) · adapter breakage after a target site change · policy
misconfiguration at a customer · mass sign-out/IdP outage · data incident
(privacy) · Chrome version regression.

---

## 19. Cross-browser strategy

* **Edge** (Chromium) works with the same package; publish to Edge Add-ons if
  required; enterprise force-install uses Edge's own policy hives.
* **Firefox/Safari**: use the build adapter (WXT) and keep browser differences
  behind `platform/` ports (`browser` vs `chrome` namespaces, event-page vs
  service-worker background, `sidePanel` vs `sidebar_action`, missing APIs).
  Treat as separate release channels with their own tests.
* Keep a **capability matrix** (feature → required API → supported browsers)
  and degrade gracefully where an API is absent.

---

## 20. Governance and compliance

* **Ownership**: CODEOWNERS for `manifest/config`, `managed-schema.json`,
  `src/security/**`, `src/platform/**`, `docs/permissions-register.md`,
  release workflows; security review required for permission or host changes.
* **Documents** (versioned): ADRs, permission register, data-flow diagrams,
  threat model, privacy review, admin guide, runbooks, release checklist.
* **Compliance mapping** (SOC 2 / ISO 27001 / ISO 9001, per
  `PLAYBOOK_AUDIT.md`): secure SDLC (CI gates, reviews), change management
  (release approvals, permission diffs), access control (publisher accounts,
  keys), logging/monitoring (telemetry, audit events), vendor management
  (store, error tracker), privacy (declarations, retention), availability
  (kill switch, staged rollout), evidence (CI artifacts, SBOM, release bundle).
* **Release evidence bundle** per version: zip hash, SBOM, test reports,
  permission diff, security scan results, approvals, rollout log.
* **Periodic**: quarterly permission-register review, annual pen test, DR drill
  for key/account loss, dependency refresh cadence, store-policy review after
  each policy update.

---

## 21. Working with Claude Code on this project

Per `PLAYBOOK_CLAUDE.md`:

* A **path-scoped rule** for `apps/extension/**` carrying the layer rules, the
  messaging/sender-validation rule, the "no `chrome.*` outside `platform/`" and
  "no remote code" rules, and links to this playbook (not `@`-imported).
* **Role policy**: only the integrator edits `wxt.config.ts`, manifest
  snapshots, `managed-schema.json`, lockfiles and release workflows; the
  security reviewer agent checks permission changes; the control plane (hooks,
  settings) stays human-owned.
* **Skills**: `new-extension-feature` (scaffold slice with ports, RPC types,
  ViewModel, tests), `new-message-type` (schema + sender-class entry +
  handler + contract test), `new-site-adapter` (adapter + HTML fixtures +
  tests), `permission-change` (register entry + justification + test).
* **Specs** for every feature include: contexts involved, messages added,
  permissions touched, policy keys, data collected, and test levels per AC.

---

## 22. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| State in service-worker globals | Lost on termination |
| `setTimeout`/`setInterval` for scheduling | Worker dies; use alarms/queue |
| Keep-alive pings to prevent termination | Fragile, discouraged; redesign |
| Calling the API from content scripts | CORS, token exposure; route via SW |
| Tokens/secrets in content world or `storage.local` | Page/malware exposure |
| `<all_urls>` and broad `matches` by default | Review friction, risk, admin rejection |
| Trusting `window.postMessage`/`sendMessage` without sender checks | Page-driven privilege escalation |
| `innerHTML` with extracted data | XSS |
| `eval`/`new Function`/remote scripts/obfuscation | CSP + store policy violations |
| Business logic in React components or content scripts | Untestable, duplicated |
| Hand-edited manifest, no permission diff | Silent privilege creep |
| Listeners registered after `await` | Missed wake-up events |
| One giant bundle for all pages | Slow cold start and injection |
| Policy ignored in UI | Admin controls bypassed |
| No migration tests | Data loss on update |
| Shipping source maps/debug code to the store | Leak and size |
| Shared extension ID across channels | Cross-environment contamination |
| Silent catch-all error handling | Invisible breakage |

---

## 23. Definition of done and checklists

### 23.1 Feature DoD

- [ ] Spec lists contexts, messages, permissions, policy keys, data, test levels.
- [ ] Domain/application tested with fake ports; ViewModel statuses covered.
- [ ] New message types have schema, sender-class permission entry, handler, contract test.
- [ ] No new permission, or register entry + security review + justification text.
- [ ] Works after SW termination at any step; idempotent handlers.
- [ ] Respects policy (locked controls, kill switch, allowed/blocked hosts).
- [ ] Telemetry: correlation ID, errors, metrics, redaction verified.
- [ ] i18n keys, a11y checks, performance budget respected.
- [ ] Storage changes have schema version + migration + fixture.

### 23.2 Release checklist

- [ ] Reproducible build; zip hash recorded; SBOM and scans green.
- [ ] Manifest/permission diff reviewed; register updated; admin changelog written.
- [ ] Unit, integration, contract, e2e (min + stable Chrome), policy, upgrade suites green.
- [ ] Privacy declarations and listing text match behavior.
- [ ] Canary soak complete; health metrics within thresholds.
- [ ] Approval gate passed; staged rollout configured; rollback and kill switch verified.
- [ ] Runbooks and admin guide current.

### 23.3 Architecture acceptance checklist

- [ ] MV3, TypeScript strict, generated manifest per channel with stable IDs.
- [ ] Layers enforced by tooling; `chrome` only in `platform/`.
- [ ] SW is the sole owner of session, SDK, durable writes and queue.
- [ ] All listeners top-level and synchronous; no durable globals.
- [ ] Typed, validated messaging with sender validation and permission table.
- [ ] Typed storage with versions, migrations, locks, quota guards.
- [ ] Content scripts minimal, isolated, adapter-based, no secrets, orphan-safe.
- [ ] UIs follow MVVM + SDK port via RPC; routes mirror the URL playbook.
- [ ] Managed schema, policy resolver, kill switch, remote config (data only), diagnostics export.
- [ ] CSP clean; no remote/eval code; dependency and supply-chain controls in CI.
- [ ] Observability: logs, metrics, errors, correlation IDs, health.
- [ ] Test pyramid including SW lifecycle, policy, upgrade, hostile-page suites.
- [ ] Release pipeline with approval gate, staged rollout, rollback, runbooks.
- [ ] Permission register, ADRs, threat model, privacy review, admin guide current.

---

## Appendix A — Manifest template (generated; shown for review)

```json
{
  "manifest_version": 3,
  "name": "__MSG_extName__",
  "description": "__MSG_extDescription__",
  "default_locale": "en",
  "version": "1.0.0.0",
  "version_name": "1.0.0",
  "minimum_chrome_version": "120",
  "key": "<public key per channel>",
  "incognito": "not_allowed",
  "background": { "service_worker": "background.js", "type": "module" },
  "action": { "default_title": "__MSG_actionTitle__", "default_popup": "popup.html" },
  "side_panel": { "default_path": "sidepanel.html" },
  "options_ui": { "page": "options.html", "open_in_tab": false },
  "permissions": ["storage", "alarms", "sidePanel", "scripting", "identity"],
  "optional_permissions": ["notifications", "downloads"],
  "host_permissions": ["https://api.<product>.example/*"],
  "optional_host_permissions": ["https://*/*"],
  "content_scripts": [
    {
      "matches": ["https://app.<site>.example/*"],
      "js": ["content-scripts/site.js"],
      "run_at": "document_idle",
      "all_frames": false,
      "world": "ISOLATED"
    }
  ],
  "storage": { "managed_schema": "managed-schema.json" },
  "content_security_policy": {
    "extension_pages": "script-src 'self'; object-src 'self'; base-uri 'none'"
  },
  "web_accessible_resources": [],
  "externally_connectable": { "matches": ["https://app.<product>.example/*"] }
}
```

## Appendix B — Managed schema and policy examples

**`managed-schema.json`** (constraints: object at top level, no
`additionalProperties`, one `type` per property)

```json
{
  "type": "object",
  "properties": {
    "TenantUrl": { "title": "Tenant URL", "description": "Base URL of your organization's tenant.", "type": "string" },
    "AllowedHosts": { "title": "Allowed sites", "type": "array", "items": { "type": "string" } },
    "BlockedHosts": { "title": "Blocked sites", "type": "array", "items": { "type": "string" } },
    "FeatureFlags": { "title": "Feature flags", "type": "object", "additionalProperties": { "type": "boolean" } },
    "KillSwitch": { "title": "Disable the extension's page features", "type": "boolean" },
    "TelemetryLevel": { "title": "Telemetry", "type": "string", "enum": ["off", "errors", "full"] },
    "DataRetentionDays": { "title": "Local data retention (days)", "type": "integer", "minimum": 0 },
    "AllowUserSettings": { "title": "Let users change preferences", "type": "boolean" }
  }
}
```

**Policy delivery (Linux managed JSON shown; equivalent in registry/plist/Admin console)**

```json
{
  "ExtensionSettings": {
    "<extension-id>": {
      "installation_mode": "force_installed",
      "update_url": "https://clients2.google.com/service/update2/crx",
      "toolbar_pin": "force_pinned"
    }
  },
  "3rdparty": {
    "extensions": {
      "<extension-id>": {
        "TenantUrl": "https://acme.<product>.example",
        "AllowedHosts": ["https://crm.acme.example/*"],
        "TelemetryLevel": "errors",
        "FeatureFlags": { "leadCapture": true, "bulkExport": false }
      }
    }
  }
}
```

## Appendix C — Typed messaging contract (sketch)

```ts
// src/contracts/protocol.ts
import { z } from 'zod';

export const Envelope = z.object({
  v: z.literal(1),
  id: z.string(),
  correlationId: z.string(),
  type: z.string(),
  payload: z.unknown(),
});

export const protocol = {
  'session.get':  { req: z.object({}).strict(), res: SessionSummary, senders: ['ui'] },
  'lead.capture': { req: LeadCaptureInput,       res: LeadCaptureResult, senders: ['ui', 'content'] },
  'policy.get':   { req: z.object({}).strict(), res: EffectivePolicySummary, senders: ['ui', 'content'] },
} as const;

export type Protocol = typeof protocol;
export type Req<K extends keyof Protocol> = z.infer<Protocol[K]['req']>;
export type Res<K extends keyof Protocol> = z.infer<Protocol[K]['res']>;
```

```ts
// src/messaging/server.ts (service worker; platform wrapper hides chrome.*)
export function registerRpc(container: Container, rt: RuntimePort) {
  rt.onMessage(async (raw, sender) => {
    const env = Envelope.safeParse(raw);
    if (!env.success) return err('bad-envelope');
    const def = protocol[env.data.type as keyof Protocol];
    if (!def) return err('unknown-type');
    const klass = classifySender(sender, rt.id());              // 'ui' | 'content' | 'external' | 'unknown'
    if (!(def.senders as readonly string[]).includes(klass)) return err('forbidden-sender');
    const parsed = def.req.safeParse(env.data.payload);
    if (!parsed.success) return err('validation');
    return handlers[env.data.type as keyof Protocol](container, parsed.data, ctxFrom(env.data, sender));
  });
}
```

## Appendix D — Typed store with version, schema and lock (sketch)

```ts
export function createStore<T>(opts: {
  area: StorageAreaPort; key: string; version: number;
  schema: z.ZodType<T>; fallback: T; migrate?: Record<number, (v: unknown) => unknown>;
}): KeyValueStore<T> {
  const read = async () => { /* load {v, data}; run migrations v→v+1; parse; fallback on failure */ };
  return {
    get: read,
    update: (fn) => locks.run(opts.key, async () => {
      const next = fn(await read());
      await opts.area.set(opts.key, { v: opts.version, data: opts.schema.parse(next) });
      return next;
    }),
  };
}
```

## Appendix E — Service worker entry and content-script entry (sketch)

```ts
// entrypoints/background.ts
export default defineBackground(() => {
  const container = buildContainer();           // cheap: wires ports → adapters lazily
  registerRpc(container, platform.runtime);     // top-level, synchronous
  registerLifecycle(container);                 // onInstalled / onStartup / onUpdateAvailable
  registerAlarms(container);                    // drain queue, refresh session/policy
  registerNavigation(container);                // narrow filters
  registerPolicyWatcher(container);             // storage.onChanged(managed)
});
```

```ts
// entrypoints/content/site.content.ts
export default defineContentScript({
  matches: ['https://app.<site>.example/*'],
  runAt: 'document_idle',
  async main(ctx) {
    if (!(await guards.allowed())) return;           // policy, kill switch, context valid
    const rpc = createRpcClient(platform.runtime);
    const adapter = siteAdapters.select(new URL(location.href));
    const stop = adapter?.observe({ emit: (e) => rpc.call('lead.capture', toDto(e)) });
    ctx.onInvalidated(() => stop?.());               // extension updated/unloaded: clean teardown
  },
});
```

## Appendix F — Architecture rules (dependency-cruiser sketch)

```text
forbidden:
  - domain/** → anything except shared-kernel
  - application/** → adapters/**, platform/**, chrome (global), react
  - viewmodels/** → react-dom, chrome, platform/**, adapters/**
  - ui/** (views) → sdk, platform/**, storage/**
  - features/<a>/** → features/<b>/** except via <b>/contracts
  - content entrypoints → sdk, storage (tokens), platform/identity
eslint:
  - no-restricted-globals: chrome, browser (allowed only in src/platform/**)
  - no-restricted-syntax: eval, new Function, setTimeout(string), innerHTML assignments
```

## Appendix G — Permissions register entry

```markdown
### scripting
- Why: inject the capture UI into customer-approved CRM pages programmatically.
- Feature(s): lead capture (F-012), side-panel assist (F-020).
- Data touched: visible text of the active page region, only after user action.
- Alternatives considered: static content_scripts (rejected: host list is tenant-specific).
- Host access: optional_host_permissions + admin AllowedHosts.
- Owner: <name>   Security reviewer: <name>   Last review: <date>   Next: <date>
- Store justification text: "<copy for listing>"
- Tests: e2e/hostile-page, permission-diff gate.
```

## Appendix H — Diagnostics bundle specification

```text
diagnostics-<version>-<timestamp>.json (user-initiated export, redacted)
 - extension: version, channel, manifest hash, id, chrome version, platform
 - permissions: granted (required/optional), host permissions granted
 - policy: managed snapshot (values validated, secrets removed), effective config, locked controls
 - state: session summary (no tokens), tenant id, queue depth/age/DLQ counts, storage usage
 - health: API reachability, last config fetch, adapter match stats, last 50 error events with correlation IDs
 - logs: ring-buffer tail (redacted), limited to N KB
```
