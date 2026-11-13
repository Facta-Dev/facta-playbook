---
name: "learn"
description: "Appends a one-line lesson to <app>'s project-local .claude/learnings.jsonl and shows how to consult prior entries before repeating similar work."
division: core
---

# `<app>` learn skill

> Reusable workflow for capturing a durable, project-local lesson learned
> while working on `<app>`. An agent invokes this skill; the agent supplies
> the lesson, the skill supplies the append and the consult procedure.

## When to Activate

Activate this skill when a session surfaces a lesson worth remembering for
`<app>`: a recurring pitfall, a pattern that worked, or a preference that
should outlive the session. Typical triggers:

- A bug traced to a root cause that will recur if unrecorded (a port that
  leaks a device type into the domain, a view that imports a repository, a
  route written by hand instead of built).
- A workaround that should become the default approach next time (the Expo
  export flag that avoids a Metro resolution failure).
- A preference stated by a reviewer or the requester that should persist.

Do NOT activate this skill for one-off facts with no recurrence value, or to
record anything resembling a secret, a credential or a connection string —
the log is committed to the project and is not a secret store.

## Core Principles

- **Append-only.** `.claude/learnings.jsonl` is never rewritten or
  reordered; each invocation adds exactly one line at the end. Never
  parse-and-rewrite the prior lines.
- **Consult before repeating.** Before starting work that resembles a past
  pitfall class (a new slice, a new port, a new route, a dependency change),
  grep `.claude/learnings.jsonl` for the relevant `kind` first.
- **One lesson, one line.** A JSONL entry is a single fact, not a narrative;
  three lessons produce three lines.
- **Local to this project.** The log stays inside the generated `<app>`
  repository: it never syncs anywhere, and a fresh clone starts empty until
  the first append.
- **Created on first use.** The template ships no
  `.claude/learnings.jsonl`; the file appears with the first successful
  append.

## Rules

- No fabricated data: only append a lesson actually observed in this session;
  never invent a plausible pitfall to pad the log.
- Every claim needs a source: the `refs` array cites a real `file:line`, an
  issue number or a URL from this session. An empty `refs` array is
  acceptable only for a pure preference statement.
- Never append a password, token, key or connection string value — cite the
  file and line where the problem lives instead.
- Append with pure shell, no JSON rewriter:
  `printf '%s\n' "{…}" >> .claude/learnings.jsonl`. A tool that reformats the
  file would reorder the history.
- Escape the double quotes inside the `note` string, or the line will not
  parse as JSON.
- Report the appended line as raw text, not inside a code fence, so it can be
  verified against the file.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| JSON Lines format (one JSON value per line) | jsonlines.org | 2026 | https://jsonlines.org |
| The recurring pitfalls worth recording: layer law, ports, routes, gates | `PLAYBOOK_FRONTEND_MOBILE.md` §4 | 2026 | local file |
| The gate that turns a lesson into a test | `PLAYBOOK_TESTS.md` | 2026 | local file |
| The learning categories used by the kit: pattern, pitfall, preference | `PLAYBOOK_CLAUDE_CODE.md` | 2026 | local file |

## Output Format

Raw structured prose, not a code fence:

```
### <app> learn — entry appended

File: .claude/learnings.jsonl
Line: {"ts":"<ISO-8601 UTC>","kind":"pattern|pitfall|preference","note":"<one-line lesson>","refs":["<file:line | issue# | url>"]}

Consult-first reminder: grep '"kind":"<kind>"' .claude/learnings.jsonl before
starting similar work.
```
