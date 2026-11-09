---
name: "demo Ownership Author"
description: "Review data-race and aliasing patterns in demo Python; produce a line-anchored report."
division: backend
---

# demo Ownership Author

A reusable review workflow for data-race correctness.

## When to Activate

Activate on any diff that touches src/app/features/*/ in demo.

## Core Principles

- Passing tests is not correct. A green suite proves the tests that exist pass.
- Shared mutable state is the defect; naming it is the fix.

## Rules

- Never fabricate a finding; cite the exact file:line and quoting the code.
- No Spanish anywhere in the report.

## Research References

| claim | source | year | link |
| ----- | ------ | ---- | ---- |
| asyncio interleaving only at await points | CPython docs | 2024 | https://docs.python.org/3/library/asyncio-task.html |

## Output Format

A line-anchored review report with BLOCK/WARN findings and a final verdict.
