---
name: "<app> Ownership Author"
description: "Review data-race patterns in <app> src/features across the <scope> slice; set <APP>_JWT_SECRET first."
division: backend
---

# <app> Ownership Author

A reusable review workflow for data-race correctness.

## When to Activate

Activate on any diff that touches src/features/<app>*/.

## Core Principles

- Compiles is not correct.

## Rules

- No fabricated data; every stat needs a source.

## Research References

| claim | source | year | link |
| ----- | ------ | ---- | ---- |
