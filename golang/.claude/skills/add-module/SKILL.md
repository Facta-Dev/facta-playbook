---
name: "<app> Add Module"
description: "Scaffold a new feature slice (internal/features/<scope>) in <app>: ports.go consumer-owned interfaces, usecase.go, module.go Register(), deps wiring in cmd/api/main.go, hexagonal direction, and the fake-backed test gate, as one change."
division: backend
---

# <app> Add Module

A reusable scaffolding workflow for adding a new feature slice to the `<app>` Go module. An agent (e.g. the API designer or ownership reviewer role) invokes this skill to produce the slice declaration, the dependency wiring, the module doc, and the boundary check together — so a new slice never lands without its registration and its boundary discipline. The output is a wiring sketch plus a boundary checklist.

## When to Activate

- A new vertical slice (`internal/features/<scope>/`) is requested.
- An existing slice needs a new dependency on `internal/platform/` or `internal/shared/`.
- The hexagonal dependency direction must be verified after a wiring change (adapters must not depend on use cases; kernel must not depend on anything internal).
- A reviewer asks for the module + wiring bundle before approving an internal-structure change.

Do not activate for in-slice refactors that add no new slice and no new cross-package dependency.

## Core Principles

- The slice is the unit of reuse. A new slice is a new `internal/features/<scope>/` directory with domain.go / ports.go / usecase.go / http.go / postgres.go / module.go / queries.sql — together.
- Hexagonal direction is invariant. `internal/shared/` (kernel) imports only stdlib; `internal/platform/` imports shared + third-party drivers; features import shared + platform-declared ports; `cmd/main.go` depends on all. A wire that reverses this is a BLOCK.
- Ports are consumer-defined. `ports.go` declares the interfaces the slice's use cases need (defined BY the consumer); `postgres.go` is the production adapter (sqlc-backed); the fake lives in the slice's `*_test.go`.
- The module doc is the contract. The slice's files open with doc comments stating what the slice owns and what it depends on.
- Slices never import each other. A shared shape moves down to `internal/shared/` through an architect decision, never as a cross-slice import edge.

## Rules

- Anti-hallucination: do not invent package paths or import edges that are not in the module. Every wiring step cites a file that exists or is being added.
- Every stat needs a source. If you cite the hexagonal architecture, cite the dependency-rules doc; if you cite the go.mod require rule, cite the Go modules reference. No unsourced claims.
- Do not emit the final sketch as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (slice / deps / module.go / direction / checklist).
- Verify the cited files exist before reporting; a wiring step that cites a `Deps` field that is not being added is a hallucination.
- A new slice must appear in `internal/features/` AND be wired via `Register(mux, d)` in `cmd/api/main.go`; one without the other is a REVISE.
- `internal/shared/` must not depend on any other internal package; `internal/platform/` must depend only on shared. A reverse edge is a BLOCK.
- Adapter code lives only in `postgres.go` (pgx) and `http.go` (net/http); `domain.go` stays pure (stdlib + shared only) or golangci-lint depguard blocks the build.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| go.mod requires are the single declaration point for direct deps | Go modules reference | 2024 | <TODO: primary doc link> |
| Hexagonal architecture: ports are consumer-defined interfaces | dependency-rules doc / hexagonal architecture | 2024 | <TODO: primary doc link> |
| Fakes in _test.go files compile out of the production binary | Go testing package docs | 2024 | <TODO: primary doc link> |
| go mod tidy resolves requires and writes go.sum post-rename | Go modules reference | 2024 | <TODO: primary doc link> |

## Output Format

A wiring sketch plus a boundary checklist, anchored to the module root `go.mod`. Plain text — not a fenced block:

```
MODULE  internal/features/<scope>

  files:
    domain.go   -> pure domain types (stdlib + internal/shared imports only)
    ports.go    -> consumer-owned interfaces (DB, clock, idgen, email)
    usecase.go  -> orchestration over the ports
    http.go     -> net/http handlers + envelope mapping
    postgres.go -> pgx adapter over generated internal/gen/db
    module.go   -> Register(mux *http.ServeMux, d Deps) route mounting
    queries.sql -> sqlc queries (schema: migrations) feeding internal/gen/db

  deps:
    go.mod require  -> any new third-party require (go get + go mod tidy)
    cmd/api/main.go -> construct the adapter, pass it in the slice's Deps
    module.go       -> Register(mux, d) joined to the shared mux

  direction:
    shared/    <- (nothing internal)
    platform/  <- shared/ + drivers
    features/<scope> <- shared/ + its declared ports   (OK: inward)

CHECKLIST
  [ ] slice declared under internal/features/<scope>/ with the file set above
  [ ] Deps wired once in cmd/api/main.go; Register(mux, d) mounted
  [ ] ports are consumer-defined; exactly one production adapter + one fake
  [ ] dependency direction is inward (shared/ imports stdlib only)
  [ ] fakes live in *_test.go (compiled out of the production binary)
  [ ] go build ./... green; golangci-lint depguard green
```

Summary line: `add-module: N module(s) added, M wiring step(s) — verdict: APPROVE | REVISE`.