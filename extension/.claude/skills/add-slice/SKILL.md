---
name: "<app> Add Slice"
description: "Scaffold a new feature slice (src/features/<scope>) in the <app> extension — domain, application, adapters, ui, viewmodels, contracts and the barrel — wired in entrypoints/ with the fake-backed test gate, as one change."
division: core
---

# <app> Add Slice

A reusable scaffolding workflow for adding a new feature slice to the `<app>` Chrome extension. An agent (for example the planner or the architect role) invokes this skill to produce the slice declaration, its contracts, its wiring and its boundary checklist together — so a new slice never lands without its registration and its boundary discipline. The output is a wiring sketch plus a boundary checklist, in plain text.

## When to Activate

- A new vertical slice (`src/features/<scope>/`) is requested.
- An existing slice needs a new dependency on `src/platform/`, `src/storage/` or `src/contracts/`.
- The hexagonal dependency direction must be verified after a wiring change: domain stays pure, application never reaches adapters, features never import each other.
- A reviewer asks for the slice plus wiring bundle before approving an internal-structure change.

Do not activate for an in-slice refactor that adds no new slice and no new cross-boundary dependency — that is ordinary implementation work.

## Core Principles

- The slice is the unit of reuse. A new slice is a new `src/features/<scope>/` directory with `domain/`, `application/`, `adapters/{in,out}/`, `contracts/`, `ui/`, `viewmodels/` and a barrel `index.ts` — together.
- The dependency direction is invariant. `domain/` imports only the shared kernel; `application/` imports domain, the shared kernel and the contracts; `adapters/` import the application layer, the contracts and the platform; `ui/` and `viewmodels/` depend on the port types; `entrypoints/` wires everything. A wire that reverses this is a BLOCK.
- Ports are consumer-defined. `application/ports.ts` declares the interfaces the slice's use cases need; the production adapter lives in `adapters/out/`, the fake lives in `tests/_shared/fakes/`.
- The `chrome.*` boundary belongs to `src/platform/`. A slice that needs a platform capability takes it through a port; it never imports `chrome` itself.
- Slices never import each other. A shape shared by two slices moves down to `src/contracts/` (zod) or `src/shared-kernel/` (pure types), never as a cross-slice import edge.

## Rules

- Anti-hallucination: do not invent file paths, port names or import edges that are not in the tree. Every scaffolding step cites a file that exists or is being added.
- Every claim needs a source: cite the local playbook or the local file for a boundary rule; cite the official Chrome documentation for a platform behaviour.
- Do not emit the final sketch as a fenced code block when raw output is expected — the sketch is plain text with labelled sections.
- Verify the cited files exist before reporting; a wiring step that cites a port that is not being added is a hallucination.
- A new slice must appear under `src/features/` **and** be wired from an entrypoint; one without the other is a REVISE.
- The `chrome` global outside `src/platform/**` is a BLOCK: ESLint (`no-restricted-globals`) and dependency-cruiser (`no-chrome-outside-platform`) will fail the build.
- A slice that needs a new permission, a new host pattern or a new durable record is out of scope for this skill — route it to `/permission-change` or `/storage-migration`.

## Research References

Provenance table — every claim in a finding needs a row. The link column points at the primary source actually read.

| claim | source | year | link |
| --- | --- | --- | --- |
| The content script runs in an isolated world by default, so page script cannot reach the extension's variables | Chrome extension content scripts documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts |
| The service worker is the only long-lived context and owns the durable state | Chrome extension service worker basics | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/basics |
| Storage is split into `local`, `session` and `managed` areas with different lifetimes | `chrome.storage` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/storage |
| `sidePanel` requires the permission and an explicit behaviour set in the worker | `chrome.sidePanel` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/sidePanel |
| The architecture rule that `chrome.*` lives only in `src/platform/**` | `PLAYBOOK_CHROME_EXTENSION.md` | 2024 | local file |
| The slice layout and the dependency direction the linter enforces | `dependency-cruiser.config.cjs` | 2024 | local file |

## Output Format

A wiring sketch plus a boundary checklist, anchored to the slice root. Plain text — not a fenced block:

```
SLICE  src/features/<scope>

  files:
    domain/            -> pure types (shared-kernel imports only)
    application/ports.ts   -> consumer-owned port types
    application/use-cases.ts -> orchestration over the ports
    adapters/out/      -> production adapters over the platform
    adapters/in/       -> inbound adapters (protocol handlers)
    contracts/         -> zod schemas this slice owns
    ui/                -> React components + the ViewModel hook
    viewmodels/        -> the state machine, no react-dom, no chrome
    index.ts           -> the barrel the entrypoints import

  deps:
    src/contracts/     -> the shared zod schemas
    src/platform/      -> the only owner of chrome.*
    entrypoints/       -> construct the adapters, pass them to the slice
    tests/<scope>/frontend/{unit,contract}/ -> the tiered tests

  direction:
    domain/        <- (nothing internal)
    application/   <- domain/ + shared-kernel + contracts
    adapters/      <- application/ + contracts + platform
    ui/            <- viewmodels/ + react

CHECKLIST
  [ ] slice declared under src/features/<scope>/ with the file set above
  [ ] barrel index.ts exported and imported by an entrypoint
  [ ] ports are consumer-defined; one production adapter + one fake
  [ ] dependency direction is inward; no cross-slice import
  [ ] no chrome.* outside src/platform/
  [ ] unit + contract tests written; pnpm run test green
```

Summary line: `add-slice: N slice(s) scaffolded, M wiring step(s) — verdict: APPROVE | REVISE`.
