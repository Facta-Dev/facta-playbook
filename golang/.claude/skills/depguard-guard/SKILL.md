---
name: "<app> Depguard Guard"
description: "Review import boundaries in <app> for hexagonal soundness; flag every domain.go adapter import, slice-to-slice edge, kernel-only violation, otel import, and unsafe import, and block any relaxation of the depguard deny rules in .golangci.yml across the <scope> slice."
division: security
---

# <app> Depguard Guard

A reusable import-boundary review workflow for the `<app>` Go module. An agent (e.g. the depguard reviewer role) invokes this skill on any diff that touches `internal/**/*.go`, `go.mod`, or `.golangci.yml`. The module invariants are domain.go purity (stdlib + `internal/shared/` only), no slice-to-slice imports, `internal/shared/` imports stdlib only, otel is CUT, and `unsafe` is out of bounds — all encoded as depguard deny rules in `.golangci.yml`; the skill enforces them absolutely and reports any escape hatch.

## When to Activate

- A diff adds or modifies an import in a `domain.go` (`net/http`, `database/sql`, `github.com/jackc/pgx`, another slice's path).
- A diff edits `.golangci.yml`, or adds a depguard rule, a `//nolint:depguard`, or a per-slice rule block that touches the deny lists.
- A new dependency is introduced whose adapter surface reaches `<app>` in internal code.
- A reviewer asks for a boundary pass before approving a PR that touches the import or lint surface.

Do not activate for diffs with no import surface and no `.golangci.yml` edit.

## Core Principles

- The deny rules are absolute. They are not defaults to relax for a deadline; any edit that weakens a depguard deny is a BLOCK.
- The import line is the boundary. A `domain.go` that compiles with `net/http` inside is an adapter wearing a domain costume; the fix is moving the code across the layer boundary.
- Slices never import each other. A shared shape moves down to `internal/shared/` through an architect decision, never as a cross-slice edge.
- golangci-lint is the first reviewer. `golangci-lint run` is the gate; the skill catches what it cannot see — config edits that revoke rules, dependency surfaces, and review-time proposals.
- Cite the invariant, then the alternative. "BLOCK — domain.go purity" is the rule; "move it to ports.go / usecase.go" is the review.

## Rules

- Anti-hallucination: do not invent forbidden imports that are not in the diff. Every finding cites a path and line that exists in the tree.
- Every stat needs a source. If you cite the hexagonal dependency rules, name the doc; if you cite depguard's behavior, name the golangci-lint linter doc. No unsourced claims.
- Do not emit the final report as a fenced code block when raw line-anchored output is expected — the report is plain text, one finding per line, severity-prefixed.
- Verify by reading the cited file and line before reporting. A finding that cites an import the diff does not contain is a hallucination.
- Zero adapter imports in `internal/features/*/domain.go` is the floor; `golangci-lint run` enforces it via depguard. The skill enforces the config + dependency surface the linter cannot see.
- otel is CUT from this template (observability = slog JSON + /healthz + /readyz): any otel import in `internal/` or `go.mod` is a BLOCK.
- Zero `import "unsafe"` in internal source, and zero slice-to-slice imports, without an architect decision recorded in the PR.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| depguard denies imports by package and file-path pattern | golangci-lint documentation, depguard | 2024 | <TODO: primary doc link> |
| domain.go purity is the hexagon's core invariant | dependency-rules doc / hexagonal architecture | 2024 | <TODO: primary doc link> |
| Slices never import each other (vertical isolation) | dependency-rules doc | 2024 | <TODO: primary doc link> |
| golangci-lint run evaluates per-module config from .golangci.yml | golangci-lint documentation, configuration | 2024 | <TODO: primary doc link> |

## Output Format

A line-anchored report, one finding per line, severity-prefixed, followed by a summary line. Plain text — not a fenced block:

```
BLOCK  internal/features/widgets/domain.go:9  `import "net/http"` — domain.go purity violated; move it to the slice's http.go or ports.go.
BLOCK  internal/features/authsignup/usecase.go:12  `import ".../internal/features/widgets"` — slices never import each other; route the shared shape to internal/shared/ via the architect.
WARN   .golangci.yml:41  depguard deny rule for net/http in domain.go patterns removed — boundary revoked; route to architect.
PASS   internal/features/widgets/postgres.go  no forbidden imports; the adapter layer is exactly where pgx belongs.
```

Summary line: `depguard-guard: N block(s), M warning(s), K pass — verdict: BLOCK | APPROVE`. Any non-zero BLOCK is an automatic BLOCK verdict.