# agents.schema.md

Markdown schema (triple-duty) for `.claude/agents/**/*.md` in the `<app>` template. This document is the source of truth for the agent file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-agents.sh`) parses this document at runtime, so the schema and the linter cannot drift. The local pytest evals (`evals/contracts_test.py`) do NOT parse this document — they hardcode the same lists as static slices (`required_frontmatter()` / `required_sections()` / `ALLOWED_DIVISIONS`). When you change a list here you MUST update `evals/contracts_test.py` to match; the linter<->schema drift guarantee holds via the bash linter, while the evals are a hand-maintained mirror of this schema doc. No JSON Schema validator library is used (external dependency forbidden at instantiation).

## File Format

Each agent is one Markdown file under `.claude/agents/<division>/<slug>.md`. The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body. The first body line is an `# <app> <Role>` heading. Every template substitution token is substituted by `scripts/rename.sh` before instantiation; zero residual placeholders may survive.

## Required Frontmatter Fields

- name
- description
- color
- division

## Required Body Sections

- Core Mission
- Expertise Areas
- Output Format
- Quality Standards
- Guiding Principles

## Allowed Divisions

- backend
- security
- testing
- core

## Validation Failure Modes

- missing frontmatter key
- bad division
- missing body section
- literal placeholder leak
- CRLF line endings
- empty frontmatter

## Field Semantics

- `name`: display name; carries the `<app>` placeholder (e.g. `<app> Ownership Reviewer`).
- `description`: one-line role summary; carries the `<app>` placeholder.
- `color`: hex color string (`#RRGGBB`) used by the board card.
- `division`: one of the Allowed Divisions above; the agent is filed under `.claude/agents/<division>/`.
