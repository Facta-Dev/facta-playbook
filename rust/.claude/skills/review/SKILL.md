---
name: "review"
description: "Runs the <app> pre-landing review checklist across unsafe justification, unwrap/expect discipline, parameterized SQL auditing, and clippy -D warnings, producing a severity-ranked gate verdict against `just ci`."
division: core
---

# <app> review skill

> Reusable pre-landing review CHECKLIST for the `<app>` Rust workspace. An
> agent invokes this skill; the agent supplies contextual judgment, the skill
> supplies the deterministic procedure.

## When to Activate

Activate this skill before landing any change to `crates/<app>-*/src/**/*.rs`,
or whenever a request asks for a "review", "gate check", or "pre-landing
pass" on `<app>`. Also trigger on:

- A pull request touching a handler, repository adapter, or domain module.
- A request to confirm a change is safe to merge against `just ci`.
- A follow-up review after fixes from a prior `/review` pass.

Do NOT activate for documentation-only or `Cargo.lock`-only changes with no
`.rs` or `Cargo.toml` lint-config edits. This skill COMPLEMENTS the
`code-reviewer` core agent rather than replacing it: the agent reasons freely
about a diff's intent and tradeoffs, while this skill walks the same four
stack categories in the same order every time. Do not restate the agent's
prose here — the two are consulted together, not merged.

## Core Principles

- **Checklist, not judgment.** Four fixed categories, always in this order:
  unsafe justification, `unwrap`/`expect` discipline, parameterized SQL
  auditing, clippy cleanliness.
- **Unsafe justification.** Every `unsafe` block, `unsafe fn`, or `unsafe
  impl` in the diff needs an inline comment naming the invariant it upholds;
  an unjustified block is a BLOCK regardless of the workspace `unsafe_code =
  "forbid"` lint (the lint catches library code, not a relaxed allow).
- **No `unwrap`/`expect` in library paths.** `crates/<app>-core/`,
  `crates/<app>-ports/`, and `crates/<app>-widgets/` library code must return
  `Result`/`Option` and propagate with `?`; `unwrap`/`expect` are acceptable
  only in `#[cfg(test)]` blocks, `main.rs` entry points, or with an inline
  comment proving the invariant that makes the panic unreachable.
- **SQL auditing.** Every `sqlx::query` / `sqlx::query_as::<_, _>` /
  `sqlx::query_scalar` call in the diff binds values with `$1`/`$2` — a
  `format!`-interpolated value in a SQL string is a BLOCK. The queries are
  runtime calls with no compile-time schema check, so also compare the column
  list and bind order against the paired `.up.sql` file; drift is a BLOCK
  because only a run against a migrated database catches it.
- **Clippy cleanliness.** The diff must not introduce a `#[allow(clippy::...)]`
  that silences a pedantic or nursery lint without an inline justification
  comment; `cargo clippy --all-targets -- -D warnings` is the gate this
  category mirrors.

## Rules

- No fabricated data: do not invent file paths, line numbers, or macro
  invocations. If a file was not read this session, list it under "not
  reviewed" instead of guessing its contents.
- Every stat needs a source: a claim like "adapter X calls `.unwrap()` at
  line 40" needs the file/line actually read, or an explicit "unverified"
  tag.
- No code-fenced final output where raw output is expected: the verdict in
  Output Format below is raw structured prose, never a fenced block.
- Read each touched file before reporting on it; unread files are listed
  separately as "not reviewed (not read)".
- BLOCK only on a real finding: an unjustified `unsafe` block, a library-path
  `unwrap`/`expect` with no inline proof, an interpolated SQL value or a query
  whose columns/bind order do not match the migration, or a clippy `#[allow]`
  with no justification.
- WARN on style-grade issues (a justification comment that is thin but
  present, a runtime query with no adapter test behind it but no injection or
  drift risk in the diff).
- APPROVE only when all four categories show no BLOCK-level finding.

## Research References

Provenance table — fill claim/source/year/link from primary sources actually
read this session. Leave the placeholder row if a claim is unverified.

| Claim | Source | Year | Link |
|-------|--------|------|------|
| `<placeholder claim>` | `<source file or doc>` | `<year>` | `<link>` |
| `unsafe_code = "forbid"` workspace lint | `Cargo.toml` `[workspace.lints.rust]` | — | local file |
| Migration pairing convention | `migrations/*.up.sql` + `.down.sql` | — | local file |

## Output Format

Raw structured prose (not a code fence when rendered as the final verdict):

```
### <app> review — pre-landing checklist

Gate: just ci (fmt-check + clippy -D warnings + check + nextest)

Findings:
- [BLOCK|WARN] unsafe-justification @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] unwrap-expect @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] sql-audit @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] clippy @ <file>:<line> — <what> | fix: <suggestion>

Not reviewed (not read):
- <file>

Verdict: APPROVE | WARN | BLOCK
```
