# Testing & CI

The gates are the contract. A change is finished when `pnpm run ci` is
green; a change to the kit's own agent runtime is finished when the evals
pass.

## 1. The test tiers

| Tier          | Tool            | What it covers                                                 | Command                  |
| ------------- | --------------- | -------------------------------------------------------------- | ------------------------ |
| Unit          | Vitest          | Domain rules, the use-cases, the view models, the governor     | `pnpm test`              |
| Contract      | Vitest          | The route table, the builders and the parser                   | `pnpm test`              |
| Architecture  | Vitest (static) | The import rules of §4 and the eight statuses                  | `pnpm test`              |
| Platform only | Vitest          | The architecture suite alone                                   | `pnpm run test:platform` |
| Evals         | `node --test`   | The `.claude` kit: agents, skills, commands, placeholder leaks | `pnpm run evals`         |
| Template      | bash            | The whole template, renamed in a temp dir                      | `pnpm run verify`        |

## 2. The layout

```text
tests/
├── _platform/architecture/import-boundaries.test.ts  # the static import rules
├── contracts/routes.test.ts                          # the route table and the parser
├── core/quality-governor.test.ts                     # the tier and the governor
└── notes/
    └── unit/
        ├── application/use-cases.test.ts
        ├── domain/note.test.ts
        └── viewmodels/notes-view-model.test.ts
```

The tests live outside the packages, in a tree that mirrors them:
`tests/<slice>/unit/` for a slice, `tests/core/` for the kernel,
`tests/contracts/` for the contracts, `tests/_platform/` for the platform and
the architecture. The shared fakes the `add-port` skill writes go in
`tests/_shared/fakes/`.

## 3. Fakes, not mocks

There is no mocking library in the dependencies. A fake is a small
hand-written object that implements a port (`SecureStorePort`, `PushPort`,
`NetworkStatePort`, `DeepLinkPort`). A fake that is ten lines long is
cheaper than a framework, and it makes the port boundary visible: if a fake
gets awkward, the port is wrong.

The view model is tested against the fakes, so the whole screen can be
tested without a device, a simulator or a React Native renderer — the view
model is framework-free and the test drives `load()` and reads `getState()`.

## 4. The architecture suite is static

`tests/_platform/architecture/import-boundaries.test.ts` reads the source
files as text, extracts the import specifiers with a regular expression and
fails when a boundary is crossed:

| Rule                                                    | What it rejects                                           |
| ------------------------------------------------------- | --------------------------------------------------------- |
| No device import in the core or the contracts           | `react`, `react-native` or `expo` in a pure package       |
| No device import in the domain or the application layer | `react`, `react-native` or `expo`                         |
| No native import in the view models                     | `react-native` or `expo` in a view model                  |
| No adapter in the views                                 | A view reaching for `**/adapters/**` or `**/*repository*` |

The same suite asserts that the view model carries the **eight statuses** of
the playbook, so the shared set cannot be quietly shortened.

ESLint enforces the same boundaries with `no-restricted-imports`, scoped by
directory (`eslint.config.mjs`); the two must agree.

It is not a lint opinion; it is a test, so it runs in `pnpm test` and not
only in `pnpm run lint`.

## 5. The commands

| Command                                     | What it runs                                               |
| ------------------------------------------- | ---------------------------------------------------------- |
| `pnpm run typecheck`                        | `tsc --noEmit` over the build program and the test program |
| `pnpm run lint`                             | ESLint plus the per-directory architecture guards          |
| `pnpm run format` / `pnpm run format:check` | Prettier, write / check                                    |
| `pnpm test`                                 | Vitest, all the tests                                      |
| `pnpm run test:platform`                    | Vitest over `tests/_platform` only                         |
| `pnpm run export:android`                   | The Expo bundle export for Android                         |
| `pnpm run evals`                            | `node --test evals/contracts.test.mjs`                     |
| `pnpm run verify`                           | `bash scripts/verify-template.sh`                          |
| `pnpm run ci`                               | typecheck → lint → test → export:android                   |

