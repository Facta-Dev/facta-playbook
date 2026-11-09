---
name: "review"
description: "Runs the <app> pre-landing review checklist across dynamic-execution justification, error-shadowing discipline, queries.ts auditing, and eslint cleanliness, producing a severity-ranked gate verdict against `make ci`."
division: core
---

# <app> review skill

> Reusable pre-landing review CHECKLIST for the `<app>` TypeScript codebase. An
> agent invokes this skill; the agent supplies contextual judgment, the skill
> supplies the deterministic procedure.

## When to Activate

Activate this skill before landing any change to `src/**/*.ts` or
`migrations/`, or whenever a request asks for a "review", "gate check", or
"pre-landing pass" on `<app>`. Also trigger on:

- A pull request touching a handler, repository adapter, or domain module.
- A request to confirm a change is safe to merge against `make ci`.
- A follow-up review after fixes from a prior `/review` pass.

Do NOT activate for documentation-only or lockfile-only changes with no
`.ts` or `package.json` edits. This skill COMPLEMENTS the `code-reviewer` core
agent rather than replacing it: the agent reasons freely about a diff's
intent and tradeoffs, while this skill walks the same four stack categories
in the same order every time. Do not restate the agent's prose here — the
two are consulted together, not merged.

## Core Principles

- **Checklist, not judgment.** Four fixed categories, always in this order:
  dynamic-execution justification, error-handling discipline, queries.ts
  auditing, eslint cleanliness.
- **No dynamic execution in src source.** Every use of `node:vm`, `eval`, or
  `new Function` (or a Function-constructor reinterpretation) in the diff
  needs an inline comment naming the invariant it upholds; an unjustified
  dynamic-execution call is a BLOCK regardless of what the tooling allows.
- **No swallowed errors in library paths.** `src/shared/` and `src/platform/`
  and `src/features/*/usecase.ts` must return errors that wrap a stable
  `apperr` code and propagate; an ignored promise (`void run()` on a fallible
  call, or a bare `catch {}`) is acceptable only in `*.test.ts`,
  `src/bin/api.ts` shutdown paths, or with an inline comment proving the
  invariant that makes ignoring it safe.
- **queries.ts auditing.** Every query added or changed in a slice's
  `queries.ts` is checked against the migration it targets: column names and
  nullability must match the paired migration's `up`; a hand-built runtime SQL string (template-
  literal interpolation of values into SQL) bypassing the committed
  `queries.ts` is flagged.
- **eslint cleanliness.** The diff must not introduce an `// eslint-disable`
  that silences a no-restricted-imports/@typescript-eslint rule without an
  inline justification comment; `pnpm lint` (eslint) is the gate this
  category mirrors.

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
- BLOCK only on a real finding: an unjustified dynamic-execution call, a
  library-path ignored-promise with no inline proof, a queries.ts entry whose
  SQL interpolates values or whose columns do not match the migration, or an
  `// eslint-disable` with no justification.
- WARN on style-grade issues (a justification comment that is thin but
  present, a runtime query that could become a queries.ts entry but is not
  itself unsound).
- APPROVE only when all four categories show no BLOCK-level finding.

## Research References

Provenance table — fill claim/source year, link from primary sources actually
read this session. Leave the placeholder row if a claim is unverified.

| Claim                                                  | Source                               | Year     | Link       |
| ------------------------------------------------------ | ------------------------------------ | -------- | ---------- |
| `<placeholder claim>`                                  | `<source file or doc>`               | `<year>` | `<link>`   |
| restricted-imports + domain-pure deny groups           | `eslint.config.mjs`                  | —        | local file |
| Migration pairing convention (non-empty `down` export) | `migrations/*_*.mjs` node-pg-migrate | —        | local file |

## Output Format

Raw structured prose (not a code fence when rendered as the final verdict):

```
### <app> review — pre-landing checklist

Gate: make ci (prettier format:check + tsc vet + eslint + openapi-typescript gen-check + vitest)

Findings:
- [BLOCK|WARN] dynamic-execution @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] error-discipline @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] query-audit @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] eslint @ <file>:<line> — <what> | fix: <suggestion>

Not reviewed (not read):
- <file>

Verdict: APPROVE | WARN | BLOCK
```
