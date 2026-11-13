# PLAYBOOK_FRONTEND.md — Frontend architecture playbook

Architecture contract for every Heimdall client: **React (web SPA)**,
**React Native (mobile)** and **Chrome extension (MV3)**. All three use
**MVVM + a shared SDK**, and all three adapt to the route contract in
`PLAYBOOK_URLS.md`. Companion of `PLAYBOOK_BACKEND.md` (slices, permissions,
error envelope).

The runnable, machine-checkable companion is
[beautifulreact](https://github.com/Facta-Dev/beautifulreact): three Agent
Skills (write, review, port) and a drop-in eslint config that enforces this
contract.

The rules that carry the whole file:

1. **View → ViewModel → SDK.** A View renders and forwards events. A
   ViewModel holds presentation logic and state. The SDK is the only thing
   that talks to the backend.
2. **The SDK mirrors the URL playbook.** `sdk.{userType}.{module}.{resource}`
   maps 1:1 to `/api/v1/{userType}/{module}/{resource}`.
3. **ViewModels and the SDK are platform-agnostic and shared.** Views are
   platform-specific. Platform differences (storage, navigation, messaging)
   enter through ports.
4. **The `{userType}` segment is an experience namespace, never security.**
   The frontend uses permissions only to shape the UI; the backend decides.
5. **One feature = one backend slice.** Frontend feature folder, URL module,
   SDK module and permission prefix share the same name.

## 1. Architecture

```text
┌──────────────────────────── client (web | native | extension) ────────────────────────────┐
│  View (platform UI)  ──►  ViewModel (hook, shared)  ──►  SDK (shared)  ──►  Backend API   │
│        ▲                        │                            │                            │
│        └──── state + commands ──┘                            └── ports: TokenStore,       │
│                                  └── ports: Navigator,           Transport, Clock         │
│                                      Notifier, Permissions                                │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Responsibility | Knows about | Must not know about |
|---|---|---|---|
| **View** | Render JSX/native components, forward user events, layout, styling, a11y | Its ViewModel's output, UI kit, platform APIs for rendering | SDK, HTTP, tokens, business rules, router internals |
| **ViewModel** | Presentation state, derived values, form state, validation messages, commands (`save`, `launch`), orchestration of SDK calls, loading/error states | SDK, ports (`Navigator`, `Notifier`), shared domain types | JSX, DOM, `react-dom`, `react-native`, `chrome.*`, router/library imports |
| **Model / SDK** | Typed API client, DTOs, error normalization, auth/tenant headers, pagination, retries | HTTP contract, ports (`TokenStore`, `Transport`) | React, UI, navigation, any platform global |

### 1.1 Dependency rules (enforced)

| From | May import | Must not import |
|---|---|---|
| `views` | own feature's `viewmodels`, `ui-*` kit, platform libs | `sdk` (except types re-exported by the ViewModel), other features' internals |
| `viewmodels` | `sdk`, `core` (ports, types), `react` (hooks only) | `react-dom`, `react-native`, `chrome`, router libs, `views` |
| `sdk` | `core` ports and types | `react`, any UI, any platform API |
| `core` | nothing | everything else |

Enforce with `dependency-cruiser` or ESLint `no-restricted-imports` in CI. A
violation fails the build.

## 2. Monorepo layout

```text
packages/
  sdk/                    # typed API client (platform-agnostic, no React)
    src/
      transport/          # HttpClient, interceptors, retry, error mapping
      resources/
        app/              # sdk.app.funnels.leads, sdk.app.projects ...
        admin/            # sdk.admin.tenants.users, sdk.admin.billing ...
        partner/
        public/
      session/            # session state, token refresh, active tenant
      types/              # generated from the OpenAPI contract
      index.ts
  core/                   # ports + shared pure TS: Navigator, Notifier, TokenStore,
                          # Permission helpers, route builders, error types
  viewmodels/             # shared ViewModels, one folder per feature
    funnels/
    projects/
    tenants/
    ...
  ui-web/                 # React DOM components (design system, web)
  ui-native/              # React Native components (design system, native)
  tokens/                 # design tokens shared by both UI kits
apps/
  web/                    # React SPA
  mobile/                 # React Native app
  extension/              # Chrome MV3 extension
```

Feature folders inside each app follow the backend slice names:

```text
apps/web/src/features/funnels/
  views/                  # LeadDetailView, LeadListView
  routes/                 # route definitions and guards for this module
apps/web/src/layouts/     # AppLayout, AdminLayout, PartnerLayout
apps/web/src/routes/      # route tree that mirrors PLAYBOOK_URLS
```

`packages/viewmodels/funnels` is the ViewModel half of the same feature, shared
by web, native and extension.

## 3. The SDK

### 3.1 Shape mirrors the URL playbook

```text
GET  /api/v1/app/funnels/leads/{leadId}
  → sdk.app.funnels.leads.get(leadId)

GET  /api/v1/admin/tenants/{tenantId}/users
  → sdk.admin.tenants.users.list(tenantId, { cursor, limit })

POST /api/v1/admin/tenants/{tenantId}/users/{userId}/suspend
  → sdk.admin.tenants.users.suspend(tenantId, userId)

POST /api/v1/app/projects/{projectId}/launch
  → sdk.app.projects.launch(projectId, input)
```

Rules:

- The SDK namespace is `{userType}.{module}.{resource}`; resource methods are
  `list`, `get`, `create`, `update`, `delete`, plus verb methods for state
  transitions (`suspend`, `launch`).
- The `{tenantId}` argument is explicit on admin and multi-tenant routes
  (first parameter). On `/app` routes the active tenant comes from the session
  (see 3.3).
- A path is built in exactly one place (the resource definition). Views and
  ViewModels never concatenate URLs.
- Types are **generated from the backend OpenAPI contract**; hand-written
  DTOs are forbidden. A contract change produces a typed compile error in
  clients, not a runtime surprise.
- Versioning: the SDK targets one API version (`/api/v1`). A breaking API
  change creates `v2` resources in the SDK alongside `v1` during the
  deprecation window. SDK package follows semver.

### 3.2 Transport and interceptors

One `HttpClient` built on `fetch` (available on web, React Native, and the
extension service worker). Interceptors, in order:

1. **Auth**: attaches `Authorization: Bearer <token>` from `TokenStore`;
   on `401` performs a single-flight refresh and replays the request once;
   if refresh fails, emits `session-expired`.
2. **Tenant**: attaches `X-Tenant-Id` from the active session tenant.
3. **Correlation**: attaches `X-Correlation-Id` (generated per call).
4. **Idempotency**: adds `Idempotency-Key` on mutating calls marked
   retryable.
5. **Retry**: exponential backoff with jitter for network errors, `429`, `5xx`
   on idempotent calls only.
6. **Error mapping**: converts the backend error envelope into a typed
   `SdkError`.

Every call accepts an `AbortSignal` so ViewModels can cancel on unmount.

### 3.3 Session and active tenant

- `Session` holds `user`, `activeRole`, `activeTenantId`, `permissions[]`
  (`module.resource.action`) and token state. It is the single source for the
  UI's notion of "who am I".
- Switching organization or role updates the session, clears the query cache
  and re-fetches permissions.
- The SDK never infers rights from the route. It only surfaces what the
  backend returns.

### 3.4 Errors

```ts
type SdkError =
  | { kind: 'unauthenticated' }                                  // 401
  | { kind: 'forbidden'; requiredPermission?: string }           // 403 (FORBIDDEN)
  | { kind: 'not-found' }                                        // 404
  | { kind: 'conflict' }                                         // 409
  | { kind: 'validation'; fields: Record<string, string> }       // 400/422
  | { kind: 'rate-limited'; retryAfterMs?: number }              // 429
  | { kind: 'network' }                                          // offline, timeout
  | { kind: 'server'; correlationId?: string };                  // 5xx
```

ViewModels translate `SdkError` into view state; they never inspect raw HTTP.

### 3.5 Platform ports

The SDK stays platform-agnostic by receiving ports at construction:

```ts
const sdk = createSdk({
  baseUrl,
  transport: fetchTransport,          // or an extension messaging transport
  tokenStore,                         // platform adapter
  clock,
});
```

| Platform | `TokenStore` | Notes |
|---|---|---|
| Web | In-memory access token + refresh via `httpOnly`, `Secure`, `SameSite` cookie | Never `localStorage` for refresh tokens |
| React Native | Keychain / Keystore (secure storage) | Biometric unlock optional |
| Extension | `chrome.storage.session` (access) and `chrome.storage.local` encrypted or cookie-based refresh | Held only by the service worker |

## 4. MVVM conventions

### 4.1 ViewModel contract

A ViewModel is a hook that returns **state** and **commands**, with no JSX:

```ts
// packages/viewmodels/funnels/useLeadDetailViewModel.ts
export function useLeadDetailViewModel(leadId: string) {
  const { sdk, navigator, notifier, permissions } = useServices();

  const lead = useQuery({
    queryKey: ['app', 'funnels', 'leads', leadId],
    queryFn: ({ signal }) => sdk.app.funnels.leads.get(leadId, { signal }),
  });

  const move = useMutation({
    mutationFn: (stage: Stage) =>
      sdk.app.funnels.leads.update(leadId, { stage }),
    onSuccess: () => notifier.success('lead.moved'),
  });

  return {
    status: toViewStatus(lead),                       // 'loading' | 'ready' | 'forbidden' | 'not-found' | 'error'
    lead: lead.data && toLeadVM(lead.data),           // presentation-ready data
    canEdit: permissions.can('funnels.leads.update'), // UI affordance only
    moveToStage: move.mutate,
    isMoving: move.isPending,
    openCampaign: () => navigator.go(routes.app.campaign(lead.data!.campaignId)),
  };
}
```

Rules:

- Returns plain data and functions. The View never receives raw SDK objects
  when a presentation shape is needed (use a mapper such as `toLeadVM`).
- Every ViewModel exposes an explicit **status**: `loading`, `ready`, `empty`,
  `forbidden`, `not-found`, `error`, `offline`. Views render all of them.
- One ViewModel per screen or per significant component; not one global store.
- Side effects (navigation, toasts, analytics, clipboard, share) go through
  **ports** (`Navigator`, `Notifier`, `Analytics`). That is what makes the
  ViewModel identical on web, native and extension.
- Services come from a React context (`useServices`), provided at the app
  root of each platform. ViewModels do not import singletons.

### 4.2 View contract

- A View calls one ViewModel hook and renders. No `fetch`, no SDK import, no
  business conditions beyond "show this status".
- Views are **not shared** between DOM and native. They share ViewModels, SDK,
  tokens and test fixtures. If the team later adopts a cross-platform view
  layer (for example `react-native-web`), it changes `ui-*` only, never the
  layers below.
- Components are split into **screen views** (bound to a ViewModel) and
  **presentational components** (props only, in `ui-web`/`ui-native`).

### 4.3 State management

| State | Where |
|---|---|
| Server state (lists, details) | Query cache (TanStack Query or equivalent) inside ViewModels, keyed by `[userType, module, resource, tenantId?, params]` |
| Session | `Session` object from the SDK, exposed through a hook |
| Filters, sort, pagination, selected tab | **URL query string** on web and extension pages; navigation params on native |
| Form state | ViewModel (form library inside the hook) |
| Ephemeral UI state (open/closed) | Local component state in the View |
| Global client store | Avoid; add only for a proven cross-screen need |

Query-key convention mirrors the URL, so invalidation is predictable:
`['app','funnels','leads']` invalidates every lead query for the end user;
`['admin','tenants',tenantId,'users']` invalidates that tenant's admin view.

## 5. Adapting to the URL playbook

### 5.1 Web SPA routes

The route tree is the frontend half of `PLAYBOOK_URLS.md` and uses the same
segments:

```text
/{userType}/{module}/{resource}[/{resourceId}[/{subresource}]]

/app/projects/:projectId
/app/projects/:projectId/funnels/:funnelId
/app/funnels/leads/:leadId
/admin/tenants/:tenantId/users
/admin/tenants/:tenantId/usage
/admin/tenants/:tenantId/billing
/partner/campaigns/:campaignId
/public/pricing
```

With a file-based router, route groups mirror the playbook:

```text
app/
  app/(authenticated)/funnels/leads/[leadId]/page
  admin/(admin)/tenants/[tenantId]/users/page
  partner/(partner)/campaigns/[campaignId]/page
  public/pricing/page
  login/page
  403/page
```

Rules:

- **One layout and one guard per `{userType}`**: `AppLayout`, `AdminLayout`
  (admin sidebar + admin role guard), `PartnerLayout`, `PublicLayout`.
- **Code-split per `{userType}`**: the admin bundle is not loaded for `/app`
  users. This is for performance and surface reduction, **not** for
  security.
- `{tenantId}` appears in the URL only where the URL playbook includes it
  (admin and cross-tenant routes). On `/app` routes the active tenant is the
  session's.
- Route builders live in `core` (`routes.app.lead(leadId)`,
  `routes.admin.tenantUsers(tenantId)`). No hand-written path strings in
  Views.
- Redirect rules: unauthenticated → `/login` (with a `returnTo`), authenticated
  without permission → `/403`, wrong experience → the user's allowed
  dashboard.
- An unknown `{userType}` or module renders a not-found page; the router does
  not guess.

### 5.2 Guards

```text
Route guard (UX)  →  checks session.activeRole and permissions
Backend (truth)   →  checks token, tenant, RBAC, ownership on every call
```

- `RequireUserType('admin')` and `RequirePermission('tenant.users.read')`
  decide what to render or where to redirect. They read the `Session`.
- A guard passing never implies the call will succeed. Every ViewModel
  handles `forbidden` from the SDK (`requiredPermission` is available to show
  a precise message).
- Hiding a button (`canEdit`) is cosmetic. Never rely on it for protection.
- A user with multiple roles has an **active role** shown in the UI;
  switching it calls the session endpoint, resets the cache and re-evaluates
  the route.

### 5.3 React Native

Native has no address bar, but it keeps the same paths as its **navigation
and deep-link contract**:

```text
heimdall://app/funnels/leads/{leadId}
https://heimdall.example/app/funnels/leads/{leadId}   # universal / app link
```

- Navigators are split by `{userType}` (App stack, Admin stack, Partner stack)
  and the linking config maps path patterns to screens using the same
  segments as the web.
- The same `routes.*` builders from `core` generate the deep link, the web
  URL and the push-notification target.
- Auth guard = navigator switch based on `Session` (auth stack vs. the
  user-type stack). Same backend-is-truth rule.
- Platform concerns stay in the app: push tokens, background fetch, secure
  storage, permissions prompts, app lifecycle (refresh on foreground).

### 5.4 Chrome extension (MV3)

The extension has several execution contexts; each has strict
responsibilities.

| Context | Role | May call SDK? |
|---|---|---|
| **Service worker** (background) | Owns the session, tokens, SDK instance, alarms, long-running work, message router | **Yes (the only place)** |
| **Popup / side panel / options page** | Extension UI. React app with a **memory router** using the same paths (`/app/funnels/leads/:leadId`) | Through messaging to the service worker, or through a transport that proxies to it |
| **Content script** | Reads/augments the host page's DOM (for example, capturing a lead from a page) | **No.** Sends messages to the service worker |

Rules:

- **Tokens live only in the service worker's reach** (`chrome.storage.session`
  for access tokens). Content scripts run next to untrusted pages and never
  see tokens.
- The SDK in extension UIs uses a **messaging transport**: requests go to the
  service worker (`chrome.runtime.sendMessage`), which executes them with the
  real transport. The SDK API and ViewModels are unchanged.
- Messages are typed and versioned (`{ type, version, payload, correlationId }`);
  the service worker validates sender (`sender.id`, `sender.url`) and ignores
  unknown messages.
- The service worker is ephemeral (MV3): no in-memory state as the source of
  truth. Persist in `chrome.storage`, schedule with `chrome.alarms`, and
  resume idempotently.
- Minimal permissions: declare the narrowest `host_permissions` (the Heimdall
  API origin and only the sites the feature needs), prefer
  `activeTab`/optional permissions, no remote code, strict extension CSP.
- Extension UI is the `/app` experience (employee). `admin` and `partner`
  routes are not shipped in the extension unless a requirement says so.
- Authentication: sign-in through the web app or `chrome.identity` flow;
  the extension never asks for the password inside a content script.
- Content-script UI is injected in a Shadow DOM so host-page CSS cannot
  break it and ours cannot leak.

## 6. Cross-cutting rules

### 6.1 Authorization in the UI

```ts
const { can } = usePermissions();          // from Session
can('funnels.leads.update');               // module.resource.action
```

- Permission strings are identical to the backend's
  (`funnels.leads.read`, `tenant.users.manage`). They are typed from a
  generated enum so a typo fails compilation.
- Use `can()` to hide or disable controls and choose redirects.
- Treat `403` as a normal state: show `requiredPermission` when present,
  offer a way back, never loop redirects.
- Ownership and tenant checks are not duplicated in the client; the backend
  returns `404`/`403` and the ViewModel maps it.

### 6.2 Long-running operations

Operations that the backend answers with `202 Accepted` (for example
`POST /api/v1/app/projects/{projectId}/launch`) are handled in the
ViewModel through an SDK helper:

```ts
const run = await sdk.app.projects.launch(projectId, input);   // 202 + runId
// poll GET /api/v1/app/projects/{projectId}/runs/{runId} with backoff,
// or subscribe to SSE/WebSocket when available
```

- Polling uses exponential backoff, stops on terminal status, pauses when the
  screen is hidden (web `visibilitychange`, native `AppState`, extension
  panel closed) and cancels on unmount.
- The UI shows progress and resumes after reload because the `runId` is in
  the URL or persisted state.

### 6.3 Secrets and sensitive data

- Tenant provider keys (for example Apify) are **write-only in the UI**: the
  admin can set or rotate them, the API never returns them, and the UI shows
  only a masked tail and last-updated time.
- No secrets, API keys or signing material in the bundle, `.env` shipped to
  the client, or extension manifest.
- Never log tokens or PII. Redact in the analytics and error-reporting
  adapters.
- Sanitize any HTML that comes from extracted lead data before rendering; do
  not use `dangerouslySetInnerHTML` with unsanitized content (especially in
  content scripts).

### 6.4 Errors and observability

- Error boundary per route/layout; a failed screen does not take down the
  shell.
- Every failure shows the `correlationId` (copyable) so support can find the
  backend trace.
- Error reporting and analytics are ports (`Analytics`, `ErrorReporter`) with
  platform adapters, tagged with `userType`, `tenantId` and app version.
- Optimistic updates only for operations that are idempotent and easily
  reverted, with rollback on error.

### 6.5 Performance and offline

- Route-level and `{userType}`-level code splitting; lists virtualized
  (`FlatList` on native).
- Cursor pagination from the SDK; never fetch unbounded lists.
- Query cache with sensible `staleTime`; invalidate by the key convention in
  4.3.
- Offline: native and extension may queue **idempotent** commands with an
  `Idempotency-Key` and replay on reconnect; every ViewModel renders the
  `offline` status.

### 6.6 i18n and accessibility

- All user-visible strings come from message keys (`lead.moved`), including
  those emitted by ViewModels through `Notifier`. No literal text in
  ViewModels.
- Accessibility is a View responsibility: roles, labels, focus management,
  contrast; tokens enforce minimum contrast.

### 6.7 Design system

- `tokens` (color, spacing, typography, radii) is the shared source;
  `ui-web` and `ui-native` implement the same component names and props.
- Layouts per `{userType}` use the same tokens with different navigation
  chrome (admin sidebar, partner navigation, mobile tab bar).

## 7. Testing

| Level | Scope | Tools (examples) |
|---|---|---|
| ViewModel | State/commands with fake SDK and ports, all statuses (`forbidden`, `offline`, `error`) | `renderHook`, Vitest/Jest |
| SDK | Request shape per `{userType}/{module}/{resource}`, headers, retry, 401 refresh, error mapping | MSW / mock server, contract tests against OpenAPI |
| View | Renders each status from a stubbed ViewModel | React Testing Library / RN Testing Library |
| Guard/route | Redirects to `/login`, `/403`, allowed dashboard; route builders match the URL playbook | Router test utils |
| Architecture | Dependency rules from 1.1 | `dependency-cruiser` / ESLint |
| End-to-end | Critical flows per platform | Playwright (web, extension with the unpacked extension loaded), Maestro or Detox (native) |

A ViewModel test must run without DOM, native runtime or `chrome` APIs. If it
needs them, a platform concern leaked into the ViewModel.

## 8. Concrete example for Heimdall: lead detail

Same feature on the three clients.

```text
Backend:   GET /api/v1/app/funnels/leads/{leadId}
SDK:       sdk.app.funnels.leads.get(leadId)
ViewModel: packages/viewmodels/funnels/useLeadDetailViewModel.ts
Permission (UI hint): funnels.leads.update
```

| Client | Entry | View |
|---|---|---|
| Web | `/app/funnels/leads/:leadId` inside `AppLayout` + `(authenticated)` guard | `apps/web/.../LeadDetailView` (DOM) |
| Native | `heimdall://app/funnels/leads/{leadId}` in the App stack | `apps/mobile/.../LeadDetailScreen` (RN) |
| Extension | Side panel memory route `/app/funnels/leads/:leadId`; the content script sends `lead-captured` to the service worker, which creates the lead with the SDK and opens the panel on the new route | `apps/extension/.../LeadDetailView` (DOM) |

Admin variant: `/admin/tenants/{tenantId}/funnels/leads/{leadId}` calls
`sdk.admin.tenants.funnels.leads.get(tenantId, leadId)`. It reuses the same
presentational components with a different ViewModel and different backend
permissions; no business logic is copied.

Launch-campaign flow: the `/app/projects/{projectId}` View calls
`vm.launch()`, the ViewModel calls `sdk.app.projects.launch(...)`, receives
`runId`, navigates through the `Navigator` port to the run route, and polls
the run until a terminal status. If the user lacks `campaigns.launch`, the
backend returns `FORBIDDEN` with `requiredPermission` and the ViewModel
exposes the `forbidden` status; the hidden button was only a courtesy.

## 9. Checklist

- [ ] Views only talk to ViewModels; ViewModels only talk to the SDK and
      ports; the SDK has no React or platform imports.
- [ ] Dependency rules (1.1) are enforced in CI.
- [ ] SDK namespaces and paths mirror `/api/v1/{userType}/{module}/{resource}`;
      types are generated from the OpenAPI contract.
- [ ] No hand-written URL strings; route builders live in `core`.
- [ ] Web routes follow `/{userType}/{module}/{resource}`; one layout and one
      guard per `{userType}`; code-split per `{userType}`.
- [ ] No generic `/user`; the end-user experience is `/app`.
- [ ] `{tenantId}` in the URL only on admin and multi-tenant routes; the
      active tenant lives in the session.
- [ ] Guards and `can()` only shape the UI; every ViewModel handles
      `forbidden`, `not-found`, `offline` and `error`.
- [ ] Permission strings are typed `module.resource.action` and match the
      backend.
- [ ] React Native deep links reuse the same path segments and `routes.*`
      builders.
- [ ] Extension: SDK and tokens only in the service worker; content scripts
      message, never call the API; minimal permissions; no remote code.
- [ ] Tokens are stored per platform in the approved store; no refresh token
      in `localStorage`.
- [ ] Provider API keys are write-only in the UI; nothing secret ships in the
      client.
- [ ] `202` operations are polled with backoff, cancellable and resumable from
      a `runId` in the URL/state.
- [ ] Filters and pagination live in the URL query string (web/extension) or
      navigation params (native).
- [ ] Strings are message keys; the `correlationId` is shown on failures.
- [ ] ViewModel tests run without DOM, native runtime or `chrome` APIs.
