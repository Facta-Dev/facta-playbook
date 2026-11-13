---
name: "review"
description: "Reviews a <app> Android diff against the module map, the UDF contract, the Android platform rules and the security posture, and emits severity-prefixed findings with file:line anchors and the real gate output."
division: core
---

# `<app>` review skill

> Reusable workflow for reviewing a change in `<app>` before it lands. An agent
> invokes this skill; the requester supplies the diff scope, the skill supplies
> the lenses, the severity ladder and the report shape.

## When to Activate

Activate this skill when a change is ready for a second pair of eyes: a pull
request, a staged diff, a branch about to be merged, or a change that touches
Gradle modules, the manual DI graph in `:app`, the manifest, or the pinned
toolchain.

Do NOT activate this skill when:

- The change is still being written — review the plan instead.
- The request is a release step (use `ship`).
- The change is a single obvious fix; read the diff once and fix it.

## Core Principles

- **Read the whole diff first.** A partial read produces findings about code a
  later hunk already fixes.
- **Four lenses, in order:** correctness of the Kotlin and the coroutines, the
  module map and the UDF contract, the Android platform surface (lifecycle,
  permissions, manifest, background work), and the security posture.
- **Every finding is concrete.** A file, a line, the wrong behaviour and what
  the user sees. A category name is not a finding.
- **Severity is honest.** `critical` breaks the build, crashes, leaks a secret
  or loses data; `major` is a wrong behaviour with a workaround; `minor` is
  clarity or consistency.
- **Gates are run, not assumed.** The report carries the real output of
  `./gradlew :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test` and
  `bash scripts/check-architecture.sh`.
- **Silence is a valid outcome.** A clean diff produces a short report that
  says what was checked.

## Rules

- No fabricated line numbers, file paths, command output or test results: read
  the file or run the command.
- A finding that cannot be reproduced by a concrete input, state or schedule is
  dropped, not softened.
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
| The module map and the permitted dependency direction | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| State, events and effects are the Compose data-flow contract | Android Developers — Compose architecture | 2026 | https://developer.android.com/develop/ui/compose/architecture |
| Collection must be lifecycle-aware, not tied to composition | Android Developers — Compose lifecycle | 2026 | https://developer.android.com/develop/ui/compose/lifecycle |
| Background work has OS-imposed limits that the app cannot override | Android Developers — Background optimization | 2026 | https://developer.android.com/topic/performance/background-optimization |
| Exported components and the merged manifest are part of the attack surface | Android Developers — Manifest overview | 2026 | https://developer.android.com/guide/topics/manifest/manifest-intro |
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
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
review: N findings — verdict: APPROVE | REVISE
```
