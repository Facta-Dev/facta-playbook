# skills.schema.md

Markdown schema (triple-duty) for `.claude/skills/<slug>/SKILL.md` in the `<app>` React Native + Expo mobile template. This document is the source of truth for the skill file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-skills.sh`) parses the `## Required` and `## Allowed` sections below at runtime, so the schema and the linter cannot drift. The native `node --test` evals (`evals/contracts.test.mjs`) read the `## Allowed Divisions` section below at runtime (`schemaDivisions()`), so the division enum cannot drift there; only the required frontmatter fields and the required body sections are hardcoded in the evals (`requiredFrontmatter()` / `requiredSections()`), so when you change one of those two lists here you MUST update `evals/contracts.test.mjs` to match. No JSON Schema validator library is used (external dependency forbidden at instantiation). A skill is a reusable workflow an agent invokes; an agent is a role.

## File Format

Each skill is one Markdown file at `.claude/skills/<slug>/SKILL.md`. The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body. The first body line is an `# <app> <Skill>` heading. The frontmatter `name` equals the directory slug, so a command can delegate to it by name. All `<app>` / `<APP>` / `<scope>` placeholders are substituted by `scripts/rename.sh` before instantiation; zero residual placeholders may survive.

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

- `name`: the skill slug; must equal the directory name (e.g. `add-slice`).
- `description`: one-line workflow summary; carries the `<app>` / `<scope>` placeholders, naming the mobile surface the skill scaffolds or reviews.
- `division`: one of the Allowed Divisions above; maps the skill to the agent division it composes with.

## Body Section Semantics

- `When to Activate`: the diff shapes that trigger the skill, and the shapes that do not.
- `Core Principles`: the invariants the skill enforces, stated as short rules.
- `Rules`: anti-hallucination rules (no fabricated data, every stat needs a source, no code-fenced final output where raw output is expected) plus skill-specific rules.
- `Research References`: a provenance table with columns `claim | source | year | link`; every claim in a finding needs a row.
- `Output Format`: the exact shape of the raw output the skill emits (plain text, one finding per line, severity-prefixed, summary line).
