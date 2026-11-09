---
name: "<app> Depguard Guard"
description: "Review import boundaries in <app> for hexagonal soundness; flag every domain.ts adapter import, slice-to-slice edge, kernel-only violation, otel import, and dynamic-execution import, and block any relaxation of the no-restricted-imports groups in eslint.config.mjs across the <scope> slice."
division: security
---

# <app> Depguard Guard

A reusable import-boundary review workflow for the `<app>` codebase. An agent (e.g. the depguard reviewer role) invokes this skill on any diff that touches `src/**/*.ts`, `package.json`, or `eslint.config.mjs`. The module invariants are domain.ts purity (node builtins + `#shared/` only), no slice-to-slice imports, `#shared/` imports node builtins only, otel is CUT, and dynamic execution (`node:vm`, `eval`, `new Function`) is out of bounds — all encoded as no-restricted-imports groups in `eslint.config.mjs`; the skill enforces them absolutely and reports any escape hatch.

## When to Activate

- A diff adds or modifies an import in a `domain.ts` (`fastify`, `pg`, `kysely`, `#gen/*`, another feature's path) — or any transport/persistence import in what claims to be domain code.
- A diff edits `eslint.config.mjs`, or loosens a `no-restricted-imports` group, or adds an override that touches the deny lists.
- A new dependency is introduced whose adapter surface reaches `<app>` in src code.
- A reviewer asks for a boundary pass before approving a PR that touches the import or lint surface.

Do not activate for diffs with no import surface and no `eslint.config.mjs` edit.

## Core Principles

- The deny rules are absolute. They are not defaults to relax for a deadline; any edit that weakens a no-restricted-imports group is a BLOCK.
- The import line is the boundary. A `domain.ts` that carries `fastify` inside is an adapter wearing a domain costume; the fix is moving the code across the layer boundary.
- Slices never import each other. A shared shape moves down to `#shared/` through an architect decision, never as a cross-slice edge.
- eslint is the first reviewer. `pnpm lint` (eslint) is the gate this category mirrors; the skill catches what the linter cannot see — config edits that revoke rules, dependency surfaces, and review-time proposals.
- Cite the invariant, then the alternative. "BLOCK — domain.ts purity" is the rule; "move it to the slice's http.ts or ports.ts" is the review.

## Rules

- Anti-hallucination: do not invent forbidden imports that are not in the diff. Every finding cites a path and line that exists in the tree.
- Every stat needs a source. If you cite the hexagonal dependency rules, name the doc; if you cite the linter's behavior, name the ESLint rule doc (`no-restricted-imports`). No unsourced claims.
- Do not emit the final report as a fenced code block when raw line-anchored output is expected — the report is plain text, one finding per line, severity-prefixed.
- Verify by reading the cited file and line before reporting. A finding that cites an import the diff does not contain is a hallucination.
- Zero adapter imports in `src/features/*/domain.ts` is the floor; eslint enforces it via the domain-pure no-restricted-imports group in `eslint.config.mjs`. The skill enforces the config + dependency surface the linter cannot see.
- otel is CUT from this template (observability = Fastify's built-in JSON logger + /healthz + /readyz): any otel import in `src/` or `package.json` is a BLOCK.
- Zero dynamic-execution imports (`node:vm`, `eval`, `new Function`) in src code, and zero slice-to-slice imports, without an architect decision recorded in the PR.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim                                                           | source                                        | year | link                     |
| --------------------------------------------------------------- | --------------------------------------------- | ---- | ------------------------ |
| no-restricted-imports denies imports by module name and pattern | ESLint documentation, no-restricted-imports   | 2024 | <TODO: primary doc link> |
| domain.ts purity is the hexagon's core invariant                | dependency-rules doc / hexagonal architecture | 2024 | <TODO: primary doc link> |
| Slices never import each other (vertical isolation)             | dependency-rules doc                          | 2024 | <TODO: primary doc link> |
| eslint flat config evaluates from eslint.config.mjs             | ESLint flat config documentation              | 2024 | <TODO: primary doc link> |

## Output Format

A line-anchored report, one finding per line, severity-prefixed, followed by a summary line. Plain text — not a fenced block:

```
BLOCK  src/features/widgets/domain.ts:9  `import fastify` — domain.ts purity violated; move it to the slice's http.ts or ports.ts.
BLOCK  src/features/authsignup/usecase.ts:12  `#features/widgets` — slices never import each other; route the shared shape to #shared/ via the architect.
WARN   eslint.config.mjs:41  the domain-pure no-restricted-imports group dropped `pg` — boundary revoked; route to architect.
PASS   src/features/widgets/postgres.ts  no forbidden imports; the adapter layer is exactly where pg belongs.
```

Summary line: `depguard-guard: N block(s), M warning(s), K pass — verdict: BLOCK | APPROVE`. Any non-zero BLOCK is an automatic BLOCK verdict.
