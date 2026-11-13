---
name: "review"
description: "Reviews a <app> mobile change against the layer law, the Result contract, the eight view-model statuses, the route grammar, the secret policy and the gate results, and reports severity-prefixed findings with a verdict."
division: core
---

# `<app>` review skill

> Reusable workflow for reviewing a change before it lands. An agent invokes
> this skill; the skill supplies the checklist, the order of the checks and
> the shape of the report.

## When to Activate

Activate this skill when a change is ready for review: a pull request, a
local diff, a branch that another session wrote.

Do NOT activate this skill when:

- The request is to write the change — implement it, with a red test first.
- The request is to fix a known failing test — use the failing output as the
  input to the fix, not the review.
- There is no diff to review.

## Core Principles

- **The law first, the style last.** A `react-native` or `expo-*` import
  outside `adapters/out/`, a thrown error where a `Result` belongs, a screen
  that skips a status, a hand-written path and a leaked secret outrank naming
  and layout.
- **Evidence over impression.** A finding names a file, a line and the
  concrete failure it causes. A preference is not a finding.
- **The gates decide.** The review runs `pnpm run ci` (or the narrowest gate
  that covers the change) and reports the real output; a diff that fails a
  gate is never approved.
- **Root cause, not symptom.** When several call sites share the mistake, the
  finding points at the shared function.
- **Report the clean areas too.** Silence must not be mistaken for "not
  looked at".

## Rules

- One line per finding, prefixed with the severity: `critical`, `major`,
  `minor`. No prose paragraphs, no code fences around the finding list.
- Every finding names a file and a line, and states the scenario in which it
  fails.
- No fabricated finding: if a rule passes, it is listed as checked; if a file
  was not read, it is not reviewed.
- The checks, in order: layer law and platform confinement, the `Result`
  contract, the eight statuses, the route grammar, secrets and device data,
  the low-tier performance posture, tests, style.
- The summary line is exactly `review: N findings — verdict: APPROVE | REVISE`.
- A missing test for a new behaviour is a `major` finding, not a nit.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The layer law and the device-as-adapter rule | `PLAYBOOK_FRONTEND_MOBILE.md` §4.1 | 2026 | local file |
| The import rules the linter and the architecture test enforce | `eslint.config.mjs`, `tests/_platform/architecture/import-boundaries.test.ts` | 2026 | local file |
| The `Result<T, E>` contract every boundary returns | `packages/core/src/result.ts` | 2026 | local file |
| The eight statuses of the view-model contract | `packages/feature-notes/src/viewmodels/notes-view-model.ts` | 2026 | local file |
| The route grammar and the untrusted-input rule | `packages/contracts/src/routes.ts` | 2026 | local file |
| The gate ladder that must be green | `package.json`, `Makefile` | 2026 | local file |
| The mobile security baseline the review checks against | OWASP Mobile Application Security Verification Standard | 2026 | https://mas.owasp.org/MASVS/ |
| The accessibility bar a screen must clear | WCAG 2.2 | 2026 | https://www.w3.org/TR/WCAG22/ |

## Output Format

```
Review of: <diff or files>
Findings:
  [critical] <file>:<line> — <defect> — <failure scenario>
  [major]    <file>:<line> — <defect> — <failure scenario>
  [minor]    <file>:<line> — <defect> — <failure scenario>
Checked and passing: <layer law · Result · statuses · routes · secrets · performance · tests>
Gates run: <command> → <result>
review: N findings — verdict: APPROVE | REVISE
```
