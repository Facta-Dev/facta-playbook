# Directory Structure

The annotated map of the kit. Everything is generic until
`scripts/rename.sh` runs: the package names and the app slug are
placeholders, and the project cannot build before that.

## 1. The tree

```text
frontend-mobile/
├── package.json              # workspace root: the scripts and the dev dependencies
├── pnpm-workspace.yaml       # the workspace: app, packages/*
├── tsconfig.json             # the build program: the packages, the app, the configs
├── tsconfig.test.json        # the test program: the same, plus tests/
├── vitest.config.mts         # the unit test configuration and the aliases
├── eslint.config.mjs         # the type-aware rules and the per-directory guards
├── Makefile                  # the same gates, as make targets
├── app/                      # the Expo app — the composition root
│   ├── package.json          # depends on the workspace packages, never on the reverse
│   ├── app.json              # the Expo config: name, slug, scheme, bundle id
│   ├── tsconfig.json         # the app program: it extends the workspace one
│   ├── metro.config.js       # the bundler: it watches the whole workspace
│   ├── index.ts              # registers the root component
│   └── App.tsx               # the composition root: builds the adapters, wires the ports
├── packages/
│   ├── core/                 # Result · DeviceTier · QualityGovernor · the four ports
│   │   └── src/
│   │       ├── result.ts     # Result<T, E>
│   │       ├── device-tier.ts # DeviceTier and the tier resolution
│   │       ├── quality-governor.ts # the governor: baseProfile, governQuality, mayStepUp
│   │       ├── ports.ts      # PushPort · SecureStorePort · NetworkStatePort · DeepLinkPort
│   │       └── index.ts      # the barrel
│   ├── contracts/            # zod schemas and the typed route table
│   │   └── src/
│   │       ├── notes.ts      # the notes DTOs
│   │       ├── routes.ts     # the typed route table, the builders and the parser
│   │       └── index.ts      # the barrel
│   └── feature-notes/        # the example slice
│       └── src/
│           ├── domain/       # the rules, pure
│           ├── application/  # the port and the use-cases
│           ├── adapters/out/ # the implementations of the port
│           ├── viewmodels/   # presentation state and commands, framework-free
│           ├── views/        # the React Native surface and the React binding
│           └── index.ts      # the barrel
├── tests/                    # vitest: the slices, the contracts, the platform
│   ├── _platform/architecture/
│   ├── contracts/
│   ├── core/
│   └── notes/unit/
├── evals/                    # node --test contract evals for the .claude kit
├── scripts/                  # rename, verify-template, lints, instruction-budget, leak-guard
├── .claude/                  # agents, skills, commands, schemas (the kit runtime)
└── docs/                     # this set
```

There is no `src/` inside `app/`: the app is a composition root, and the
entry and the root component sit at its root next to the config.

`app/metro.config.js` exists because the app is one package of a workspace:
Metro watches the workspace root, so it can follow the symlinks pnpm creates
for the shared packages, and it resolves `node_modules` from the app first and
from the root second — so React and React Native are never duplicated.

## 2. The workspace

The root `pnpm-workspace.yaml` lists the members:

```yaml
packages:
  - app
  - packages/*
```

The root `package.json` holds the **scripts**, not the code. Every gate is
run from the root, on every package:

| Script           | What it runs                                                             |
| ---------------- | ------------------------------------------------------------------------ |
| `typecheck`      | `tsc -p tsconfig.json --noEmit` and `tsc -p tsconfig.test.json --noEmit` |
| `lint`           | `eslint .` — type-aware rules plus the architecture guards               |
| `format`         | `prettier --write .`                                                     |
| `format:check`   | `prettier --check .`                                                     |
| `test`           | `vitest run`                                                             |
| `test:platform`  | `vitest run tests/_platform` — the architecture suite alone              |
| `export:android` | `pnpm --filter app exec expo export --platform android`                  |
| `evals`          | `node --test evals/contracts.test.mjs`                                   |
| `verify`         | `bash scripts/verify-template.sh`                                        |
| `ci`             | typecheck → lint → test → export:android                                 |

