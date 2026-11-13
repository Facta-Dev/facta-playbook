# commands.schema.md

Markdown schema (triple-duty) for `.claude/commands/*.md` in the `<app>` native iOS template (Swift 6 + SwiftUI). A command is a thin shim: its `name` must equal the file slug, and its body delegates to the skill of the same slug. `scripts/lint-commands.sh` parses this document at runtime; the node contract evals assert the same rules.

## File Format

Each command is one Markdown file under `.claude/commands/<slug>.md`. It opens with a YAML frontmatter block delimited by `---` fences, followed by a `# /<slug>` heading and a `## Delegate` section. No command carries its own workflow: the skill is the single source.

## Required Frontmatter Fields

- name
- description
- division

## Required Body Sections

- Delegate

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
- command name does not equal the file slug
- command has no matching skill directory

## Field Semantics

- `name`: the command slug in kebab-case; it must equal the file name without `.md`.
- `description`: one line naming the skill it delegates to.
- `division`: one of the Allowed Divisions above.
