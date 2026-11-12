# ADR 0002 — Classic service worker, no module type

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the background context of the extension

## Context

Manifest V3 runs the background context as a service worker. The manifest may
declare it as a module (`"background": { "service_worker": "background.js",
"type": "module" }`) or as a classic script. The choice has real consequences
for how the worker is built and started.

The template's worker has to:

- register its message listener **synchronously**, during the first
  evaluation of the worker script — a worker that is still starting misses
  the event that woke it;
- ship as a single file that Chrome can load without a module graph;
- keep no state in module globals, because the worker is terminated after
  ~30 seconds of inactivity and restarted cold.

## Decision

**Build the worker as a classic (non-module) service worker.** The manifest
declares no `type`:

```ts
background: {
  service_worker: "background.js",
},
```

The Vite build emits `dist/background.js` as a classic IIFE bundle: no
`import` or `export` statements survive in it, and `chrome.runtime.id` is
inlined. The listener is registered at the top level of
`entrypoints/background.ts`:

```ts
registerMessageServer({ runtime, logger, senderPolicy, handlers });
void setPanelBehavior({ openPanelOnActionClick: true });
```

## Alternatives considered

| Option                                   | Why not                                                                                                                                                                                                                                                              |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `"type": "module"`                       | A module worker is loaded as a module graph, which is a stricter contract for a context that must start fast and register its listener in the first tick. The classic bundle removes a class of start-up failure for no gain: the worker imports nothing at runtime. |
| A background _page_ (MV2)                | Manifest V3 removed it; an event page would also keep a DOM alive for nothing.                                                                                                                                                                                       |
| An `importScripts` bundle of shared code | The bundler already produces one file; splitting it by hand would only add a load-order constraint.                                                                                                                                                                  |

## Consequences

- The worker is one file with no runtime import: no module resolution in the
  browser, no top-level-await hazard before the listener registration.
- Everything the worker needs is wired at the top level of the entry file.
  A listener registered inside a promise callback would miss events.
- The bundle cannot share code with the pages at runtime; the shared code
  (the logger, for instance) is bundled into each output separately.
- The state rules of [Service Worker Lifetime](../02-service-worker-lifetime.md)
  apply without exception: nothing that must survive lives in a module
  global.

## See also

- [ADR 0001 — Vite and vite-plugin-web-extension](0001-vite-and-vite-plugin-web-extension.md)
- [Service Worker Lifetime](../02-service-worker-lifetime.md)
- [Manifest & Permission Engineering](../06-manifest-and-permission-engineering.md)
