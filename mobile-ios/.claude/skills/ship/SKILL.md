---
name: "ship"
description: "Cuts a release of the <app> iOS app: runs the gate ladder, checks the archive configuration (Swift 6 mode, signing, privacy manifest, entitlements, deployment target) and reports the artifacts."
division: core
---

# `<app>` ship skill

> Reusable workflow for taking a change from "green on my machine" to "the
> gates are green and the archive is reproducible". An agent invokes this
> skill; the requester supplies the intent to release, the skill supplies the
> ladder and the checklist. User-invoked only: a release is never automatic.

## When to Activate

Activate this skill when a change is ready to be released or handed over: the
user asks to ship, to cut a build, to prepare an archive, or to check that the
tree is in a releasable state.

Do NOT activate this skill when:

- The change is not finished — a failing gate is a fix, not a release.
- The request is only to review the diff (use `review`).
- The request is to write the requirements (use `spec`).

## Core Principles

- **The gates are the definition of done.** `make ci`,
  `bash scripts/check-architecture.sh` and `node --test
  evals/contracts.test.mjs` must pass, and the output is quoted; the native
  tier runs on macOS and is reported as SKIP elsewhere.
- **The archive is a different artifact.** A simulator build is not an
  archive: destination, signing, entitlements and the privacy manifest must be
  checked on the release configuration.
- **The bundle that ships is the one that is reviewed.** `Info.plist` keys,
  entitlements and the privacy manifest of the release build.
- **Reproducibility over speed.** The pinned toolchain (Swift 6, iOS 18,
  XcodeGen from `project.yml`) is part of the artifact.
- **Report what happened.** The summary lists the real results, the artifacts
  and what was skipped.

## Rules

- No fabricated build output, artifact name, size or test result: run the
  command or say it was not run.
- A failing gate blocks the release; the skill fixes the cause or reports the
  block, it never relaxes the check.
- Signing material is never printed, never committed and never uploaded; only
  its presence is verified.
- The release checklist covers the deployment target, the privacy manifest,
  the entitlements/provisioning match and the App Store processing state.
- Secrets, tokens and personal data never enter a build log or a release note.
- The release note states the user-visible change, the risk and the rollback
  path (the previous build stays available in App Store Connect).

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The archive is the distributable artifact | Apple — Distributing your app | 2026 | https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases |
| Privacy manifest files ship with the binary | Apple — Privacy manifest files | 2026 | https://developer.apple.com/documentation/bundleresources/privacy-manifest-files |
| Entitlements must match the provisioning profile | Apple — Entitlements | 2026 | https://developer.apple.com/documentation/bundleresources/entitlements |
| The release checklist of a native iOS client | `PLAYBOOK_MOBILE_IOS_SWIFT.md` §5.1 | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |

## Output Format

```
Release: <app> — <configuration> — <version>
Gates:
  bash scripts/check-architecture.sh -> <result>
  node --test evals/contracts.test.mjs -> <result>
  make ci -> <result>
  native gates -> <result or SKIP: reason>
Release checks:
  Swift 6 / strict concurrency: <ok | finding>
  signing: <configured | missing> — <no secret printed>
  entitlements / provisioning: <match | finding>
  privacy manifest: <ok | missing | finding>
  deployment target: <version> — <store requirement ok | finding>
  Info.plist / capabilities: <keys>
Artifacts: <path or name>
Not verified: <what this environment could not check>
```
