---
name: kit-audit
description: Audit the user-level Claude Code kit — validator findings, drift against enabled plugins, and context budgets — and write proposals for what needs fixing. Use after kit changes or on a schedule.
when_to_use: "/kit-audit", "audit ~/.claude", "check kit health", "plugin drift"
disable-model-invocation: true
---

# Kit audit

## Procedure

1. Run `python3 ~/.claude/scripts/kit_check.py --json` and capture the
   summary plus findings.
2. Read `~/.claude/settings.json.enabledPlugins` and
   `~/.claude/plugins/installed_plugins.json`; compare each plugin's agents,
   commands, and skills with the user copies.
3. Report in four buckets:
   - **errors** — invalid frontmatter, duplicate names, secrets, missing hook
     files;
   - **drift** — user copies that duplicate or lag an enabled plugin;
   - **budget** — always-loaded rule bytes, agent description total, skills
     listing size;
   - **stale** — files under `backups/`, orphaned hooks, unused templates.
4. For each fix that touches agents, skills, rules, or docs, write a proposal
   in `~/.claude/proposals/<YYYY-MM-DD>-<slug>.md` (problem, evidence,
   proposed diff, risk, rollback).
5. Never edit `hooks/`, `settings*.json`, `scripts/`, or permissions. List
   those as human-only items.

## Done when

- The report covers all four buckets with concrete paths and counts.
- Every actionable item has a proposal or an explicit "accepted as-is" note.
