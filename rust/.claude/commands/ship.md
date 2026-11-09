---
name: ship
description: "Cut an <app> release: version bump + CHANGELOG + release PR (no image publish)."
division: core
---

# /ship

## Delegate

Delegate to the **ship** skill. Activate when the request is to cut a release for <app> — determine the version bump from conventional commits, update `Cargo.toml`'s `[workspace.package]` version, prepend `CHANGELOG.md`, and open the release PR. Publishing the container image to GHCR is a separate, `workflow_dispatch`-only `cd.yml` step this command never triggers.
