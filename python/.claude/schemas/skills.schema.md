# skills.schema.md

Markdown schema (triple-duty) for `.claude/skills/<slug>/SKILL.md` in the `<app>` template. This document is the source of truth for the skill file format: it defines the frontmatter fields, the body sections, the allowed `division` enum, and the validation failure modes. The pure-bash linter (`scripts/lint-skills.sh`) parses the `## Required` and `## Allowed` sections below at runtime, so the schema and the linter cannot drift. The local pytest evals (`evals/contracts_test.py`) do NOT parse this document — they hardcode the same lists as static slices (`required_frontmatter()` / `required_sections()` / `ALLOWED_DIVISIONS`). When you change a list here you MUST update `evals/contracts_test.py` to match; the linter<->schema drift guarantee holds via the bash linter, while the evals are a hand-maintained mirror of this schema doc. No JSON Schema validator library is used (external dependency forbidden at instantiation). A skill is a reusable workflow an agent invokes; an agent is a role.

## File Format

Each skill is one Markdown file at `.claude/skills/<slug>/SKILL.md`. The file opens with a YAML frontmatter block delimited by `---` fences, followed by a Markdown body. The first body line is an `# <app> <Skill>` heading. Every template substitution token is substituted by `scripts/rename.sh` before instantiation; zero residual placeholders may survive.

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

- `name`: display name; carries the `<app>` placeholder (e.g. `<app> Ownership Author`).
- `description`: one-line workflow summary; carries the `<app>` placeholder, naming the surface the skill reviews or scaffolds.
- `division`: one of the Allowed Divisions above; maps the skill to the agent division it composes with.

## Body Section Semantics

- `When to Activate`: the diff shapes that trigger the skill, and the shapes that do not.
- `Core Principles`: the invariants the skill enforces, stated as short rules.
- `Rules`: anti-hallucination rules (no fabricated data, every stat needs a source, no code-fenced final output where raw output is expected) plus skill-specific rules.
- `Research References`: a provenance table with columns `claim | source | year | link`; every claim in a finding needs a row.
- `Output Format`: the exact shape of the raw output the skill emits (plain text, one finding per line, severity-prefixed, summary line).
