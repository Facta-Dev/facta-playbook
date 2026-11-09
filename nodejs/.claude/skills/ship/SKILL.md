---
name: "ship"
description: "Cuts an <app> release: derives the bump from conventional commits, prepends CHANGELOG.md, opens one release PR tagged vX.Y.Z — never runs pnpm install/add or commits pnpm-lock.yaml from the release context."
division: core
---

# <app> ship skill

> Reusable workflow for cutting an <app> release PR. An agent invokes this
> skill; the agent supplies the role, the skill supplies the workflow.

## When to Activate

Activate this skill when the request is to cut a release for <app>: determine
the version bump from commit history, prepend a `CHANGELOG.md` entry, and
open the release PR whose head lands the `vX.Y.Z` tag. Node projects version
by git tag — `package.json`'s `version` field is not the release surface for
`<app>` (the app is a service, not a registry-published package; nothing
bumps that field), so the tag on the release commit IS the version.

Do NOT activate this skill when:

- The request is to scaffold a new handler or slice (use `add-handler` /
  `add-module` instead).
- The request is to file an issue or feature spec (use `spec` instead).
- There are no commits since the last version tag (nothing to ship).

## Core Principles

- Preflight before anything else: the working tree must be clean-ish (no
  unrelated unstaged changes), the current branch must not be the default
  branch, and `git log` must be reachable. `git`/`gh` are used ONLY at ship
  time — no new dependency, no publish tooling is introduced.
- The version bump is derived, never guessed: any `BREAKING CHANGE`/`!` commit
  since the last tag (or the full history if there is no tag) forces a major
  bump; any `feat` commit forces minor; otherwise patch.
- The git tag `vX.Y.Z` is the single version surface for this template —
  nothing releases `<app>` to an npm registry, so no per-file version bump is
  needed and none is invented.
- Never run `pnpm install`/`pnpm add` or commit a `pnpm-lock.yaml` from the
  ship context: dependency bumps and resolution are a separate chore commit;
  the release commit only carries the changelog. (The skeleton ships no
  pnpm-lock.yaml until post-rename `pnpm install` — INV-T3 — and that commit
  belongs to instantiation, not to a release.)
- The changelog entry is derived from real `git log` subjects grouped by
  conventional-commit type; nothing is fabricated.
- One PR per release, titled with the new version, ending in `Closes #<n>`
  when the release closes a tracked issue — never a Co-Authored-By trailer.

## Rules

- Never run destructive git operations: no `--force` push, no history rewrite,
  no skipping hooks.
- Never touch `pnpm-lock.yaml`, `src/gen/`, or `migrations/` as part of a
  release commit.
- Never invent changelog entries — every line traces to a real commit subject
  from `git log` since the last tag.
- Never publish a container image from this skill — that is `cd.yml`'s job,
  triggered manually and gated on its own secrets.
- The release commit message is exactly `chore(release): v<X.Y.Z>` with no
  trailer.

## Research References

| Claim                                                    | Source   | Year   | Link   |
| -------------------------------------------------------- | -------- | ------ | ------ |
| (claim placeholder — replace with any external citation) | (source) | (year) | (link) |

## Output Format

Deliver, in order:

1. The determined bump (major/minor/patch) with the commit(s) that justified
   it.
2. The tag to create: `v<X.Y.Z>` (semver, `v`-prefixed; no version file edit —
   the git tag is the version surface).
3. The prepended `CHANGELOG.md` section: `## v<X.Y.Z> — <date>` with commit
   subjects grouped by type (feat / fix / chore / etc.).
4. The release commit message: `chore(release): v<X.Y.Z>`.
5. The PR title and body: title `chore(release): v<X.Y.Z> — <summary> (Closes
#<n>)`, body summarizing the changelog section.

Example changelog section:

```
## v0.3.0 — 2026-07-13

### Features
- feat(widgets): add priority column + queries.ts projection

### Fixes
- fix(auth): reject expired jti on refresh, not just on mint
```
