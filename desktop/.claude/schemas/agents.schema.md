# agents.schema.md

Markdown schema (triple-duty) for `.claude/agents/**/*.md` in the `<app>` template. This document is the source of truth for the agent file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-agents.sh`) parses this document at runtime, so the schema and the linter cannot drift. The native evals (`evals/contracts_test.cpp`, Qt Test via CTest) parse this document at runtime as well — `loadSchema()` reads the same `Required Frontmatter Fields` / `Required Body Sections` / `Allowed Divisions` bullets the bash linter reads — so the schema, the linter and the evaluator cannot drift, and there is no hand-maintained mirror to keep in sync. No JSON Schema validator library is used (external dependency forbidden at instantiation).

## File Format

Each agent is one Markdown file under `.claude/agents/<division>/<slug>.md`. The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body. The first body line is an `# <app> <Role>` heading. All `<app>` / `<APP>` / `<scope>` placeholders are substituted by `scripts/rename.sh` before instantiation; zero residual placeholders may survive.

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

- client
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
- `description`: one-line role summary; carries the `<app>` / `<scope>` placeholders.
- `color`: hex color string (`#RRGGBB`) used by the board card.
- `division`: one of the Allowed Divisions above; the agent is filed under `.claude/agents/<division>/`.
