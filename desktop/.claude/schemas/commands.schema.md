# commands.schema.md

Markdown schema (triple-duty) for `.claude/commands/<cmd>.md` in the `<app>` template. This document is the source of truth for the command file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-commands.sh`) parses the `## Required` and `## Allowed` sections below at runtime, so the schema and the linter cannot drift. The native evals (`evals/contracts_test.cpp`, Qt Test via CTest) parse this document at runtime as well — `loadSchema()` reads the same `Required Frontmatter Fields` / `Required Body Sections` / `Allowed Divisions` bullets the bash linter reads — so the schema, the linter and the evaluator cannot drift, and there is no hand-maintained mirror to keep in sync. No JSON Schema validator library is used (external dependency forbidden at instantiation). A command is a slash shim that delegates 1:1 to a skill of the same slug.

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

- `Delegate`: a single delegation statement ("Delegate to the **<slug>** skill.") plus a one-line activation condition.
