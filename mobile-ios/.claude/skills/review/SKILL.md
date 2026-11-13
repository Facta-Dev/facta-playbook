---
name: "review"
description: "Reviews a <app> iOS diff against the package map, the UDF contract, the Apple platform rules and the security posture, and emits severity-prefixed findings with file:line anchors and the real gate output."
division: core
---

# `<app>` review skill

> Reusable workflow for reviewing a change in `<app>` before it lands. An agent
> invokes this skill; the requester supplies the diff scope, the skill supplies
> the lenses, the severity ladder and the report shape.

## When to Activate

Activate this skill when a change is ready for a second pair of eyes: a pull
request, a staged diff, a branch about to be merged, or a change that touches
`Package.swift`, the composition root, `project.yml`, the app's
`Info.plist` keys or the pinned Swift 6 toolchain.

Do NOT activate this skill when:

- The change is still being written — review the plan instead.
- The request is a release step (use `ship`).
- The change is a single obvious fix; read the diff once and fix it.

## Core Principles

- **Read the whole diff first.** A partial read produces findings about code a
  later hunk already fixes.
- **Four lenses, in order:** correctness of the Swift and the concurrency, the
  package map and the UDF contract, the Apple platform surface (lifecycle,
  capabilities, privacy manifest, background work), and the security posture.
- **Every finding is concrete.** A file, a line, the wrong behaviour and what
  the user sees. A category name is not a finding.
- **Severity is honest.** `critical` breaks the build, crashes, leaks a secret
  or loses data; `major` is a wrong behaviour with a workaround; `minor` is
  clarity or consistency.
- **Gates are run, not assumed.** The report carries the real output of
  `bash scripts/check-architecture.sh`, `node --test
  evals/contracts.test.mjs` and `make ci`; native gates are reported as run or
  SKIP with the reason.
- **Silence is a valid outcome.** A clean diff produces a short report that
  says what was checked.

## Rules

- No fabricated line numbers, file paths, command output or test results: read
  the file or run the command.
- A finding that cannot be reproduced by a concrete input, state or schedule
  is dropped, not softened.
- The review never rewrites the code; it reports.
- Findings name the root cause when several call sites share the same bad
  helper.
- The report states which lenses produced no findings, so the coverage is
  visible.
- A skipped or weakened test is a `critical` finding, never a `minor` one.
- Secrets are named by location only; a secret value never enters the report.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The package map and the permitted dependency direction | `PLAYBOOK_MOBILE_IOS_SWIFT.md` §4 | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |
| State, events and effects are the SwiftUI data-flow contract | Apple — Managing model data | 2026 | https://developer.apple.com/documentation/swiftui/managing-model-data-in-your-app |
| Concurrency is checked by the compiler in Swift 6 mode | Swift.org — Concurrency | 2026 | https://docs.swift.org/swift-book/documentation/the-swift-programming-language/concurrency/ |
| Capabilities and privacy are declared in the bundle | Apple — Privacy manifest files | 2026 | https://developer.apple.com/documentation/bundleresources/privacy-manifest-files |
| Mobile storage and authentication requirements | OWASP MASVS | 2026 | https://mas.owasp.org/MASVS/ |

## Output Format

```
Review of: <files or diff>
Findings:
  [critical] <file>:<line> — <what breaks> — <symptom>
  [major]    <file>:<line> — ...
  [minor]    <file>:<line> — ...
Checked and passing: <lenses and rules with no finding>
Gates:
  bash scripts/check-architecture.sh -> <result>
  node --test evals/contracts.test.mjs -> <result>
  make ci -> <result>
  native gates -> <result or SKIP: reason>
review: N findings — verdict: APPROVE | REVISE
```
