---
name: "<app> Add Module"
description: "Scaffold a new workspace module (crates/<app>-<scope>) in <app>: Cargo.toml member entry, workspace.dependencies path pin, src/lib.rs module doc, hexagonal dependency direction, and the mock-port test gate, as one change."
division: backend
---

# <app> Add Module

A reusable scaffolding workflow for adding a new crate to the `<app>` Cargo workspace. An agent (e.g. the API designer or ownership reviewer role) invokes this skill to produce the member declaration, the dependency wiring, the module doc, and the dependency-direction check together — so a new crate never lands without its workspace registration and its boundary discipline. The output is a wiring sketch plus a boundary checklist.

## When to Activate

- A new vertical slice (a `crates/<app>-<scope>` crate) is requested.
- An existing crate needs a new dependency on another workspace member.
- The hexagonal dependency direction must be verified after a wiring change (ports must not depend on adapters; core must not depend on ports).
- A reviewer asks for the module + wiring bundle before approving a workspace change.

Do not activate for in-crate refactors that add no new member and no new cross-crate dependency.

## Core Principles

- The workspace is the unit of reuse. A new crate is a new `members` entry, a new `[workspace.dependencies]` path pin, and a new `src/lib.rs` with a module doc — together.
- Hexagonal direction is invariant. `<app>-core` depends on nothing in the workspace; `<app>-ports` depends on `<app>-core`; adapters depend on ports; `<app>-api` depends on all. A wire that reverses this is a BLOCK.
- Ports are traits, adapters are impls. `crates/<app>-ports` holds `#[async_trait] trait ...`; `crates/<app>-ports/src/adapters/` and `crates/<app>-ports/src/mock/` hold impls behind `#[cfg]` gates.
- Mocks live behind `#[cfg(any(test, feature = "mock"))]`. Mock code never reaches a release binary.
- The module doc is the contract. `src/lib.rs` opens with a `//!` doc that states what the crate owns and what it depends on.

## Rules

- Anti-hallucination: do not invent crate names or dependency paths that are not in the workspace. Every wiring step cites a `Cargo.toml` line that exists or is being added.
- Every stat needs a source. If you cite the hexagonal architecture, cite the dependency-rules doc; if you cite the Cargo workspace `members` rule, cite the Cargo book. No unsourced claims.
- Do not emit the final sketch as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (member / deps / lib.rs / direction / checklist).
- Verify the cited `Cargo.toml` lines exist before reporting; a wiring step that cites a `[workspace.dependencies]` entry that is not being added is a hallucination.
- A new crate must appear in `members` AND in `[workspace.dependencies]` with a `path = "crates/<app>-<scope>"` pin; one without the other is a REVISE.
- `<app>-core` must not depend on any other workspace member; `<app>-ports` must depend only on `<app>-core`. A reverse edge is a BLOCK.
- Mock modules must be gated with `#[cfg(any(test, feature = "mock"))]`; a mock that leaks past the gate is a soundness bug.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| Cargo workspace members are listed in [workspace] members | Cargo book, workspaces | 2024 | <TODO: primary doc link> |
| Hexagonal architecture: ports are traits, adapters are impls | dependency-rules doc / hexagonal architecture | 2024 | <TODO: primary doc link> |
| #[cfg(any(test, feature = "mock"))] keeps mock code out of release | Rust reference, conditional compilation | 2024 | <TODO: primary doc link> |
| async-trait enables trait objects with async fns in Arc<dyn Trait> | async-trait crate docs | 2024 | <TODO: primary doc link> |

## Output Format

A wiring sketch plus a boundary checklist, anchored to the workspace `Cargo.toml`. Plain text — not a fenced block:

```
MODULE  crates/<app>-<scope>

  member:
    Cargo.toml [workspace] members  -> add "crates/<app>-<scope>"

  deps:
    Cargo.toml [workspace.dependencies]  -> "<app>-<scope>" = { path = "crates/<app>-<scope>" }
    crates/<app>-<scope>/Cargo.toml       -> <app>-core = { workspace = true }

  lib.rs:
    crates/<app>-<scope>/src/lib.rs  -> //! <app>-<scope> — owns <scope> use cases; depends on <app>-core only.

  direction:
    <app>-core  -> (none)
    <app>-ports -> <app>-core
    <app>-<scope> -> <app>-core   (OK: inward)

CHECKLIST
  [ ] member declared in [workspace] members
  [ ] path pin declared in [workspace.dependencies]
  [ ] src/lib.rs module doc states ownership + dependencies
  [ ] dependency direction is inward (core depends on nothing; ports on core)
  [ ] mock module (if any) behind #[cfg(any(test, feature = "mock"))]
  [ ] cargo check --workspace green
```

Summary line: `add-module: N module(s) added, M wiring step(s) — verdict: APPROVE | REVISE`.
