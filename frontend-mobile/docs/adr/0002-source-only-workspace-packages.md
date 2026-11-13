# ADR 0002 — Source-only workspace packages

|                |                                                       |
| -------------- | ----------------------------------------------------- |
| **Status**     | Accepted                                              |
| **Date**       | 2026-11-12                                            |
| **Scope**      | How the workspace packages are published and resolved |
| **Supersedes** | —                                                     |

## Context

The workspace has four kinds of package:

| Package              | What it is                                                     |
| -------------------- | -------------------------------------------------------------- |
| `app`                | The Expo application — the composition root                    |
| `packages/core`      | The kernel: `Result`, the device tier, the governor, the ports |
| `packages/contracts` | The zod schemas and the route table                            |
| `packages/feature-*` | The slices                                                     |

Each is a pnpm workspace package with its own `package.json`. The question
this ADR answers is how a package is **consumed**: does it ship compiled
JavaScript, or does it ship TypeScript source?

The two common answers:

1. **Compiled.** The package has a `build` step that emits `dist/`, and
   `main` points at `dist/index.js`, `types` at `dist/index.d.ts`. The
   consumers import the build output.
2. **Source-only.** The package has no build step; `main` and `types` point
   at `src/index.ts`, and the consumer's bundler compiles it.

The choice is not a detail. It decides whether there is a build step in the
workspace at all, what the watch scripts do, what CI runs, and what happens
when two packages change at once.

The constraints that decided it:

1. **Metro is the bundler.** In a React Native app, the bundler compiles
   the bundle. A separate `tsc` build would compile the same source a second
   time, with a different set of settings, and the two results could
   disagree.
2. **The app is the only entry point.** There is no published npm package
   here; nothing is consumed by a stranger. The workspace is internal.
3. **Expo's ecosystem expects source.** The Expo modules, the config
   plugins and the bundler's transformer all assume TypeScript source in the
   project.
4. **The typecheck must still be real.** A source-only package must be
   type-checked, or the types are a lie. The check happens once, at the
   workspace root, with `tsc --noEmit` over the build program and the test
   program.

## Decision

**The workspace packages ship source. `main`, `types` and `exports` all point
at `src/index.ts`. There is no build step.**

```json
{
  "name": "@<scope>/feature-notes",
  "private": true,
  "type": "module",
  "main": "src/index.ts",
  "types": "src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  }
}
```

Three consequences, and each one has a counterpart that must exist:

| Consequence                                             | The counterpart                                                         |
| ------------------------------------------------------- | ----------------------------------------------------------------------- |
| The bundler compiles the packages as part of the bundle | Each package's `exports` points at the source                           |
| `tsc --noEmit` at the root type-checks everything       | The root `tsconfig.json` includes the packages, the app and the configs |
| There is no `dist/` and no `build` script               | There is also no stale artifact to go out of date                       |

### The name is the resolution

The packages are imported **by their published name**, and the name is what
resolves:

- **The bundler** (Metro, through Expo) reads the `main`/`exports` of the
  workspace package and compiles `src/index.ts` as part of the bundle.
- **`tsc`** maps the same names to the same sources in `tsconfig.json`:

  ```json
  "paths": {
    "@<scope>/core": ["./packages/core/src/index.ts"],
    "@<scope>/contracts": ["./packages/contracts/src/index.ts"],
    "@<scope>/feature-notes": ["./packages/feature-notes/src/index.ts"]
  }
  ```

- **Vitest** declares the same three aliases, with scope-agnostic patterns
  (`/^@[^/]+\/core$/` and so on), so the alias survives the rename.

The three lists must agree. When they drift, the symptom is an import that
resolves in the editor and fails in the bundle — or the reverse.

### TypeScript resolution

The root `tsconfig.json` sets the options that make source resolution work:

```json
{
  "compilerOptions": {
    "strict": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "noEmit": true
  }
}
```

`moduleResolution: "bundler"` is what lets `tsc` resolve
`@<scope>/feature-notes` to `packages/feature-notes/src/index.ts`. `noEmit`
is not incidental: there is nothing to emit.

### The typecheck covers the packages

Because there is no build, the typecheck is the only thing that proves the
packages are sound. The root `typecheck` script runs `tsc --noEmit` over
two programs: `tsconfig.json` (**includes** `packages/*/src`, `app` and
`vitest.config.mts`) and `tsconfig.test.json`, which adds `tests`. A type
error in `packages/core` fails `pnpm run typecheck` exactly as it would
fail a build.

