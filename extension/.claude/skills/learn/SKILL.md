---
name: "<app> Learn"
description: "Appends a one-line lesson to the <app> extension's project-local .claude/learnings.jsonl and shows how to consult the prior entries before repeating similar work."
division: core
---

# <app> Learn

A reusable workflow for capturing a durable, project-local lesson learned while working on the `<app>` Chrome extension. An agent invokes this skill; the agent supplies the lesson, the skill supplies the append and consult procedure. The log is append-only and lives in the repository, so it must carry only non-sensitive facts.

## When to Activate

- A session surfaces a lesson worth remembering for `<app>`: a recurring pitfall, a pattern that worked, or a team preference discovered mid-task.
- A bug is traced to a root cause that would recur if unrecorded — for example a storage version bumped without a migration, or a handler that parsed before validating the sender.
- A workaround should become the default approach next time.
- A reviewer states a preference that should persist beyond this session.

Do not activate for one-off facts with no recurrence value, and never for anything resembling a secret, a credential, a token or a customer value — the log is committed to the project and is not a secret store.

## Core Principles

- **Append-only.** `.claude/learnings.jsonl` is never rewritten or reordered; each invocation adds exactly one line at the end. Never parse-and-rewrite the prior lines.
- **Consult before you repeat.** Before starting work that resembles a past pitfall class — a storage migration, a protocol change, a permission grant, a boundary edge — grep the log for the relevant `kind` first.
- **Deliberately local.** The log belongs to this repository only; it does not sync to, read from, or depend on any external store, and a fresh clone starts with no entries until the first `/learn` call.
- **One lesson, one line.** A JSONL entry is a single fact, not a running narrative; three lessons are three lines.
- **Created on first use, not shipped.** The template ships no `.claude/learnings.jsonl`; the file is created by the first successful append.

## Rules

- No fabricated data: only append a lesson actually observed in this session — never invent a plausible-sounding pitfall to pad the log.
- Every claim needs a source: the `refs` array cites a real `file:line`, a message type, a version constant or an issue reference from this session; an empty `refs` array is acceptable only for a pure preference statement.
- No code-fenced final output where raw output is expected: report the appended line as raw text, so it is easy to copy-verify against the file.
- Append with plain `printf`, never with a JSON-mutating tool that could reformat or reorder the prior lines.
- Escape embedded double quotes in `note` before appending, or the line will not parse as valid JSON.
- Never append a password, token, key, cookie value or connection string — reference the file and line where the issue lives instead.
- The `kind` is one of `pattern`, `pitfall` or `preference`, and it is what the consult step greps on.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| JSON Lines format — one JSON value per line | jsonlines.org | 2024 | https://jsonlines.org |
| The log is project-local and is never synced anywhere | `PLAYBOOK_CLAUDE_CODE.md` | 2024 | local file |
| The learning categories used by the kit: pattern, pitfall, preference | `PLAYBOOK_CLAUDE_CODE.md` | 2024 | local file |
| The lessons referenced by `AGENTS.md` before repeating similar work | `AGENTS.md` | 2024 | local file |

## Output Format

Raw structured prose, not a code fence. The appended line is printed verbatim so the caller can copy-verify it against the file, and the consult reminder names the exact grep to run before starting work that resembles the recorded lesson:

```
### <app> learn — entry appended

File: .claude/learnings.jsonl
Line: {"ts":"<ISO-8601 UTC>","kind":"pattern|pitfall|preference","note":"<one-line lesson>","refs":["<file:line | issue# | url>"]}

Consult-first reminder: grep '"kind":"<kind>"' .claude/learnings.jsonl before
starting similar work.
```
