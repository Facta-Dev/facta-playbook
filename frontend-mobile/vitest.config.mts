// <app> — unit test configuration.
//
// The workspace packages resolve by their published name; the aliases below
// point at the TypeScript sources so a test never needs a build step.
// The pattern is scope-agnostic on purpose: the scope token is substituted
// by scripts/rename.sh, and the alias must survive that rename.
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const source = (relative: string): string => fileURLToPath(new URL(relative, import.meta.url));

export default defineConfig({
  resolve: {
    alias: [
      { find: /^@[^/]+\/core$/, replacement: source("./packages/core/src/index.ts") },
      { find: /^@[^/]+\/contracts$/, replacement: source("./packages/contracts/src/index.ts") },
      {
        find: /^@[^/]+\/feature-notes$/,
        replacement: source("./packages/feature-notes/src/index.ts"),
      },
    ],
  },
  test: {
    include: ["tests/**/*.{test,spec}.{ts,tsx,mts}"],
    environment: "node",
  },
});
