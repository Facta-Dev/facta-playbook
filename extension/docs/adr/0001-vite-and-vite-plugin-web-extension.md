# ADR 0001 — Vite and vite-plugin-web-extension

- **Status**: Accepted
- **Date**: 2026
- **Scope**: the build of the extension template

## Context

A Chrome extension is a set of HTML entry points and one background script
that must ship as plain files with predictable names, plus a React surface
that needs a fast development loop. The template needed a bundler that:

1. produces an unpacked extension in `dist/` that Chrome can load directly
   (`chrome://extensions` → _Load unpacked_);
2. writes `dist/manifest.json` from one source of truth, so the manifest is
   never edited by hand;
3. keeps the entry-point names in the manifest in sync with the files on
   disk;
4. supports TypeScript, React and fast rebuilds without adding a second
   build system.

The backend kits in this repository use different toolchains; the extension
needed one that produces a browser bundle rather than a server binary.

## Decision

**Use Vite 7 with `vite-plugin-web-extension` 4.5, with `entrypoints/` as the
Vite `root`.**

```ts
// vite.config.ts
export default defineConfig({
  root: "entrypoints",
  publicDir: "../public",
  resolve: {
    alias: [
      { find: /^@platform\//, replacement: `${resolve(srcRoot, "platform")}/` },
      { find: /^@features\//, replacement: `${resolve(srcRoot, "features")}/` },
      { find: /^@shared\//, replacement: `${resolve(srcRoot, "shared-kernel")}/` },
    ],
  },
  plugins: [
    react(),
    webExtension({
      manifest: () => manifest,
      additionalInputs: [],
      browser: null as never,
      disableAutoLaunch: true,
    }),
  ],
  build: { outDir: "../dist", emptyOutDir: true },
});
```

The consequences that matter:

- **`entrypoints/` is the root.** Every file under it is an entry point: the
  worker, the side panel page, the options page and the content script.
  `src/` holds the code they wire together.
- **The manifest is a module.** `src/manifest.ts` exports a typed
  `chrome.runtime.ManifestV3`; the plugin writes it to `dist/manifest.json`.
  The manifest is reviewed as code and type-checked like code.
- **`public/` is copied verbatim** into `dist/`, which is where the default
  locale (`_locales/en/messages.json`) lives and where a file such as the
  managed policy schema has to be placed for an enterprise deployment.
- **Aliases mirror `tsconfig.json`** (`@platform/`, `@features/`,
  `@shared/`), so the editor, the bundler and the tests agree on the path
  of an import.

## Alternatives considered

| Option                                       | Why not                                                                                                                                                                                                                                          |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Plain Vite with a hand-written manifest copy | Two sources of truth for the manifest; the entry file names drift from the manifest                                                                                                                                                              |
| `@wxt/extension`                             | A larger framework with its own conventions; the kit would depend on its entry-point and manifest conventions instead of the Chrome ones. The playbook discusses WXT as an option, but the template stays on plain Vite and plain manifest APIs. |
| esbuild with a shell script                  | The manifest generation, the HTML entry points and the dev server would all be hand-written; more script to maintain, less to review.                                                                                                            |
| No bundler (native ES modules)               | Chrome MV3 forbids remotely hosted code but allows local modules only with `"type": "module"`, which the worker does not use (see ADR 0002); the React build step would still need a bundler.                                                    |

## Consequences

- The build output is a plain unpacked extension: `dist/manifest.json`,
  `dist/background.js`, `dist/sidepanel/index.html` and `index.js`,
  `dist/options/index.html` and `index.js`, `dist/content/example.content.js`,
  the shared chunk `dist/logger.js` and `dist/_locales/en/messages.json`.
- The plugin resolves the HTML entry points and their script names; a file
  renamed in `entrypoints/` must be renamed in the manifest too.
- `disableAutoLaunch: true` keeps the plugin from opening a browser during
  a build; `additionalInputs: []` keeps the input list explicit.
- Adding a new surface means adding a file under `entrypoints/` and a path
  in `src/manifest.ts` — two lines, both reviewable.

## See also

- [ADR 0002 — Classic service worker](0002-classic-service-worker.md)
- [Directory Structure & Project Skeleton](../05-directory-structure.md)
- [Testing & CI](../12-testing-and-ci.md)
