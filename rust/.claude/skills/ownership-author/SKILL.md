---
name: "<app> Ownership Author"
description: "Review borrow-checker and lifetime patterns in <app> crates; produce a line-anchored report of ownership, Arc<dyn Trait> wiring, and Send+Sync boundaries across the <scope> slice before the diff lands."
division: backend
---

# <app> Ownership Author

A reusable review workflow for borrow-checker and lifetime correctness in the `<app>` Rust workspace. An agent (e.g. the ownership reviewer role) invokes this skill on any diff that touches `crates/<app>-*/src/**/*.rs`; the skill is the workflow, the agent is the role. The output is a line-anchored report, not a rewrite.

## When to Activate

- A diff modifies `crates/<app>-api/src/state.rs`, `crates/<app>-api/src/routes/*.rs`, `crates/<app>-ports/src/**/*.rs`, or any `Arc<dyn Trait>` wiring.
- A handler signature changes, a trait object is introduced, or a lifetime is elided across a crate boundary.
- A reviewer asks for an ownership pass before approving a PR that crosses the axum 0.8 `Send + Sync` spawn boundary.

Do not activate for pure formatting, doc-only, or migration-only diffs.

## Core Principles

- Compiles is not correct. A borrow that passes `cargo build` and panics on the second concurrent request is a bug this skill owns.
- The composition root stays thin. `AppState` holds `Arc<dyn Trait>` collaborators and nothing else; logic lives in `crates/<app>-widgets` / `<app>-core`.
- Never reach for `unsafe`. The workspace ships `unsafe_code = "forbid"`; the fix is a different data structure or lifetime, never an escape hatch.
- Cite the failure mode, not the rule. "Not Send+Sync" is the rule; "panics on the second concurrent request because axum 0.8 spawns the handler" is the review.
- `tokio::sync::Mutex` only across `.await`; `std::sync::Mutex` for short in-memory holds; `RwLock` when reads dominate.

## Rules

- Anti-hallucination: do not fabricate file paths, line numbers, or compiler diagnostics. Every finding must cite a path and line that exists in the diff or the working tree.
- Every stat needs a source. If you claim "panics on the second request", cite the axum spawn model; if you cite a clippy lint, name the lint. No unsourced numbers.
- Do not emit the final review as a fenced code block when raw line-anchored output is expected — the report is plain text with one finding per line, severity-prefixed.
- Verify claims against the tree before reporting: read the cited file and line. A finding that cites a line the diff does not contain is a hallucination.
- `AppState` must remain `Clone + Send + Sync` with every field `Arc` or `Clone`; a field that breaks this is a BLOCK.
- No `unsafe` proposal; `unsafe_code = "forbid"` is a workspace invariant. Propose the safe alternative (`Arc::clone`, `From`, explicit lifetime) instead.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| axum 0.8 handlers run on a multi-thread runtime and require Send+Sync state | axum documentation / tower service model | 2024 | <TODO: primary doc link> |
| Arc::clone is a single atomic increment | Rust standard library docs | 2024 | <TODO: primary doc link> |
| NLL and two-phase borrows resolve borrow conflicts | Rust reference | 2024 | <TODO: primary doc link> |
| unsafe_code = "forbid" is a workspace lint | Cargo book, workspace lints | 2024 | <TODO: primary doc link> |

## Output Format

A line-anchored report, one finding per line, severity-prefixed, followed by a summary line. Plain text — not a fenced block:

```
BLOCK  crates/<app>-api/src/state.rs:42  AppState holds Rc<RefCell<WidgetCache>> — not Send+Sync; axum 0.8 spawns handlers on a multi-thread runtime and this will panic on the first concurrent request. Replace with Arc<tokio::sync::RwLock<WidgetCache>>.
WARN   crates/<app>-ports/src/domain/widget_repository.rs:18  elided lifetime binds the returned trait object to 'static implicitly; pass the borrow through with an explicit 'a.
PASS   crates/<app>-api/src/routes/widgets.rs  Arc<dyn WidgetRepository> cloned per-request is correct — single atomic increment.
```

Summary line: `ownership-author: N block(s), M warning(s), K pass — verdict: BLOCK | APPROVE`.
