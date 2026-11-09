---
name: "<app> Add Module"
description: "Scaffold a new feature slice (src/features/<scope>) in <app>: ports.ts consumer-owned interfaces, usecase.ts, module.ts register(), deps wiring in src/bin/api.ts, hexagonal direction, and the fake-backed test gate, as one change."
division: backend
---

# <app> Add Module

A reusable scaffolding workflow for adding a new feature slice to the `<app>` codebase. An agent (e.g. the API designer or ownership reviewer role) invokes this skill to produce the slice declaration, the dependency wiring, the module doc, and the boundary check together — so a new slice never lands without its registration and its boundary discipline. The output is a wiring sketch plus a boundary checklist.

## When to Activate

- A new vertical slice (`src/features/<scope>/`) is requested.
- An existing slice needs a new dependency on `#platform/` or `#shared/`.
- The hexagonal dependency direction must be verified after a wiring change (adapters must not depend on use cases; kernel must not depend on anything internal).
- A reviewer asks for the module + wiring bundle before approving an internal-structure change.

Do not activate for in-slice refactors that add no new slice and no new cross-feature dependency.

## Core Principles

- The slice is the unit of reuse. A new slice is a new `src/features/<scope>/` directory with domain.ts / ports.ts / usecase.ts / http.ts / postgres.ts / module.ts / queries.ts — together.
- Hexagonal direction is invariant. `#shared/` (kernel) imports only node builtins; `#platform/` imports shared + third-party drivers; features import shared + platform-declared ports; `src/bin/api.ts` depends on all. A wire that reverses this is a BLOCK.
- Ports are consumer-defined. `ports.ts` declares the interfaces the slice's use cases need (defined BY the consumer); `postgres.ts` is the production adapter (pg, over the slice's Kysely-typed `queries.ts`); the fake lives beside `usecase.test.ts`.
- The module doc is the contract. The slice's files open with header comments stating what the slice owns and what it depends on.
- Slices never import each other. A shared shape moves down to `#shared/` through an architect decision, never as a cross-slice import edge.

## Rules

- Anti-hallucination: do not invent import paths or import edges that are not in the tree. Every wiring step cites a file that exists or is being added.
- Every stat needs a source. If you cite the hexagonal architecture, cite the dependency-rules doc; if you cite the `imports` subpath-map rule, cite the Node.js packages documentation. No unsourced claims.
- Do not emit the final sketch as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (slice / deps / module.ts / direction / checklist).
- Verify the cited files exist before reporting; a wiring step that cites a `Deps` field that is not being added is a hallucination.
- A new slice must appear in `src/features/` AND be wired via `register(app, d)` in `src/bin/api.ts`; one without the other is a REVISE.
- `#shared/` must not depend on any other internal feature; `#platform/` must depend only on shared. A reverse edge is a BLOCK.
- Adapter code lives only in `postgres.ts` (pg) and `http.ts` (fastify); `domain.ts` stays pure (node builtins + `#shared/` only) or eslint domain-pure blocks the build.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim                                                                                                  | source                                        | year | link                     |
| ------------------------------------------------------------------------------------------------------ | --------------------------------------------- | ---- | ------------------------ |
| package.json `imports` subpath maps (#pkg/) are the single declaration point for internal module edges | Node.js packages documentation                | 2024 | <TODO: primary doc link> |
| Hexagonal architecture: ports are consumer-defined interfaces                                          | dependency-rules doc / hexagonal architecture | 2024 | <TODO: primary doc link> |
| Fakes beside usecase.test.ts compile out of the production build                                       | vitest documentation (include patterns)       | 2024 | <TODO: primary doc link> |
| pnpm install resolves package.json post-rename (no lockfile ships — INV-T3)                            | pnpm documentation                            | 2024 | <TODO: primary doc link> |

## Output Format

A wiring sketch plus a boundary checklist, anchored to the module root `package.json`. Plain text — not a fenced block:

```
MODULE  src/features/<scope>

  files:
    domain.ts    -> pure domain types (node builtins + #shared/ imports only)
    ports.ts     -> consumer-owned interfaces (DB, clock, idgen, email)
    usecase.ts   -> orchestration over the ports
    http.ts      -> Fastify handlers + envelope mapping
    postgres.ts  -> pg adapter over the slice's Kysely-typed queries.ts
    module.ts    -> register(app, d) route mounting
    queries.ts   -> Kysely table/row types + named queries (documented delta vs go sqlc)

  deps:
    package.json imports -> any new internal edge (#<pkg>/...) declared once
    src/bin/api.ts       -> construct the adapter, pass it in the slice's Deps
    module.ts            -> register(app, d) joined to the shared app

  direction:
    #shared/   <- (nothing internal)
    #platform/ <- #shared/ + drivers
    #features/<scope> <- #shared/ + its declared ports   (OK: inward)

CHECKLIST
  [ ] slice declared under src/features/<scope>/ with the file set above
  [ ] Deps wired once in src/bin/api.ts; register(app, d) mounted
  [ ] ports are consumer-defined; exactly one production adapter + one fake
  [ ] dependency direction is inward (#shared/ imports node builtins only)
  [ ] fakes live beside usecase.test.ts (compiled out of the production build)
  [ ] pnpm build green; eslint domain-pure + slice-isolation green
```

Summary line: `add-module: N module(s) added, M wiring step(s) — verdict: APPROVE | REVISE`.
