---
name: "ship"
description: "Prepares a <app> desktop release: bumps the version in cmake/versioning.cmake, updates the changelog and the packaging metadata, runs the full gate ladder and states what remains for the human to tag and publish."
division: core
---

# `<app>` ship skill

> Reusable workflow for cutting a release of the `<app>` desktop app. An
> agent invokes this skill; the skill supplies the checklist, the version
> bump and the gate ladder. Tagging, signing and publishing stay with the
> human.

## When to Activate

Activate this skill when the request is to cut or prepare a release: a
version bump, a release branch, a changelog entry, an artifact that CI must
build and sign.

Do NOT activate this skill when:

- The request is a code change without a release (implement it).
- The request is to file an issue (use `spec`) or to plan a large change (use
  `plan`).

## Core Principles

- **The version has one source.** `DESKTOP_VERSION` in
  `cmake/versioning.cmake` is the truth; the packaging metadata, the
  `about` dialog and the changelog all read from it, so they cannot disagree.
- **The gates decide, not the calendar.** A release is prepared only when the
  whole ladder is green: configure, build, offscreen `ctest`, evals, and the
  format and tidy checks.
- **The artifact is built by CI, on a signed tag.** The agent prepares the
  commit and the release notes; it does not tag, sign or publish.
- **The notes describe behaviour, not commits.** One line per user-visible
  change, plus the migration notes when the storage schema moved.
- **Nothing secret travels.** No token, key or private URL appears in the
  release notes, the changelog or the artifact.

## Rules

- Never tag, push, sign or publish from an agent session: the human does
  that. The agent stops at a green tree and a prepared commit.
- Never bump the version in one file only: check the packaging metadata and
  the `about` text for a duplicated version string.
- A storage schema change requires the migration note in the changelog and
  the version bump in the `note_store.cpp` migration list — the two travel
  together.
- The changelog entry lists what a user can see, not which commits landed.
- The full gate ladder is run and its real output reported; a skipped gate is
  reported as skipped, never as green.
- The release notes are written from the diff and the issue list, never from
  memory.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The version lives in a single CMake cache variable | `cmake/versioning.cmake` | - | `cmake/versioning.cmake` |
| The release checklist and the packaging matrix | `PLAYBOOK_DESKTOP_QT.md` §14, §16 | - | `PLAYBOOK_DESKTOP_QT.md` |
| The gate ladder that must be green before a release | `PLAYBOOK_TESTS.md` | - | `PLAYBOOK_TESTS.md` |
| CMake presets produce the release configuration | CMake documentation | 2025 | https://cmake.org/cmake/help/latest/manual/cmake-presets.7.html |
| Migrations are forward-only and numbered | `src/infrastructure/src/storage/note_store.cpp` | - | `src/infrastructure/src/storage/note_store.cpp` |

## Output Format

```
Release prepared for: <app> <old version> → <new version>
Version bump: cmake/versioning.cmake → <old> → <new>
Changelog: <path> — <entries>
Schema version: <unchanged | N+1 — migration note included>
Gates:
  cmake --preset release → <result>
  cmake --build build-release → <result>
  QT_QPA_PLATFORM=offscreen ctest --test-dir build-release → <result>
  make evals → <result>
  make lint → <result>
Release notes:
  - <user-visible change>
Ready for the human: commit, tag, sign, publish
Not done: <anything the human must complete>
```
