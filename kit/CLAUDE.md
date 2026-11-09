## Language

Every artifact is English: code, identifiers, comments, docstrings, tests,
specs, commit messages, agent and skill definitions, documentation. Non-English
text appears only in user-facing localization. State this once in each repo's
`CLAUDE.md`. The repo-level `CLAUDE.md` carries the project-specific commands.

## Model tiers (Ollama)

Aliases resolve through `ANTHROPIC_DEFAULT_*` (settings.json): `opus` =
escalation (`deepseek-v4-pro:cloud`), `sonnet` = workhorse
(`glm-5.3-flash:cloud`), `haiku` = cheap digging (`minimax-m3.1-flash:cloud`).
A bare `claude` must stay on `sonnet`.

- Escalate with `/model opus` for design work or after the same solution failed
  twice; return to `sonnet` when done.
- Pass `model` explicitly to subagents: haiku for search and mechanical work,
  sonnet for implementation, opus for architecture and deep review.

## Delegation and cost

- Consult the `advisor` agent before locking a plan, when the same error
  reappears twice, and before declaring a task done. Pass concrete context
  (plan, diff, test output), never "investigate the repo".
- Check spend with `/cccost` or `/ecc:cost-report` when a session feels heavy;
  prefer one focused session per topic over one long mixed one.

## Memory hygiene

- Auto memory lives in `projects/<project>/memory/`: one fact per file plus one
  `MEMORY.md` index line. Save preferences, constraints, and pointers — not
  code structure or git history.
- Durable lessons go to `~/.claude/lessons/`; kit changes go to
  `~/.claude/proposals/` and need human approval.
- Never store secrets, tokens, or credentials in memory, docs, or settings.

## Multi-session etiquette

One topic per session; write shared state into the repo (specs, ADRs, plan
files) instead of relying on transcript memory. Re-read a file before editing
when another session may have touched it.

## Kit governance

- Plugins are namespaced: use `/ecc:*` commands and `ecc:*` agents; never copy
  plugin components into `~/.claude`.
- Kit changes go through the `kit-curator` agent, except one-line edits.
- The control plane (`settings.json`, `hooks/`, permissions, model routing) is
  human-only: propose it, never edit it unprompted.
- Run `/kit-check` after changing agents, skills, or rules.

## Spec-driven development

Non-trivial work follows `spec-new` → `spec-clarify` → `spec-plan` →
`spec-tasks` → `tdd-cycle` → `spec-verify`. Status machine: draft → clarified →
planned → in-progress → verified → closed (or superseded). Specs live in
`specs/<slug>/`; templates resolve from `specs/_templates/`, then
`~/.claude/templates/sdd/`, then the embedded skeleton.

## Never

- Never `git push --force` or `git reset --hard` without explicit approval.
- Never read or print `.env`, `*.pem`, `*.key`, credentials, `~/.ssh`, or
  `~/.aws` contents.
- Never edit settings, hooks, or permissions without being asked.
