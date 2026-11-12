# Directory Structure & Project Skeleton

The annotated map of the kit. Everything is generic until
`scripts/rename.sh` runs: the package name and the manifest name are
placeholders, and the project cannot build before that.

## 1. The tree

```text
extension/
├── package.json              # scripts: build, typecheck, lint, format, test, evals, verify, ci
├── Makefile                  # the same gates as make targets (ci, build, test, lint, verify)
├── vite.config.ts            # root: entrypoints/, outDir: dist/, aliases for the three layers
├── vitest.config.ts          # include: tests/**/*.{test,spec}.{ts,tsx,mts}
├── eslint.config.mjs         # flat config; bans `chrome` outside src/platform
├── dependency-cruiser.config.cjs  # the six architecture rules
├── tsconfig.json             # app build; tsconfig.test.json adds tests/
├── managed-schema.json       # the administrator policy schema (see admin-guide.md)
├── src/
│   ├── manifest.ts           # THE manifest — the single source of truth
│   ├── contracts/            # zod schemas shared by both ends (protocol, notes, policy, storage, errors)
│   ├── platform/             # runtime, storage, sidepanel — the only folder that touches chrome.*
│   ├── security/             # sender classification and match patterns
│   ├── messaging/            # server, client, the notes handlers
│   ├── policy/               # reader + resolver (administrator over user)
│   ├── storage/              # the typed notes store
│   ├── telemetry/            # the redacting logger
│   ├── shared-kernel/        # result, ids, error codes
│   └── features/notes/       # the example slice
├── entrypoints/              # the composition roots (Vite root)
│   ├── background.ts
│   ├── sidepanel/{index.html,main.tsx}
│   ├── options/{index.html,main.ts}
│   └── content/example.content.ts
├── public/_locales/en/       # the default locale
├── packages/sdk/             # the injectable client SDK (no chrome, no React)
├── tests/                    # unit, contract and architecture tests
├── evals/                    # native node --test contract evals for the .claude kit
├── scripts/                  # rename, verify, leak-guard, linters, budget
├── .claude/                  # agents, skills, commands, schemas (the kit runtime)
└── docs/                     # this set
```

## 2. `entrypoints/` is the Vite root

`vite.config.ts` sets `root: "entrypoints"` and `publicDir: "../public"`.
The plugin turns each entry file into a Chrome entry point and writes
`dist/manifest.json` from `src/manifest.ts`. The consequence: **`src/` holds
code, `entrypoints/` holds composition roots.** A file in `entrypoints/` may
wire the layers together and may touch `chrome.*`; a file in `src/` may not
(except `src/platform/**`).

## 3. The path aliases

`tsconfig.json` and `vite.config.ts` declare the same three aliases, so a
import reads the same in the editor, in the bundler and in the tests:

| Alias         | Points to             | Rule                                       |
| ------------- | --------------------- | ------------------------------------------ |
| `@platform/*` | `src/platform/*`      | The only place `chrome.*` is allowed       |
| `@features/*` | `src/features/*`      | Sealed: no feature imports another feature |
| `@shared/*`   | `src/shared-kernel/*` | The kernel: no browser, no framework       |

## 4. The tests mirror the code

```text
tests/
├── _platform/architecture/platform-isolation.test.ts  # static check: no chrome outside src/platform
├── _platform/security/sender-allowlist.test.ts
├── _shared/fakes/{fake-runtime.ts,fake-storage.ts}    # hand-written fakes, no mocking library
├── _shared/result.ts
└── notes/frontend/{unit,contract}/                    # one folder per slice
```

There is no mocking library in the dependencies: the fakes are small
hand-written objects that implement the platform ports. A fake that is ten
lines long is cheaper than a framework that is not installed.

## 5. The project skeleton you copy

To start a project from this template:

```sh
cp -a extension/ my-extension/          # copy, then work in the copy
cd my-extension/
bash scripts/rename.sh <app-slug> <org-scope>
pnpm install
pnpm format                              # the substitution re-aligns the markdown tables
make ci
```

`scripts/rename.sh` substitutes the placeholder tokens across every text
file (the `scripts/` folder itself and the frozen eval fixtures are
excluded), removes the lockfile, and tells you to run `pnpm format` once.
The lockfile is never committed: the template ships no lockfile and the
first install happens after the rename.

Then replace the `notes` slice with your own feature, following
[Playbooks](13-playbooks.md).

## Do / Don't

| Do                                                    | Don't                                     |
| ----------------------------------------------------- | ----------------------------------------- |
| Put a composition root in `entrypoints/`              | Put a composition root in `src/features/` |
| Import through `@platform/`, `@features/`, `@shared/` | Use long relative paths across the tree   |
| Keep the tests next to the slice they cover           | Put every test in one folder              |
| Run `pnpm format` after a rename                      | Commit the lockfile                       |

## See also

- [Vertical Slicing](04-vertical-slicing.md)
- [Playbooks](13-playbooks.md)
- [ADR 0001 — Vite and vite-plugin-web-extension](adr/0001-vite-and-vite-plugin-web-extension.md)
