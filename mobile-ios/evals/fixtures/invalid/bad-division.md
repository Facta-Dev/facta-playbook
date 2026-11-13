---
name: "demo iOS Author"
description: "Review demo iOS diffs for package boundaries; produce a line-anchored report."
division: backend
---

# demo iOS Author

A reusable review workflow for iOS package and SwiftUI data-flow correctness.

## When to Activate

Activate on any diff that touches Packages/ or Apps/ in demo.

## Core Principles

- The package graph is provable, not assumed.
- Pure Swift packages never import SwiftUI or UIKit.

## Rules

- No fabricated data; every finding needs a file:line.
- No code-fenced final output where raw output is expected.

## Research References

| claim                  | source                | year | link                                     |
| ---------------------- | --------------------- | ---- | ---------------------------------------- |
| typed navigation routes | Apple NavigationStack | 2026 | https://example.invalid/ios/navigation   |

## Output Format

Plain text, one finding per line, severity-prefixed, then a summary line.
