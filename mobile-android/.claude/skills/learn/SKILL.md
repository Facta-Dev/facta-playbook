---
name: "learn"
description: "Records what the <app> Android project learned: a repeated review finding, a corrected decision or a new toolchain fact, written as a durable memory entry with the rule and the source it came from."
division: core
---

# `<app>` learn skill

> Reusable workflow for turning a correction into a durable rule. An agent
> invokes this skill; the requester supplies the correction, the skill supplies
> the memory shape, the evidence requirement and the file target.

## When to Activate

Activate this skill when the same finding has been corrected twice, when a
decision was made and later reversed, when a toolchain fact was learned the
hard way (a Gradle, AGP or Kotlin failure with a non-obvious fix), or when a
convention is being established that the next session must not invent again.

Do NOT activate this skill when:

- The finding was a one-off typo — fix it and move on.
- The request is to write a plan (use `plan`) or to review a diff
  (use `review`).
- Nothing was corrected; there is no rule to carry forward.

## Core Principles

- **A lesson needs a trigger.** The entry says when to apply it, not only what
  it says.
- **One rule per entry.** A memory file that lists ten rules is never read.
- **The source is part of the rule.** A fix that came from a build log, a
  review comment or an upstream document is cited; a rule invented from
  memory is not trusted.
- **Durable, not conversational.** The entry survives a month; it does not
  narrate the session that produced it.
- **No secrets, ever.** A memory file is a file in the repository.

## Rules

- Never store a secret, a token, a credential, a personal detail or a customer
  name in a memory entry — not masked, not partially, not by accident.
- Every entry cites its evidence: the command output, the file and line, the
  review comment or the upstream link.
- An entry is written after the second occurrence of the same finding, or
  after a decision that was explicitly corrected — never speculatively.
- The entry names the toolchain version it applies to (Gradle 9.6.0, AGP
  9.5.1, Kotlin 2.4.21, Compose BOM 2026.09.00) when the version matters.
- The entry says what to do, not what went wrong: a rule, not a story.
- The memory is written to the project's memory file, in English, in the same
  format as the existing entries.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The architecture rules a lesson must not contradict | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| The agent kit's learning plane and where memory entries belong | `PLAYBOOK_CLAUDE_CODE.md` | - | `../PLAYBOOK_CLAUDE_CODE.md` |
| Kotlin's own guidance on what is worth remembering as a rule | Kotlin coding conventions | 2026 | https://kotlinlang.org/docs/coding-conventions.html |
| Cancellation and structured concurrency are the recurring coroutine trap | Kotlin documentation | 2026 | https://kotlinlang.org/docs/cancellation-and-timeouts.html |
| Dependency verification makes the exact artifact set auditable | Gradle documentation | 2026 | https://docs.gradle.org/current/userguide/dependency_verification.html |

## Output Format

```
Lesson: <one line, the rule>
Trigger: <when this applies>
Rule: <what to do instead>
Evidence: <file:line | command output | upstream link>
Scope: <module or file set it applies to>
Applies to: <toolchain version, when relevant>
Written to: <memory file path>
```
