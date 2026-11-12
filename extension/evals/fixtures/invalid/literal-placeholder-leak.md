---
name: "demo Extension Author"
description: "Review demo extension diffs for permission posture; produce a line-anchored report."
division: core
---

# demo Extension Author

A reusable review workflow for extension permission and layer correctness.

## When to Activate

Activate on any diff that touches src/ or entrypoints/ in demo.

## Core Principles

- Least privilege is provable, not assumed.

## Rules

- No fabricated data; every stat needs a source.

## Research References

| claim                       | source                | year | link                                           |
| --------------------------- | --------------------- | ---- | ---------------------------------------------- |
| host_permissions stay empty | Chrome extension docs | 2026 | https://example.invalid/extensions/permissions |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.

Residual tokens that must never survive the rename pass: <app>, <APP>, <scope>.
