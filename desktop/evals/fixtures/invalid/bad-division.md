---
name: "demo Ownership Author"
description: "Review ownership and layering patterns in demo desktop slices."
division: backend
---

# demo Ownership Author

A reusable review workflow for ownership correctness.

## When to Activate

Activate on any diff that touches src/features/*/ in demo.

## Core Principles

- Compiles is not correct.

## Rules

- No fabricated data; every stat needs a source.

## Research References

| claim | source | year | link |
| --- | --- | --- | --- |
| viewmodels own no widgets | demo playbook | 2024 | https://example.invalid/playbook/viewmodels |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.
