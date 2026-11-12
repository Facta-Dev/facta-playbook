# install-state.schema.md

Markdown schema (triple-duty) for `.claude/install-state.json` written by `scripts/install-claude-runtime.sh`. This document is the source of truth for the install provenance file: it defines the top-level fields, the operation object shape, the allowed `strategy` / `ownership` enums, and the validation failure modes. The pure-bash installer (`scripts/install-claude-runtime.sh`) writes this file and reads it for idempotent updates; a future uninstaller reads it to reverse every operation. No JSON Schema validator library is used (external dependency forbidden at instantiation).

## File Format

A single JSON object at `.claude/install-state.json` written by the installer on first install and rewritten on every idempotent update. The object records every copy/symlink/adapter operation so a subsequent run can reconcile (skip unchanged, update changed) and an uninstall can reverse each operation. The installer never clobbers a file it did not write; user-owned files are skipped with a recorded warning.

## Required Top-Level Fields

- schema_version
- installed_at
- updated_at
- targets
- operations

## Required Operation Fields

- source
- destination
- strategy
- ownership

## Allowed Strategies

- copy
- symlink
- adapter

## Allowed Ownership

- installer
- user
- skipped

## Validation Failure Modes

- missing top-level key
- missing operation field
- bad strategy
- bad ownership
- literal placeholder leak
- non-JSON file

## Field Semantics

- `schema_version`: integer; bumped when the provenance shape changes.
- `installed_at` / `updated_at`: ISO 8601 UTC timestamps.
- `targets`: array of runtime target slugs the install applied to (from the manifest).
- `operations`: array of operation objects; each records one copy/symlink/adapter action.
- `source`: absolute path of the canonical `.claude/{agents,skills,commands}` file.
- `destination`: absolute path of the target runtime config dir file.
- `strategy`: `copy` (file copied), `symlink` (symlinked), or `adapter` (format-converted from MD+YAML to the runtime's native format).
- `ownership`: `installer` (the installer owns this file), `user` (pre-existed, the installer skipped it), or `skipped` (no-op on this run).