### The lockfile

The lockfile is **never committed**. The template ships no lockfile and the
first install happens on the renamed copy, after `scripts/rename.sh`. A
committed lockfile from the template would pin versions that were never
tested together.

## Alternatives considered

### Compiled packages (`tsc` → `dist/`)

**Rejected.** It buys three things the template does not need:

- **A publishable artifact.** Nothing here is published to a registry; the
  workspace is internal to the app.
- **A faster bundle.** The bundler caches its transform; the second run is
  fast anyway. The build step moves work, it does not remove it.
- **A `dist/` that matches what `tsc` accepts.** A useful property when the
  consumers are on a different toolchain.

And it costs two things the template needs:

- **A second compiler configuration.** The `tsc` build and the bundler's
  transform would have to agree on the JSX runtime, the decorators and the
  path mapping. Every disagreement shows up as a runtime error in the bundle,
  not in the build.
- **A stale-artifact class of bug.** A package changes; the consumer
  imports the old `dist/`. The symptom is a change that "does not take
  effect", which is a bad afternoon.

### A single package with folders instead of a workspace

**Rejected.** It removes the package boundary, and the package boundary is
what makes a slice sealed: the package boundary is where a cross-feature
import has to be declared, and `package.json` declares each slice's
dependencies explicitly.

### Turborepo / Nx for the task graph

**Rejected for now.** The workspace has four packages and no build step, so
there is nothing to cache and no graph to schedule. The caching win appears
when the workspace grows a real build; the tooling cost appears immediately.

### `tsc` with `emitDeclarationOnly` for the types

**Rejected.** It would give a `dist/` of `.d.ts` files and no JavaScript,
which is exactly the "two artifacts" problem: the types would be built and
the code would not, so the types could describe something the bundle does
not contain.

## Consequences

**Accepted:**

- No build step in the workspace. `pnpm run typecheck`, `pnpm test`,
  `pnpm run lint`, `pnpm run export:android` and `pnpm run evals` are the
  whole gate.
- No stale artifacts: what the editor shows is what the bundle runs.
- One compiler configuration, used by the editor, by the typecheck and by
  the bundler.

**Accepted, with a known gap:**

- **The three resolution lists must be kept in step.** The package
  `exports`, the `paths` in `tsconfig.json` and the aliases in
  `vitest.config.mts` declare the same three names. A test that asserts the
  lists agree is an adoption step, not a shipped feature.
- **A package cannot be consumed by a tool that expects JavaScript.** The
  evals, the lint and the typecheck all run in the same workspace and
  understand TypeScript source. A separate consumer of `packages/core` —
  a script, a generator — would need a build. None exists today.
- **`tsc` is not a bundler.** The typecheck does not prove that the bundle
  builds. The template closes the gap with `pnpm run export:android`, which
  is part of `pnpm run ci` — a job that runs it in CI is an adoption step
  ([Testing & CI §9](../12-testing-and-ci.md#9-the-adoption-steps)).

**Consequences for the code:**

- Every package's `package.json` points `main`, `types` and `exports` at
  `src/index.ts`.
- The root `tsconfig.json` includes `packages/*/src`, `app` and the configs;
  `tsconfig.test.json` adds `tests`; `app/tsconfig.json` extends the root
  one.
- `vitest.config.mts` declares the same three aliases, scope-agnostic.
- `scripts/rename.sh` never generates a lockfile and never builds.

## Revisit trigger

Revisit this ADR when:

1. A workspace package must be consumed outside the app — a generator, a
   CI script that cannot read TypeScript, or a published SDK.
2. The workspace grows a build step (a code generator, a native module
   package) and the incremental cost of the build starts to matter.
3. The resolution lists stop matching each other and become a maintenance
   burden.

At that point the change is: give the affected packages a `build` script,
point `main` at `dist/`, and keep `types` at `src` — or move both to
`dist/`. The rule that survives the change is the one that matters: **the
editor, the typecheck and the bundle must agree about what the source
means.**

## See also

- [Directory Structure](../05-directory-structure.md)
- [ADR 0001 — Expo with a plain `App.tsx` entry](0001-expo-with-a-plain-app-entry.md)
- [Testing & CI](../12-testing-and-ci.md)
