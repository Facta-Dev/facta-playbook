---
name: "review"
description: "Reviews a <app> change against the desktop layer law, the Widgets-first rule, the GUI-thread rule, the secret policy and the gate results, and reports severity-prefixed findings with a verdict."
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

- **The law first, the style last.** Layer violations, GUI-thread blocking, a
  QML surface without an ADR and a leaked secret outrank naming and layout.
- **Evidence over impression.** A finding names a file, a line and the
  concrete failure it causes. A preference is not a finding.
- **The gates decide.** The review runs `make ci` (or the narrowest gate that
  covers the change) and reports the real output; a diff that fails a gate is
  never approved.
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
- The checks, in order: layer law, GUI-thread law, Widgets-first, secrets and
  privacy, storage and migrations, error handling, tests, style.
- The summary line is exactly `review: N findings — verdict: APPROVE | REVISE`.
- A missing test for a new behaviour is a `major` finding, not a nit.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The layer law and the link-time enforcement | `PLAYBOOK_DESKTOP_QT.md` §3 | - | `PLAYBOOK_DESKTOP_QT.md` |
| Widgets-first and the Compatibility Mode fallback | `PLAYBOOK_DESKTOP_QT.md` §2 | - | `PLAYBOOK_DESKTOP_QT.md` |
| The secrets policy and the keystore requirement | `PLAYBOOK_SECURITY.md` | - | `PLAYBOOK_SECURITY.md` |
| The gate ladder: configure, build, offscreen ctest, evals | `PLAYBOOK_TESTS.md` | - | `PLAYBOOK_TESTS.md` |
| `ctest` runs the registered suites and reports their result | CMake documentation | 2025 | https://cmake.org/cmake/help/latest/manual/ctest.1.html |

## Output Format

```
Review of: <diff or files>
Findings:
  [critical] <file>:<line> — <defect> — <failure scenario>
  [major]    <file>:<line> — <defect> — <failure scenario>
  [minor]    <file>:<line> — <defect> — <failure scenario>
Checked and passing: <layer law · threading · widgets-first · secrets · storage · tests>
Gates run: <command> → <result>
review: N findings — verdict: APPROVE | REVISE
```
