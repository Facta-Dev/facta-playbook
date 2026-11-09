---
name: "<app> Unsafe Guard"
description: "Review unsafe blocks in <app> for soundness; flag every unsafe fn, unsafe impl, std::mem::transmute, and extern \"C\" FFI boundary, and block any relaxation of the unsafe_code = \"forbid\" workspace invariant across the <scope> slice."
division: security
---

# <app> Unsafe Guard

A reusable soundness review workflow for the `<app>` Rust workspace. An agent (e.g. the unsafe reviewer role) invokes this skill on any diff that touches `crates/<app>-*/src/**/*.rs` or `Cargo.toml` lint configuration. The workspace invariant is `unsafe_code = "forbid"`; the skill enforces it absolutely and reports any escape hatch.

## When to Activate

- A diff adds or modifies `unsafe`, `unsafe fn`, `unsafe impl`, `std::mem::transmute`, `std::mem::transmute_copy`, `extern "C"`, `extern "Rust"`, or `#[link(...)]`.
- A diff edits `[workspace.lints.rust] unsafe_code`, or adds a per-crate `#![forbid(...)]` / `#![deny(...)]` / `#![allow(...)]` that touches `unsafe_code`.
- A new dependency is introduced whose FFI surface reaches `<app>` at the boundary.
- A reviewer asks for a soundness pass before approving a PR that touches the FFI or lint surface.

Do not activate for diffs with no `unsafe` surface and no Cargo.toml lint edit.

## Core Principles

- `forbid` is absolute. It is not a default to relax for a deadline; any edit toward `deny` or `allow` is a BLOCK.
- `transmute` is never the answer. Use `bytemuck::cast` / `zerocopy` (which encode safety in traits) or an explicit `From` / `TryFrom` impl.
- FFI is a decision, not a convenience. A legitimate `extern "C"` boundary requires an architect decision and a `cargo deny` review of the bound crate.
- The compiler is the first reviewer. `unsafe_code = "forbid"` means `cargo build` is the gate; the skill catches what the compiler cannot see — Cargo.toml edits, dependency surfaces, and review-time proposals.
- Cite the invariant, then the alternative. "BLOCK — unsafe_code = forbid" is the rule; "use Arc::clone / From / bytemuck" is the review.

## Rules

- Anti-hallucination: do not invent `unsafe` blocks that are not in the diff. Every finding cites a path and line that exists in the tree.
- Every stat needs a source. If you cite the four undefined-behavior roots, name the Rust reference section; if you cite `cargo deny`, name the check. No unsourced claims.
- Do not emit the final report as a fenced code block when raw line-anchored output is expected — the report is plain text, one finding per line, severity-prefixed.
- Verify by reading the cited file and line before reporting. A finding that cites an `unsafe` block the diff does not contain is a hallucination.
- Zero `unsafe` in `crates/<app>-*/src/**/*.rs` is the floor; the compiler enforces it via `forbid`. The skill enforces the Cargo.toml + dependency surface the compiler cannot see.
- Zero `std::mem::transmute` / `transmute_copy`; these are always `unsafe` and always a BLOCK.
- Zero `extern "C"` / `extern "Rust"` / `#[link]` introduced without an architect decision recorded in the PR.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| unsafe_code = "forbid" fails the build on any unsafe block | Rust reference, unsafe keyword | 2024 | <TODO: primary doc link> |
| The four undefined-behavior roots for unsafe Rust | Rust reference, unsafe Rust | 2024 | <TODO: primary doc link> |
| std::mem::transmute is always unsafe and unsound by default | Rust std docs, mem::transmute | 2024 | <TODO: primary doc link> |
| cargo deny check surfaces transitive unsafe in dependencies | cargo-deny documentation | 2024 | <TODO: primary doc link> |

## Output Format

A line-anchored report, one finding per line, severity-prefixed, followed by a summary line. Plain text — not a fenced block:

```
BLOCK  crates/<app>-core/src/rbac.rs:88  `unsafe { *ptr }` dereferences a raw pointer — workspace invariant unsafe_code = "forbid". Replace with a safe borrow or Arc::clone.
BLOCK  crates/<app>-api/Cargo.toml:42  `#![allow(unsafe_code)]` relaxes the workspace forbid — remove it.
WARN   Cargo.toml:91  dependency `fast-sys = "0.3"` links an FFI crate; cargo deny will surface its unsafe surface. Route to architect.
PASS   crates/<app>-ports/src/adapters/widget_pg.rs  no unsafe; sqlx is the only FFI surface and it is encapsulated.
```

Summary line: `unsafe-guard: N block(s), M warning(s), K pass — verdict: BLOCK | APPROVE`. Any non-zero BLOCK is an automatic BLOCK verdict.