`pnpm run ci` does **not** run the evals: the evals check the `.claude`
layer, and they are a separate gate (`make evals`, `pnpm run evals`).

The `Makefile` mirrors them as `install`, `build`, `test`, `lint`,
`typecheck`, `format`, `format-check`, `evals`, `verify` and `ci`.

The lockfile is **never committed**: the template ships no lockfile and the
first install happens on the renamed copy. `.npmrc` sets
`node-linker=hoisted`, because React Native and Metro need **one** copy of
`react` and `react-native` in the tree.

## 3. The path aliases

The workspace packages are imported **by their published name**, never by a
relative path across the tree. `tsconfig.json` maps each name to the
TypeScript source:

| Alias                      | Points to                               | Rule                                       |
| -------------------------- | --------------------------------------- | ------------------------------------------ |
| `@<scope>/core`            | `packages/core/src/index.ts`            | The kernel: no device, no framework        |
| `@<scope>/contracts`       | `packages/contracts/src/index.ts`       | The wire shapes and the route table        |
| `@<scope>/feature-<slice>` | `packages/feature-<slice>/src/index.ts` | Sealed: no feature imports another feature |

`vitest.config.mts` declares the same three aliases, with **scope-agnostic**
patterns (`/^@[^/]+\/core$/` and so on): the scope token is substituted by
`scripts/rename.sh`, and the alias must survive that rename.

Each package's `package.json` points `main`, `types` and `exports` at
`src/index.ts`, so the name resolves to the source in the editor, in the
typecheck, in the tests and in the bundle — with no build step in between
([ADR 0002](adr/0002-source-only-workspace-packages.md)).

## 4. The tests mirror the code

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

The tests live outside the packages, in a tree that mirrors them: a slice's
tests are under `tests/<slice>/unit/`, the shared kernel's under
`tests/core/`, the contracts' under `tests/contracts/`, and the platform's
under `tests/_platform/`.

There is no mocking library in the dependencies: the fakes are small
hand-written objects that implement the ports. A fake that is ten lines long
is cheaper than a framework, and it keeps the port boundary visible: if a
fake gets awkward, the port is wrong. The `add-port` skill puts the shared
fakes in `tests/_shared/fakes/`.

## 5. The project skeleton you copy

To start a project from this template:

```sh
cp -a frontend-mobile/ my-app/          # copy, then work in the copy
cd my-app/
bash scripts/rename.sh mycompany acme    # <app> slug first, then <scope>
pnpm install
pnpm run format                          # the substitution re-aligns the markdown tables
pnpm run ci
```

`scripts/rename.sh` substitutes the placeholder tokens across every text
file (the `scripts/` folder itself and the frozen eval fixtures are
excluded), removes the lockfile, and tells you to run `pnpm run format` once.
The lockfile is never committed.

Both values ship as identifiers — the npm package `@<scope>/<app>`, the
Android `applicationId` and the iOS bundle identifier `com.<scope>.<app>` —
so the script accepts only a lowercase identifier (a letter first, then
letters, digits or underscore) and rejects anything else **before** it
touches the tree.

Then replace the `notes` slice with your own feature, following
[Playbooks](13-playbooks.md).

> **Never install in the template root.** The template ships no lockfile; an
> install in the template root creates one that the rename script then
> deletes, and the install is repeated on every copy. Install on the renamed
> copy.

## Do / Don't

| Do                                                | Don't                                       |
| ------------------------------------------------- | ------------------------------------------- |
| Put the composition root in `app/`                | Put a composition root in a feature package |
| Import through the package name (`@<scope>/core`) | Use long relative paths across the tree     |
| Keep the tests next to the slice they cover       | Put every test in one folder                |
| Run `pnpm run format` after a rename              | Commit the lockfile                         |
| Install on the renamed copy                       | Run `pnpm install` in the template root     |

## See also

- [Vertical Slicing](04-vertical-slicing.md)
- [Playbooks](13-playbooks.md)
- [ADR 0002 — Source-only workspace packages](adr/0002-source-only-workspace-packages.md)
