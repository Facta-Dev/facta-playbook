// <app> — unit test configuration. Vitest reads this file instead of
// vite.config.ts (whose root is entrypoints/ for the extension build), so the
// test tree stays at the repository root.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.{test,spec}.{ts,tsx,mts}"],
  },
});
