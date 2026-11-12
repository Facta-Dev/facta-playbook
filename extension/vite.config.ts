// <app> — extension build adapter.
//
// The manifest is generated from src/manifest.ts (never hand-edited). The
// Vite root is entrypoints/ (PLAYBOOK §4): manifest paths are relative to
// that folder, so background.ts publishes as background.js and
// sidepanel/index.html as sidepanel/index.html — no path translation layer.
import { resolve } from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import webExtension from "vite-plugin-web-extension";
import manifest from "./src/manifest";

const srcRoot = resolve(import.meta.dirname, "src");

export default defineConfig({
  root: "entrypoints",
  publicDir: "../public",
  resolve: {
    // Mirrors the "paths" of tsconfig.json.
    alias: [
      { find: /^@platform\//, replacement: `${resolve(srcRoot, "platform")}/` },
      { find: /^@features\//, replacement: `${resolve(srcRoot, "features")}/` },
      { find: /^@shared\//, replacement: `${resolve(srcRoot, "shared-kernel")}/` },
    ],
  },
  plugins: [
    react(),
    webExtension({
      // TS manifest — the plugin accepts a function returning the object.
      manifest: () => manifest,
      // Every page is named in the manifest; extra pages go here.
      additionalInputs: [],
      browser: null as never, // never auto-launch; the e2e harness launches Chrome
      disableAutoLaunch: true,
    }),
  ],
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
});
