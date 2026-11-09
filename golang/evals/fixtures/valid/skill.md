---
name: "demo Ownership Author"
description: "Review data-race and aliasing patterns in demo Go; produce a line-anchored report."
division: backend
---

# demo Ownership Author

A reusable review workflow for data-race correctness.

## When to Activate

Activate on any diff that touches internal/features/*/ in demo.

## Core Principles

- Compiles is not correct.
- The composition root stays thin.

## Rules

- No fabricated data; every stat needs a source.
- No code-fenced final output where raw output is expected.

## Research References

| claim | source | year | link |
| --- | --- | --- | --- |
| slices.Clone stops aliasing bugs | Go std docs | 2024 | https://example.invalid/std/slices |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.