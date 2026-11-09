---
name: "demo Ownership Author"
description: "Review data-race and aliasing patterns in demo Go slices."
---

# demo Ownership Author

A reusable review workflow for data-race correctness.

## When to Activate

Activate on any diff that touches internal/features/*/ in demo.

## Core Principles

- Compiles is not correct.

## Rules

- No fabricated data; every stat needs a source.

## Research References

| claim | source | year | link |
| --- | --- | --- | --- |