# Testing & CI

The gates are the contract. A change is finished when `make ci` is green and
the template verification passes; a change to the kit's own agent runtime is
finished when the evals pass.

## 1. The test tiers

| Tier           | Tool            | What it covers                                                                   | Command                             |
| -------------- | --------------- | -------------------------------------------------------------------------------- | ----------------------------------- |
| Unit           | Vitest          | Domain rules, view models, the policy resolver                                   | `pnpm test`                         |
| Contract       | Vitest          | The message server and the protocol                                              | `pnpm test`                         |
| Architecture   | Vitest (static) | No `chrome` outside `src/platform`, no cross-feature import                      | `pnpm run test:platform`            |
| Evals          | `node --test`   | The `.claude` kit: agents, skills, commands, manifest posture, placeholder leaks | `pnpm run evals`                    |
| SDK self-check | `node`          | The injectable SDK, including its failure paths                                  | `pnpm --dir packages/sdk run check` |
| Template       | bash            | The whole template, renamed in a temp dir                                        | `pnpm run verify`                   |

## 2. The layout

```text
tests/
├── _platform/architecture/platform-isolation.test.ts
├── _platform/security/sender-allowlist.test.ts
├── _shared/fakes/{fake-runtime.ts,fake-storage.ts}
├── _shared/result.ts
└── notes/frontend/
    ├── unit/domain/note.test.ts
    ├── unit/storage/notes-store.test.ts
    ├── unit/viewmodels/notes-view-model.test.ts
    ├── unit/policy/policy-resolver.test.ts
    └── contract/message-server.test.ts
```

The tests mirror the code: `tests/<slice>/frontend/{unit,contract}/` for a
slice, `tests/_platform/` for the platform and the architecture, `tests/_shared/`
for the fakes.

## 3. Fakes, not mocks

There is no mocking library in the dependencies. `tests/_shared/fakes/fake-runtime.ts`
and `fake-storage.ts` are hand-written objects that implement the platform
ports (`RuntimePort`, `StorageArea`). A fake that is ten lines long is
cheaper than a framework, and it makes the port boundary visible: if a fake
gets awkward, the port is wrong.

The message server is tested through its `RuntimePort`, so the test never
opens a browser: the fake records the listener and lets the test deliver a
message with a chosen sender.

## 4. The architecture test is static

`tests/_platform/architecture/platform-isolation.test.ts` walks the tree with
the TypeScript compiler API and fails when `chrome` is used outside
`src/platform`. It is not a lint opinion; it is a test, so it runs in
`pnpm test` and not only in `pnpm lint`.

The same rules are enforced twice on purpose:

| Rule                               | ESLint                  | dependency-cruiser           | Static test                  |
| ---------------------------------- | ----------------------- | ---------------------------- | ---------------------------- |
| No `chrome` outside `src/platform` | `no-restricted-globals` | `no-chrome-outside-platform` | `platform-isolation.test.ts` |
| No feature imports a feature       | `no-restricted-imports` | `no-cross-feature-imports`   | —                            |
| No domain → application            | —                       | `no-domain-to-application`   | —                            |
| No application → adapters          | —                       | `no-application-to-adapters` | —                            |
| No cycles                          | —                       | `no-circular`                | —                            |
| No SDK → React or `chrome`         | —                       | `no-sdk-react-or-chrome`     | —                            |

## 5. The commands

| Command                                     | What it runs                                                         |
| ------------------------------------------- | -------------------------------------------------------------------- |
| `pnpm run build`                            | `vite build` — the unpacked extension in `dist/`                     |
| `pnpm run typecheck`                        | `tsc` over the app and the test projects                             |
| `pnpm run lint`                             | `eslint .` then `depcruise --config dependency-cruiser.config.cjs .` |
| `pnpm run format` / `pnpm run format:check` | Prettier, write / check                                              |
| `pnpm test`                                 | Vitest, all the tests                                                |
| `pnpm run test:platform`                    | Vitest, only `tests/_platform`                                       |
| `pnpm run evals`                            | `node --test evals/contracts.test.mjs`                               |
| `pnpm run verify`                           | `bash scripts/verify-template.sh`                                    |
| `pnpm run ci`                               | typecheck → lint → test → build                                      |

The `Makefile` exposes the same gates as `make ci`, `make build`,
`make test`, `make lint`, `make format`, `make format-check` and
`make verify`.

## 6. The template verification

`pnpm run verify` is the gate for the _template_, not for a project. It
stages a throwaway copy of the tree in a temp directory, renames it with
`scripts/rename.sh`, and runs every gate on the instantiated copy:

```text
rename · leak-guard (placeholders) · lint-agents · lint-skills ·
lint-commands · instruction-budget · bash -n · pnpm install · typecheck ·
lint · test · build · evals · run-live no-op guard
```

Each stage prints one `PASS` or `FAIL` line and the script exits non-zero if
any stage failed. The working tree is never mutated.

## 7. A red test before the code

The loop is the same as in the backend kits: write the failing test, watch it
fail for the right reason, then write the smallest change that makes it
pass. `pnpm test -- <file>` runs one file.

## Do / Don't

| Do                                             | Don't                              |
| ---------------------------------------------- | ---------------------------------- |
| Test the domain and the ports, not the browser | Mock `chrome` globally             |
| Put the test next to the slice it covers       | Collect every test in one folder   |
| Run `make ci` before declaring a change done   | Run only the tests you wrote       |
| Keep the fake in `tests/_shared/fakes`         | Add a mocking library for one test |

## See also

- [Playbooks](13-playbooks.md)
- [ADR 0001 — Vite and vite-plugin-web-extension](adr/0001-vite-and-vite-plugin-web-extension.md)
- [The agent guide](../AGENTS.md)
