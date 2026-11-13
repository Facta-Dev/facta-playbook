---
name: "learn"
description: "Records what the <app> iOS project learned: a repeated review finding, a corrected decision or a new toolchain fact, written as a durable memory entry with the rule and the source it came from."
division: core
---

# `<app>` learn skill

> Reusable workflow for turning a correction into a durable rule. An agent
> invokes this skill; the requester supplies the correction, the skill supplies
> the memory shape, the evidence requirement and the file target.

## When to Activate

Activate this skill when the same finding has been corrected twice, when a
decision was made and later reversed, when a toolchain fact was learned the
hard way (a Swift 6 concurrency diagnostic, an XcodeGen setting, a signing
failure with a non-obvious fix), or when a convention is being established
that the next session must not invent again.

Do NOT activate this skill when:

- The finding was a one-off typo — fix it and move on.
- The request is to write a plan (use `plan`) or review a diff (use `review`).
- Nothing was corrected; there is no rule to carry forward.

## Core Principles

- **A lesson needs a trigger.** The entry says when to apply it, not only what
  it says.
- **One rule per entry.** A memory file that lists ten rules is never read.
- **The source is part of the rule.** A fix that came from a build log, a
  review comment or an upstream document is cited.
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
- The entry names the toolchain it applies to (Swift 6, iOS 18, XcodeGen)
  when the version matters.
- The entry says what to do, not what went wrong: a rule, not a story.
- The entry is appended to `.claude/learnings.jsonl` in English, in the same
  shape as the existing entries.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The architecture rules a lesson must not contradict | `PLAYBOOK_MOBILE_IOS_SWIFT.md` | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |
| The agent kit's learning plane and where memory entries belong | `PLAYBOOK_CLAUDE_CODE.md` | - | `../PLAYBOOK_CLAUDE_CODE.md` |
| Swift 6 concurrency diagnostics are the recurring trap | Swift.org — Adopting Swift 6 | 2026 | https://www.swift.org/migration/documentation/swift-6-concurrency-migration-guide/ |
| The project file is generated, so build facts live in project.yml | XcodeGen — ProjectSpec | 2026 | https://github.com/yonaskolb/XcodeGen/blob/master/Docs/ProjectSpec.md |

## Output Format

```
Lesson: <one line, the rule>
Trigger: <when this applies>
Rule: <what to do instead>
Evidence: <file:line | command output | upstream link>
Scope: <package or file set it applies to>
Applies to: <toolchain version, when relevant>
Written to: .claude/learnings.jsonl
```
