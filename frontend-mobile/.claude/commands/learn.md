---
name: learn
description: "Delegate to the learn skill — append one durable lesson to <app>'s project-local .claude/learnings.jsonl and consult the prior entries before repeating similar work."
division: core
---

# /learn

## Delegate

Delegate to the **learn** skill. Activate when a session surfaces a lesson
worth keeping for `<app>`: a recurring pitfall, a pattern that worked, or a
preference that should outlive the session.

Steps:

1. Consult first: grep `.claude/learnings.jsonl` for the same `kind` and read
   the entries before writing a new one.
2. Reduce the lesson to one fact, with the real `file:line`, issue number or
   URL that proves it.
3. Check the note against the secret rules — no token, key, password or
   connection string ever reaches the log.
4. Append exactly one line with `printf` to `.claude/learnings.jsonl`; the
   file is never rewritten or reordered.
5. Report the appended line as raw text so it can be verified against the
   file.
