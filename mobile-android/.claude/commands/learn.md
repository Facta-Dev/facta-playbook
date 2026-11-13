---
name: learn
description: "Delegate to the learn skill — record a durable rule for the <app> Android project after a repeated review finding or a corrected decision, with the evidence that produced it."
division: core
---

# /learn

## Delegate

Delegate to the **learn** skill. Activate when the same finding has been
corrected twice, when a decision was reversed, or when a toolchain fact was
learned the hard way and must not be relearned.

Steps:

1. Confirm there is something to learn: a rule that changes future behaviour,
   not a one-off fix.
2. Write the rule as one line with its trigger — when it applies, not only
   what it says.
3. Attach the evidence: the `file:line`, the command output, the review
   comment or the upstream document the rule came from.
4. Name the scope (the module or file set it applies to) and the toolchain
   version it depends on (Gradle 9.6.0, AGP 9.5.1, Kotlin 2.4.21, Compose
   BOM 2026.09.00) when the version matters.
5. Write the entry to the project memory file in English, and report the
   path. A secret, token or personal detail never enters the entry.
