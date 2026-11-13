# commands.schema.md

Markdown schema (triple-duty) for `.claude/commands/<cmd>.md` in the `<app>` React Native + Expo mobile template. This document is the source of truth for the command file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-commands.sh`) parses the `## Required` and `## Allowed` sections below at runtime, so the schema and the linter cannot drift. The native `node --test` evals (`evals/contracts.test.mjs`) read the `## Allowed Divisions` section below at runtime (`schemaDivisions()`), so the division enum cannot drift there; only the required frontmatter fields and the required body sections are hardcoded in the evals (`requiredFrontmatter()` / `requiredSections()`), so when you change one of those two lists here you MUST update `evals/contracts.test.mjs` to match. No JSON Schema validator library is used (external dependency forbidden at instantiation). A command is a slash shim that delegates 1:1 to a skill of the same slug.

## File Format

Each command is one Markdown file at `.claude/commands/<cmd>.md`, where `<cmd>` equals the skill slug it delegates to (1:1 mapping). The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body. The first body line is a `# /<cmd>` heading. All `<app>` / `<APP>` / `<scope>` placeholders are substituted by `scripts/rename.sh` before instantiation; zero residual placeholders may survive.

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
- command slug not matching a skill slug
- CRLF line endings
- empty frontmatter

## Field Semantics

- `name`: the command slug; must equal the skill slug it delegates to.
- `description`: one-line summary; carries the `<app>` placeholder.
- `division`: one of the Allowed Divisions above; must match the delegated skill's division.

## Body Section Semantics

- `Delegate`: a single delegation statement ("Delegate to the **<slug>** skill.") plus the concrete steps to take.
