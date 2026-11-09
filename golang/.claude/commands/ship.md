---
name: ship
description: "Cut an <app> release: version tag + CHANGELOG + release PR (no image publish)."
division: core
---

# /ship

## Delegate

Delegate to the **ship** skill. Activate when the request is to cut a release
for <app> — determine the version bump from conventional commits, prepend the
`CHANGELOG.md` section, and open the release PR (Go projects version by tag;
there is no version field to bump in `go.mod`). Publishing the container image
to GHCR is a separate, `workflow_dispatch`-only `cd.yml` step this command
never triggers.