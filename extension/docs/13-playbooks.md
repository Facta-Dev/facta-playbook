# Playbooks

Repeatable recipes for the changes you will make most. Each: the steps, the
files touched and the gate that proves it. If a change touches more files
than listed, the architecture regressed — investigate before proceeding.

## 1. Add a slice (a new feature)

1. `src/features/<slice>/` with `domain/`, `application/`, `adapters/out/`,
   `viewmodels/`, `ui/` and `contracts/`.
2. **Domain first**: the entity, its invariants, its `Result` errors. No
   `chrome`, no React, no storage.
3. `application/ports.ts`: the interface the slice needs, declared by the
   slice.
4. `application/use-cases.ts`: the use-cases, returning `Result`.
5. `adapters/out/`: the implementation of the port, over the platform
   adapter.
6. `viewmodels/` and `ui/`: the state and the surface.
7. `contracts/index.ts`: re-export the shared zod schemas.
8. `index.ts`: the barrel. The entrypoints import from here and from
   nowhere else inside the slice.
9. `src/messaging/`: the handlers and the entry in the `PROTOCOL` table with
   its allowed sender classes.
10. `entrypoints/background.ts`: wire the handlers into
    `registerMessageServer`.

**Gate**: `make ci`.

## 2. Add a message type

1. `src/contracts/protocol.ts`: the payload schema (strict), the result
   schema, the request and the response schema, and the entry in
   `PROTOCOL` with its `senders`.
2. `src/messaging/<slice>.ts`: the handler. It returns the result the
   contract declares — the server validates it.
3. `src/messaging/server.ts`: nothing to change; the handler table is typed
   by the protocol.
4. The client: use `createRpcClient` from `src/messaging/client.ts`, or send
   the envelope by hand.

**Gate**: `make ci` plus a contract test that sends the message and asserts
the response and the error cases.

## 3. Change a permission

1. `docs/permissions-register.md`: add or update the entry — what it grants,
   why, what it exposes, how it is removed.
2. Run the `permission-change` skill and the
   `manifest-permissions-reviewer` agent.
3. `src/manifest.ts`: the declaration.
4. The code that uses it, behind the platform adapter.

**Gate**: `make ci` and `pnpm run evals` — the manifest posture test fails if
`host_permissions` is not empty or if a broad pattern appears.

## 4. Ship a storage migration

1. `src/contracts/storage.ts`: bump `NOTES_STORAGE_VERSION` (or the version
   of your store) and add the new shape to the record schema.
2. `src/storage/<slice>-store.ts`: add a step to `MIGRATIONS` with the
   matching `from` and `to`. A step must **remove** the keys it replaces, or
   the strict schema rejects the migrated record.
3. A test with the old record as the fixture: the store must read it, apply
   the step and write the new shape.
4. A test with a corrupt record: the store must back it up and start empty,
   not throw.

**Gate**: `make ci`, plus the `storage-migration` skill and the
`storage-migration-reviewer` agent.

## 5. Add a UI surface

1. The surface lives in the slice: `viewmodels/` for the state, `ui/` for the
   component, and a `useSyncExternalStore` binding.
2. A new page needs an entry in `entrypoints/` (an HTML file and a script)
   and, for a side panel or an options page, a path in `src/manifest.ts`.
3. The component is a pure function of the view-model state: no `chrome`,
   no direct storage access.

**Gate**: `make ci` and a manual check in Chrome (`chrome://extensions` →
_Load unpacked_ → `dist/`).

## 6. Add a UI surface that talks to the worker

1. Prefer the messaging client: `createRpcClient(runtime)` from
   `src/messaging/client.ts` validates the request and the answer.
2. The message type must exist in `PROTOCOL` with `senders: ["ui"]` if only
   an extension page may send it.
3. The worker must be the only writer of the store: a second context that
   opens the store can lose a concurrent write. The shipped side panel goes
   through the client for that reason — keep it that way.

**Gate**: `make ci` plus a contract test for the new type.

## 7. Start a project from the template

1. Copy the kit and run `bash scripts/rename.sh <app-slug> <org-scope>`.
2. `pnpm install`, then `pnpm format` (the substitution re-aligns the
   markdown tables).
3. `make ci` — the gates must be green before you change anything.
4. Replace the `notes` slice with your own; keep the layering.
5. `pnpm run verify` proves the template itself is still sound.

**Gate**: `make ci` and `pnpm run verify`.

## Do / Don't

| Do                                                              | Don't                                        |
| --------------------------------------------------------------- | -------------------------------------------- |
| Follow the recipe; a longer diff means the architecture changed | Add a shortcut "just this once"              |
| Update the register entry in the same change                    | Change a permission without a register entry |
| Write the migration test with the old record                    | Migrate in place without a fixture           |
| Keep the worker as the only writer of a store                   | Let two contexts write the same key          |

## See also

- [Layers & MVVM](03-hexagonal-layers-and-mvvm.md)
- [Vertical Slicing](04-vertical-slicing.md)
- [Testing & CI](12-testing-and-ci.md)
