# MEMORY.md — `<app>` premise

Durable, versioned premise for every session. Keep it under ~100 lines: facts
an agent cannot derive quickly from the code, not a copy of the code.

## Product

`<app>` is a Chrome extension (Manifest V3): a service worker that owns the
state, a side panel as the main UI, an options page, and one content script
per matched site family. It talks to a backend through `packages/sdk`. The
extension is a **public client**: PKCE, no client secret, no token at rest in
`storage.local`.

## Slices

A slice is `src/features/<slice>/` with `domain/` · `application/` ·
`adapters/{in,out}/` · `contracts/` · `ui/` · `viewmodels/`, and a barrel
`index.ts` the composition roots import from.

| Slice   | Owns                              | Notes                                            |
| ------- | --------------------------------- | ------------------------------------------------ |
| `notes` | `notes:record` in `storage.local` | **example slice** — replace with the real domain |

Slices never import each other. A shape shared by two slices moves to
`src/contracts/` (zod) or `src/shared-kernel/` (pure types), never a
cross-slice import edge.

## Permission posture

`src/manifest.ts` declares exactly three permissions — `storage`, `sidePanel`,
`alarms` — and **no host permissions at all** (`host_permissions: []`). The
content script matches `https://example.com/*` only. A new permission is a
security review: an entry in `docs/permissions-register.md`, a justification,
and a test. `debugger` is never declared, not even as an optional permission.

Storage areas: `local` is durable, `session` is volatile, `managed` is the
read-only administrator policy. Effective policy = managed > user > defaults,
resolved by the pure `resolvePolicy` in `src/policy/`.

## Messaging

One envelope, `{ v, id, type, correlationId, payload }`, validated by the
same zod schemas on both ends (`src/contracts/protocol.ts`).
`PROTOCOL_VERSION` is **1**; an unknown version or type is a typed error, not
an exception. Errors are values: `Result<T, RpcError>` with stable codes from
`src/shared-kernel/errors.ts`.

Sender validation happens **before** any parsing: `sender.id` must equal the
extension id, the origin must be an extension page, the frame must be the top
frame, and the message type must allow the sender's class (`ui` or
`content`). The per-type allowlist is the `PROTOCOL` table.

## Storage

Every durable record carries a schema version. The notes record is at
**version 2** (`NOTES_STORAGE_VERSION`): v1 kept the body under `text` and
had no `updatedAt`; v2 uses `body` and stamps `updatedAt`. Migrations are
pure `(vN) → (vN+1)` functions, idempotent, run on install and lazily on
first read. A corrupt record is dropped to `notes:record.backup` for one
release cycle instead of taking the worker down.

## Test tiers

| Tier                                  | Where                                     | Scope                                                    |
| ------------------------------------- | ----------------------------------------- | -------------------------------------------------------- |
| unit                                  | `tests/<slice>/frontend/unit/`            | domain, use cases, pure functions — fakes only           |
| viewmodel                             | `tests/<slice>/frontend/unit/viewmodels/` | states and commands with a fake RPC client               |
| contract                              | `tests/<slice>/frontend/contract/`        | the protocol, both ends, the same schemas                |
| platform                              | `tests/_platform/`                        | architecture isolation + sender allowlist, against fakes |
| integration / e2e / policy / security | added per feature                         | real Chrome, managed policy, hostile pages               |

No test touches the network, a real customer page or a real identity
provider. Every message type ships a contract test **and** a sender-class
test.

## Stack

Node **≥ 24** · TypeScript **5.9** (strict, `noUncheckedIndexedAccess`,
`exactOptionalPropertyTypes`) · React **19.3** · zod **4.6** · Vite **7.3** ·
Vitest **5.1** · ESLint **9.39** + typescript-eslint **8.72** ·
dependency-cruiser **18.7** · Prettier **3.10** · pnpm **11** · Chrome **116**
minimum. Runtime dependencies are exactly `react`, `react-dom` and `zod`.

## Gates

`make ci` (typecheck + lint + test + build) · `pnpm run evals`
(`node --test evals/contracts.test.mjs`) · `scripts/verify-template.sh`
(template factory only: it stages a renamed copy and runs every gate on it).

## Where the contracts live

`PLAYBOOK_CHROME_EXTENSION.md` (architecture, manifest, messaging, storage,
security) · `PLAYBOOK_TESTS.md` · `PLAYBOOK_SECURITY.md` ·
`PLAYBOOK_FRONTEND.md` · `PLAYBOOK_URLS.md` · `PLAYBOOK_AUDIT.md` ·
`PLAYBOOK_CLAUDE_CODE.md` · `AGENTS.md` (agent workflow).
