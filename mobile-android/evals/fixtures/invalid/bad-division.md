---
name: "demo Android Author"
description: "Review demo Android diffs for layer boundaries; produce a line-anchored report."
division: backend
---

# demo Android Author

A reusable review workflow for Android module and UDF correctness.

## When to Activate

Activate on any diff that touches feature/, core/ or app/ in demo.

## Core Principles

- The module graph is provable, not assumed.
- Pure JVM modules never import the Android SDK.

## Rules

- No fabricated data; every finding needs a file:line.
- No code-fenced final output where raw output is expected.

## Research References

| claim              | source             | year | link                                          |
| ------------------ | ------------------ | ---- | --------------------------------------------- |
| typed NavKey routes | Navigation 3 docs  | 2026 | https://example.invalid/android/navigation    |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.
