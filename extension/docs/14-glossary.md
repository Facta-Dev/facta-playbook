# Glossary

Every term used in this set, defined once. Where a term has a Chrome meaning
and a kit meaning, the kit meaning is the one that counts here.

**Adapter** — An implementation of a port that talks to the outside world.
`src/features/notes/adapters/out/rpc-note-repository.ts` implements
`NoteRepository` over the message protocol.

**Alarms** — The `chrome.alarms` API: the only reliable way to make the
worker wake up on a schedule. Requires the `alarms` permission.

**Barrel** — The `index.ts` of a slice: its only public face. The
composition roots import from the barrel, never from a layer inside it.

**Blocked host** — A host pattern the administrator refuses. Checked before
the allowlist in `isUrlAllowed`.

**Cold start** — The worker waking up from nothing. Every listener must be
registered during the first evaluation of the worker module.

**Composition root** — The file that knows everything and wires it together:
`entrypoints/background.ts`, `entrypoints/sidepanel/main.tsx`,
`entrypoints/options/main.ts`. It contains no rules.

**Content script** — Code injected into a page by the browser, running in an
isolated world. The kit's example only sets a dataset attribute on the
document element.

**Contract** — A zod schema in `src/contracts/` that both ends of a message
validate with. The contract bounds what may travel; the domain bounds what
may exist.

**Domain** — The rules of a feature: entities, invariants and their errors.
Pure: no browser, no framework, no storage.

**Domain rule vs contract bound** — The contract says "a title is at most
200 characters" (what may travel on the wire); the domain says "at most 120,
and no markup" (what may be a note). Both exist on purpose.

**Envelope** — The wrapper of every message: `{ v, id, type, correlationId,
payload }`.

**Error code** — One of the closed set in `src/shared-kernel/errors.ts`:
`bad-envelope`, `unknown-type`, `forbidden-sender`, `validation-failed`,
`not-found`, `internal`.

**Extension page** — A page served from `chrome-extension://<id>`: the side
panel, the options page. Sender class `ui`.

**Fail closed** — When a policy or a record cannot be read, the extension
refuses instead of allowing. An empty allowlist allows nothing.

**Feature flag** — A boolean setting. Managed and user flags merge; the
administrator wins on a collision.

**Host pattern** — A match pattern such as `https://*.example.com/*`.
`src/security/matchers.ts` implements the matching.

**Layered rules** — The `PROTOCOL` table's `senders` array: which context
classes may send a given message type.

**Locked setting** — A setting the administrator published. The resolver
reports it in `locked[]`; the UI renders it read-only.

**Manifest** — The extension's declaration, written once in
`src/manifest.ts` and built into `dist/manifest.json`.

**Managed policy** — The administrator's policy, published through
`chrome.storage.managed` and described by `managed-schema.json`.

**Manifest V3 (MV3)** — The extension platform generation this kit targets:
a service worker instead of a background page, and no remotely hosted code.

**Note** — The example entity of the `notes` slice: an id, a title, a body
and two timestamps.

**Options page** — The extension's settings surface. In this kit it shows
the effective policy, read-only.

**Platform** — `src/platform/**`: the only folder allowed to name
`chrome.*`. It hands out plain values, never `chrome` types.

**Policy** — The effective settings: managed over user over defaults.

**Port** — An interface the consuming slice declares, so the use-case
depends on the abstraction and not on the browser.

**Protocol version** — `PROTOCOL_VERSION` in `src/contracts/protocol.ts`,
currently `1`. A message with another version is refused.

**Result** — A value-typed outcome: `{ ok: true, value }` or
`{ ok: false, error }`. Expected failures are values, not exceptions.

**Rule** — A statement about what a domain may contain. Enforced by the
domain functions, and by the strict schemas at the boundaries.

**Sender class** — `ui` or `content`. Computed by `classifySender` before
anything is parsed.

**Sender validation** — The check that a message comes from this extension
and from a context allowed to send it.

**Service worker** — The background context of MV3. Event-driven, killed
after ~30 s of inactivity, restarted cold.

**Side panel** — The docked panel opened from the toolbar. The main surface
of this kit.

**Slice** — A feature, cut end to end: domain, use-cases, storage,
messages and UI. Sealed: a slice never imports another slice.

**Strict object** — A zod object that rejects unknown keys. Used for every
payload, record and settings shape that the extension owns.

**Trust boundary** — A point where data that is not trusted crosses into
code that is. The extension has four: page → content script, message →
worker, record → store, administrator → policy.

**View model** — The presentation state and the commands that change it. No
React, no DOM, no browser. `useSyncExternalStore` binds it to a component.

**Vertical slicing** — Cutting the code by feature instead of by technical
layer, so one feature is one folder with all of its layers.

**Write-through** — Writing to the durable area first and mirroring into the
volatile one after a debounce. The mirror can be lost when the worker dies;
the durable write cannot.

## See also

- [Overview & Principles](00-overview-and-principles.md)
- [Playbooks](13-playbooks.md)
