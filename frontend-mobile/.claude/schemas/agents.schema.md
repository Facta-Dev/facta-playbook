# agents.schema.md

Markdown schema (triple-duty) for `.claude/agents/**/*.md` in the `<app>` React Native + Expo mobile template. This document is the source of truth for the agent file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-agents.sh`) parses this document at runtime, so the schema and the linter cannot drift. The native `node --test` evals (`evals/contracts.test.mjs`) read the `## Allowed Divisions` section below at runtime (`schemaDivisions()`), so the division enum cannot drift there; only the required frontmatter fields and the required body sections are hardcoded in the evals (`requiredFrontmatter()` / `requiredSections()`), so when you change one of those two lists here you MUST update `evals/contracts.test.mjs` to match. No JSON Schema validator library is used (external dependency forbidden at instantiation).

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

- `name`: display name; carries the `<app>` placeholder (e.g. `<app> React Native Reviewer`).
- `description`: one-line role summary; carries the `<app>` / `<scope>` placeholders, naming the React Native or Expo surface the agent owns.
- `color`: hex color string (`#RRGGBB`) used by the board card.
- `division`: one of the Allowed Divisions above; the agent is filed under `.claude/agents/<division>/` and the directory name must equal this value.
