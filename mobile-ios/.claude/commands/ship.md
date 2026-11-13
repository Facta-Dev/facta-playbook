---
name: ship
description: "Delegate to the ship skill — run the <app> iOS gate ladder, check the archive configuration (Swift 6 mode, signing, privacy manifest, entitlements, deployment target) and report the artifacts."
division: core
---

# /ship

## Delegate

Delegate to the **ship** skill. User-invoked only: activate when the user asks
to ship, cut a build, prepare an archive or check that the tree is in a
releasable state.

Steps:

1. Run the gate ladder and quote the real output: `bash
   scripts/check-architecture.sh`, `node --test evals/contracts.test.mjs`,
   `make ci`; run the native tier on macOS or report it as SKIP with the
   reason. A failing gate blocks the release.
2. Check the release configuration: Swift 6 mode and strict concurrency,
   signing configured (presence only — never print the material), entitlements
   matching the provisioning profile, the privacy manifest, the deployment
   target against the current store requirement.
3. Verify the `Info.plist` keys and capabilities of the release bundle.
4. Report the artifacts by path or name, and what this environment could not
   verify.
5. Write the release note: the user-visible change, the risk and the rollback
   path (the previous build stays available in App Store Connect).
6. Keep secrets, tokens and personal data out of every build log and release
   note.
