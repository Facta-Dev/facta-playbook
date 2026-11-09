---
name: "demo Ownership Author"
description: "Review borrow-checker and lifetime patterns in demo crates; produce a line-anchored report."
division: backend
---

# demo Ownership Author

A reusable review workflow for borrow-checker correctness.

## When to Activate

Activate on any diff that touches crates/demo-*/src/**/*.rs.

## Core Principles

- Compiles is not correct.
- The composition root stays thin.

## Rules

- No fabricated data; every stat needs a source.
- No code-fenced final output where raw output is expected.

## Research References

| claim | source | year | link |
| --- | --- | --- | --- |
| Arc::clone is a single atomic increment | Rust std docs | 2024 | https://example.invalid/std/sync/struct.Arc |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.