`make` mirrors them: `make ci`, `make test`, `make lint`, `make typecheck`,
`make build`, `make evals`, `make verify`, `make format-check`.

The `scripts/` folder holds the gates that do not belong to the language
toolchain:

| Script                              | What it proves                                     |
| ----------------------------------- | -------------------------------------------------- |
| `scripts/rename.sh`                 | The placeholder substitution, in the right order   |
| `scripts/verify-template.sh`        | The template, renamed in a temp copy, gate by gate |
| `scripts/lint-agents.sh`            | The agents match the schema                        |
| `scripts/lint-skills.sh`            | The skills match the schema                        |
| `scripts/lint-commands.sh`          | The commands match the schema                      |
| `scripts/instruction-budget.sh`     | The always-loaded instruction set fits the budget  |
| `scripts/leak-guard.sh`             | No placeholder and no secret survives the rename   |
| `scripts/install-claude-runtime.sh` | The `.claude` layer lands in the target runtime    |

## 6. The template verification

`pnpm run verify` is the gate for the _template_, not for a project. It
stages a throwaway copy of the tree in a temp directory, renames it with
`scripts/rename.sh`, and runs every gate on the instantiated copy:

```text
rename · leak-guard (placeholders) · lint-agents · lint-skills ·
lint-commands · instruction-budget · bash -n · pnpm install · typecheck ·
lint · test · export (android) · evals (node --test) · run-live no-op guard
```

Each stage prints one `PASS` or `FAIL` line and the script exits non-zero if
any stage failed. The working tree is never mutated.

## 7. What runs where

| Gate                    | Runs on every PR | Runs nightly | Runs per release |
| ----------------------- | ---------------- | ------------ | ---------------- |
| Typecheck and lint      | ✔                | ✔            | ✔                |
| Unit and contract tests | ✔                | ✔            | ✔                |
| Architecture suite      | ✔                | ✔            | ✔                |
| Android export          | ✔                | ✔            | ✔                |
| Evals                   | ✔                | ✔            | ✔                |
| Template verification   | ✔                | ✔            | ✔                |
| Emulator E2E            | smoke            | full         | full             |
| Device-pool performance | —                | ✔            | ✔                |
| Push delivery suite     | —                | ✔            | ✔                |
| Store-readiness checks  | —                | —            | ✔                |

The last four rows are **not shipped** by the template: they need a device
pool, a provider sandbox and a store account
([Device Support Matrix](device-support-matrix.md)).

## 8. A red test before the code

The loop is the same as in the other kits: write the failing test, watch it
fail for the right reason, then write the smallest change that makes it
pass. `pnpm test -- <file>` runs one file.

For a new port, the test comes first and it is the fake that comes with it:
the port is declared, the fake implements it, the use-case is written against
it, and only then does the real adapter appear.

## 9. The adoption steps

| Step                                          | Why it matters                                                                                                           |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| A device-pool job with real Low-tier phones   | An emulator is not the floor ([02](02-device-tiers-and-the-quality-governor.md))                                         |
| A push sandbox (FCM/APNs) test                | The permission and the rotation paths are device-only                                                                    |
| A secret scan in CI                           | The bundle is the attack surface ([09](09-security-and-secure-store.md))                                                 |
| A dependency audit                            | The supply chain is part of the store review                                                                             |
| A bundle-size budget per variant              | The download size is the install rate                                                                                    |
| A second architecture program for a new slice | The ESLint scopes and the suite are per-directory today ([03](03-hexagonal-layers-and-mvvm.md#8-the-architecture-suite)) |

## Do / Don't

| Do                                            | Don't                              |
| --------------------------------------------- | ---------------------------------- |
| Test the domain and the ports, not the device | Mock `expo-*` globally             |
| Put the test next to the slice it covers      | Collect every test in one folder   |
| Run `pnpm run ci` before declaring it done    | Run only the tests you wrote       |
| Keep the fake small and hand-written          | Add a mocking library for one test |

## See also

- [Playbooks](13-playbooks.md)
- [Device Support Matrix](device-support-matrix.md)
- [The agent runtime](../.claude/)
