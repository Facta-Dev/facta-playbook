---
name: learn
description: "Delegate to the learn skill — record a repeated review finding, a corrected decision or a new toolchain fact as a durable memory entry with its trigger, its rule and its evidence."
division: core
---

# /learn

## Delegate

Delegate to the **learn** skill. Activate after the second occurrence of the
same finding, after a decision was explicitly corrected, or when a toolchain
fact was learned the hard way.

Steps:

1. State the lesson as a rule: what to do, not what went wrong.
2. State the trigger: when the rule applies.
3. Cite the evidence: the command output, the `file:line`, the review comment
   or the upstream link.
4. Name the toolchain when the version matters (Swift 6, iOS 18, XcodeGen).
5. Name the scope: the package or file set the rule covers.
6. Check that no secret, token, credential or personal detail is present, not
   even masked.
7. Append the entry to `.claude/learnings.jsonl` in the same shape as the
   existing entries, in English.
