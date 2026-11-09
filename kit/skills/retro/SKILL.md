---
name: retro
description: Triage the lessons inbox, deduplicate entries, and propose promotions into CLAUDE.md, rules, skills, or hooks. Use at the end of a work cycle.
when_to_use: "/retro", "review lessons", "promote recurring lessons", "weekly retro"
argument-hint: [days]
disable-model-invocation: true
---

# Retro

The inbox is `~/.claude/lessons/*.md`; each file uses
`~/.claude/templates/sdd/lessons.md` (date, topic, source, promoted, what
happened, why, how to apply).

## Procedure

1. List the lessons (optionally limited to the last `<days>`).
2. Group by topic and merge duplicates: identical cause and fix becomes one
   entry; keep the earliest date and note the recurrence count.
3. For each surviving lesson, pick the lowest rung that fixes it:
   - one-off → mark it and move on;
   - recurring preference → propose a `CLAUDE.md` bullet;
   - path- or stack-specific → propose a `rules/` file with `paths:`;
   - repeatable procedure → propose a skill;
   - must-hold-every-time → propose a hook (human-only, never edit directly).
4. Write one proposal to `~/.claude/proposals/<YYYY-MM-DD>-<slug>.md` with
   problem, evidence, proposed diff, risk, rollback.
5. Mark triaged lessons `promoted: true` in their frontmatter; leave the files
   in place.

## Done when

- Duplicates are merged, promoted lessons are flagged, and every actionable
  item has exactly one proposal. Report the counts.
