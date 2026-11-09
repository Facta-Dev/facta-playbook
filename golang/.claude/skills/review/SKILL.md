---
name: "review"
description: "Runs the <app> pre-landing review checklist across unsafe-import justification, error-shadowing discipline, sqlc query auditing, and golangci-lint cleanliness, producing a severity-ranked gate verdict against `make ci`."
division: core
---

# <app> review skill

> Reusable pre-landing review CHECKLIST for the `<app>` Go module. An
> agent invokes this skill; the agent supplies contextual judgment, the skill
> supplies the deterministic procedure.

## When to Activate

Activate this skill before landing any change to `internal/**/*.go`,
`cmd/**/*.go`, or `migrations/`, or whenever a request asks for a "review",
"gate check", or "pre-landing pass" on `<app>`. Also trigger on:

- A pull request touching a handler, repository adapter, or domain module.
- A request to confirm a change is safe to merge against `make ci`.
- A follow-up review after fixes from a prior `/review` pass.

Do NOT activate for documentation-only or `go.sum`-only changes with no
`.go` or `go.mod` edits. This skill COMPLEMENTS the `code-reviewer` core
agent rather than replacing it: the agent reasons freely about a diff's
intent and tradeoffs, while this skill walks the same four stack categories
in the same order every time. Do not restate the agent's prose here — the
two are consulted together, not merged.

## Core Principles

- **Checklist, not judgment.** Four fixed categories, always in this order:
  unsafe-import justification, error-handling discipline, sqlc query
  auditing, golangci-lint cleanliness.
- **No `unsafe` in internal source.** Every `import "unsafe"` (or
  `reflect.SliceHeader`-style memory reinterpretation) in the diff needs an
  inline comment naming the invariant it upholds; an unjustified import is a
  BLOCK regardless of what the tooling allows.
- **No swallowed errors in library paths.** `internal/shared/`,
  `internal/platform/`, and `internal/features/*/usecase.go` must return
  errors that wrap a stable `apperr` code and propagate; an ignored error
  (`_, _ :=` or a bare `_ =` on a fallible call) is acceptable only in
  `*_test.go`, `cmd/api/main.go` shutdown paths, or with an inline comment
  proving the invariant that makes ignoring it safe.
- **sqlc query auditing.** Every query added or changed in a slice's
  `queries.sql` is checked against the migration it targets: column names,
  nullability, and parameter binding order must match the paired `.up.sql`
  file; a hand-built runtime SQL string bypassing the committed
  `internal/gen/db` output is flagged.
- **golangci-lint cleanliness.** The diff must not introduce a `//nolint:`
  that silences a depguard/errcheck/errname lint without an inline
  justification comment; `golangci-lint run` is the gate this category
  mirrors.

## Rules

- No fabricated data: do not invent file paths, line numbers, or query
  invocations. If a file was not read this session, list it under "not
  reviewed" instead of guessing its contents.
- Every stat needs a source: a claim like "adapter X ignores the scan error
  at line 40" needs the file/line actually read, or an explicit "unverified"
  tag.
- No code-fenced final output where raw output is expected: the verdict in
  Output Format below is raw structured prose, never a fenced block.
- Read each touched file before reporting on it; unread files are listed
  separately as "not reviewed (not read)".
- BLOCK only on a real finding: an unjustified `unsafe` import, a
  library-path ignored error with no inline proof, a queries.sql entry whose
  bound parameters do not match the migration, or a `//nolint:` with no
  justification.
- WARN on style-grade issues (a justification comment that is thin but
  present, a runtime query that could become a sqlc query but is not itself
  unsound).
- APPROVE only when all four categories show no BLOCK-level finding.

## Research References

Provenance table — fill claim/source year, link from primary sources actually
read this session. Leave the placeholder row if a claim is unverified.

| Claim | Source | Year | Link |
|-------|--------|------|------|
| `<placeholder claim>` | `<source file or doc>` | `<year>` | `<link>` |
| depguard import rules | `.golangci.yml` | — | local file |
| Migration pairing convention | `migrations/*.up.sql` + `.down.sql` | — | local file |

## Output Format

Raw structured prose (not a code fence when rendered as the final verdict):

```
### <app> review — pre-landing checklist

Gate: make ci (gofmt-check + go vet + golangci-lint + go test ./...)

Findings:
- [BLOCK|WARN] unsafe-import @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] error-discipline @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] sqlc-query-audit @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] golangci @ <file>:<line> — <what> | fix: <suggestion>

Not reviewed (not read):
- <file>

Verdict: APPROVE | WARN | BLOCK
```