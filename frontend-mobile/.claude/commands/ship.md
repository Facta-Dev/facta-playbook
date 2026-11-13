---
name: ship
description: "Delegate to the ship skill — prepare a <app> mobile release: run the gate ladder, check the Expo config and the low-tier pass, and state what remains for the human to build and publish."
division: core
---

# /ship

## Delegate

Delegate to the **ship** skill. Activate when a `<app>` mobile release is
being cut or prepared for the store.

Steps:

1. Run the gate ladder and report the real output of each: `pnpm run
   typecheck`, `pnpm run lint`, `pnpm run test`, `pnpm run export:android`,
   plus `pnpm run evals` and `pnpm run verify`.
2. Check the version, the slug, the scheme and the bundle identifiers in
   `app/app.json`, and confirm that the pins (React `19.3.0`, React Native
   `0.88.1`, Expo `~58.0.24`, TypeScript `~6.0.3`) did not move.
3. Write the release notes from the diff and the issue list, and state
   whether the low-tier manual pass is done or pending.
4. Prepare the commit and stop. Building, signing and publishing the store
   artifact stay with the human.
