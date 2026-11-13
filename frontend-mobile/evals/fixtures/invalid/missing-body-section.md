---
name: "demo Mobile Author"
description: "Review demo React Native diffs for layer boundaries; produce a line-anchored report."
division: core
---

# demo Mobile Author

A reusable review workflow for React Native layer correctness and device-tier policy.

## When to Activate

Activate on any diff that touches app/ or packages/ in demo.

## Core Principles

- The quality governor owns the tier decision; a screen never guesses.

## Rules

- No fabricated data; every stat needs a source.

## Research References

| claim                          | source          | year | link                                      |
| ------------------------------ | --------------- | ---- | ----------------------------------------- |
| core imports no react-native   | Expo SDK 58 docs | 2026 | https://example.invalid/expo/versions      |
