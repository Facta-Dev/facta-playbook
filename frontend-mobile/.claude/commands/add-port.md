---
name: add-port
description: "Delegate to the add-port skill — add a device capability to <app> as a consumer-defined port, with its fake, its adapter, its wiring and the test that proves the seam."
division: security
---

# /add-port

## Delegate

Delegate to the **add-port** skill. Activate when `<app>` needs a new device
capability — secure storage, push, network state, deep links or a new one.

Steps:

1. Name the capability and the consumer that needs it. The port belongs to
   the consumer, not to the platform.
2. Declare the plain interface in `packages/core/src/ports.ts` with no
   `react-native` and no `expo` import.
3. Write the fake in `tests/_shared/fakes/` and the unit test that runs
   against the fake.
4. Write the adapter in `adapters/out/` and keep every platform import
   inside it.
5. Wire the adapter into the composition root in `app/App.tsx`; nothing else
   constructs a platform object.
6. Run `pnpm run typecheck`, `pnpm run lint` and `pnpm run test` and report the
   real output.
