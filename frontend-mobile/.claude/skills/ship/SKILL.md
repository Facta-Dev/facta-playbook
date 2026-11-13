---
name: "ship"
description: "Prepares a <app> mobile release: runs the full gate ladder (typecheck, lint, test, expo export android), checks the Expo config and the low-tier pass, states what remains for the human to build and publish."
division: core
---

# `<app>` ship skill

> Reusable workflow for cutting a release of the `<app>` mobile app. An
> agent invokes this skill; the skill supplies the checklist, the gate ladder
> and the release notes. Building the store artifact, signing and publishing
> stay with the human.

## When to Activate

Activate this skill when the request is to cut or prepare a release: a
version bump, a release branch, a changelog entry, an artifact that CI must
build, or a readiness check before a store submission.

Do NOT activate this skill when:

- The request is a code change without a release (implement it).
- The request is to file an issue (use `spec`) or to plan a large change (use
  `plan`).

## Core Principles

- **The gates decide, not the calendar.** A release is prepared only when the
  whole ladder is green: `pnpm run typecheck`, `pnpm run lint`,
  `pnpm run test`, `pnpm run export:android`, plus `pnpm run evals` and
  `pnpm run verify` for the template itself.
- **The pins are the release contract.** React `19.3.0`, React Native
  `0.88.1`, Expo `~58.0.24` and TypeScript `~6.0.3` are what the bundle is
  built against; a release never moves them silently.
- **The artifact is built by CI, on a signed tag.** The agent prepares the
  commit and the release notes; it does not build for the store, sign or
  publish.
- **The notes describe behaviour, not commits.** One line per user-visible
  change, plus the notes for a schema or route change.
- **Nothing secret travels.** No token, key or private URL appears in the
  release notes, the changelog or the artifact.

## Rules

- Never sign, publish or push a store artifact from an agent session: the
  human does that. The agent stops at a green tree and a prepared commit.
- The version is never bumped in one place only: check the Expo config in
  `app/app.json` and the workspace `package.json` for a duplicated version
  string.
- The lockfile is never committed; the template ships without one.
- The full gate ladder is run and its real output reported; a skipped gate is
  reported as skipped, never as green.
- A low-tier manual pass is part of the release, and its absence is stated
  plainly in the readiness report.
- The release notes are written from the diff and the issue list, never from
  memory.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The version, the slug, the scheme and the bundle identifiers | `app/app.json` | 2026 | local file |
| The gate ladder that must be green before a release | `package.json`, `Makefile` | 2026 | local file |
| The release recipe and the store checklist | `docs/13-playbooks.md` §6 | 2026 | local file |
| The low-tier budgets the manual pass checks | `docs/02-device-tiers-and-the-quality-governor.md` | 2026 | local file |
| The Android export the release gate runs | `expo export --platform android` | 2026 | https://docs.expo.dev/build/introduction/ |
| The Expo app configuration fields the release depends on | Expo app config reference | 2026 | https://docs.expo.dev/versions/latest/config/app/ |

## Output Format

```
Release prepared for: <app> <old version> → <new version>
Version bump: app/app.json → <old> → <new>
Changelog: <path> — <entries>
Gates:
  pnpm run typecheck → <result>
  pnpm run lint → <result>
  pnpm run test → <result>
  pnpm run export:android → <result>
  pnpm run evals → <result>
  pnpm run verify → <result>
Low-tier manual pass: <done | pending>
Release notes:
  - <user-visible change>
Ready for the human: commit, tag, build, sign, publish
Not done: <anything the human must complete>
```
