# skills.schema.md

Markdown schema (triple-duty) for `.claude/skills/**/SKILL.md` in the `<app>` native iOS template (Swift 6 + SwiftUI). It is the source of truth for the skill format: frontmatter fields, body sections, the allowed `division` enum and the validation failure modes. `scripts/lint-skills.sh` parses this document at runtime; the node contract evals mirror the same lists as static slices and must be updated in the same change. No JSON Schema validator library is used.

## File Format

Each skill is one directory under `.claude/skills/<slug>/` holding a `SKILL.md`. The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body whose first line is an `# <app> <slug> skill` heading. Every slash command at `.claude/commands/<slug>.md` delegates 1:1 to the skill of the same slug.

## Required Frontmatter Fields

- name
- description
- division

## Required Body Sections

- When to Activate
- Core Principles
- Rules
- Research References
- Output Format

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

- `name`: display name matching the skill slug in kebab-case.
- `description`: one line naming what the skill produces and when to use it.
- `division`: one of the Allowed Divisions above.
- `Research References`: a table of claim, source, year and link; every row names where the rule comes from.
