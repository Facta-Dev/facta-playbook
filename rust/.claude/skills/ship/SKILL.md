---
name: "ship"
description: "Cuts an <app> release: bumps Cargo.toml [workspace.package] version from conventional commits, prepends CHANGELOG.md, and opens one release PR — never commits Cargo.lock from the skeleton context."
division: core
---

# <app> ship skill

> Reusable workflow for cutting an <app> release PR. An agent invokes this
> skill; the agent supplies the role, the skill supplies the workflow.

## When to Activate

Activate this skill when the request is to cut a release for <app>: determine
the version bump from commit history, update the `[workspace.package]`
`version` field in the root `Cargo.toml` (every workspace member inherits it
via `version.workspace = true`), prepend a `CHANGELOG.md` entry, and open the
release PR.

Do NOT activate this skill when:

- The request is to scaffold a new handler or module (use `add-handler` /
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
- `Cargo.toml`'s `[workspace.package] version` is the single version file for
  this template (per the batch version-file decision) — every crate under
  `crates/` inherits it via `version.workspace = true`; no per-crate version
  bump is needed.
- Never commit a `Cargo.lock` from the skeleton context: the template ships no
  lockfile by design (a literal `<app>` placeholder is not a valid crate name
  until `rename.sh` runs), and the release commit must not introduce one.
- The changelog entry is derived from real `git log` subjects grouped by
  conventional-commit type; nothing is fabricated.
- One PR per release, titled with the new version, ending in `Closes #<n>`
  when the release closes a tracked issue — never a Co-Authored-By trailer.

## Rules

- Never run destructive git operations: no `--force` push, no history rewrite,
  no skipping hooks.
- Never touch `Cargo.lock` as part of a release commit.
- Never invent changelog entries — every line traces to a real commit subject
  from `git log` since the last tag.
- Never publish a container image or crate from this skill — that is `cd.yml`'s
  job, triggered manually and gated on its own secrets.
- The release commit message is exactly `chore(release): v<X.Y.Z>` with no
  trailer.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| (claim placeholder — replace with any external citation) | (source) | (year) | (link) |

## Output Format

Deliver, in order:

1. The determined bump (major/minor/patch) with the commit(s) that justified
   it.
2. The `Cargo.toml` `[workspace.package] version` diff.
3. The prepended `CHANGELOG.md` section: `## v<X.Y.Z> — <date>` with commit
   subjects grouped by type (feat / fix / chore / etc.).
4. The release commit message: `chore(release): v<X.Y.Z>`.
5. The PR title and body: title `chore(release): v<X.Y.Z> — <summary> (Closes
   #<n>)`, body summarizing the changelog section.

Example changelog section:

```
## v0.3.0 — 2026-07-13

### Features
- feat(widgets): add priority column + adapter projection

### Fixes
- fix(auth): reject expired jti on refresh, not just on mint
```
