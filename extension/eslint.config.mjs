// <app> — ESLint 9 flat config.
//
// Two layers of rules:
//   - type-aware TypeScript rules over the extension sources, the build
//     configs and the tests;
//   - architecture guards: chrome.* belongs to src/platform/**, and features
//     never import each other.
//
// Every rule is scoped by directory, never by file name, so a new file can be
// added without touching this config.
import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["dist/**", "node_modules/**", "coverage/**", ".claude/**", "**/*.cjs"],
  },
  js.configs.recommended,
  {
    // TypeScript files outside the type-aware set still need the TS parser.
    files: ["**/*.{ts,tsx}"],
    languageOptions: { parser: tseslint.parser },
  },
  {
    // Type-aware rules: sources, build configs and tests.
    files: [
      "src/**/*.{ts,tsx}",
      "entrypoints/**/*.{ts,tsx}",
      "tests/**/*.{ts,tsx}",
      "vite.config.ts",
      "vitest.config.ts",
    ],
    extends: [tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.test.json"],
        tsconfigRootDir: import.meta.dirname,
      },
      globals: { ...globals.browser, chrome: "readonly" },
    },
    rules: {
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-explicit-any": "error",
    },
  },
  {
    // Node-side files: build configs, test scripts and the evals.
    files: [
      "vite.config.ts",
      "vitest.config.ts",
      "**/*.config.mjs",
      "tests/**/*.{ts,tsx}",
      "evals/**/*.{ts,mjs}",
      "scripts/**/*.{ts,mjs}",
    ],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    // chrome.* is the platform layer's job (PLAYBOOK §3.1 and §4): feature
    // code goes through src/platform/**, never the raw extension API.
    files: ["src/**/*.{ts,tsx}", "entrypoints/**/*.{ts,tsx}"],
    ignores: ["src/platform/**"],
    rules: {
      "no-restricted-globals": [
        "error",
        {
          name: "chrome",
          message: "chrome.* belongs to src/platform/** — import the platform adapter instead.",
        },
        {
          name: "browser",
          message: "browser.* belongs to src/platform/** — import the platform adapter instead.",
        },
      ],
    },
  },
  {
    // Composition roots (entrypoints) are the one place allowed to hold the
    // raw extension API — they wire the layers, they do not implement them.
    files: ["entrypoints/**"],
    rules: { "no-restricted-globals": "off" },
  },
  {
    // Features are sealed: a feature never imports another feature. Import
    // inside your own slice with a relative path; the composition root wires
    // the slices together.
    files: ["src/features/**"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              regex: "^@features/",
              message: "features are sealed — import inside your own slice with a relative path",
            },
          ],
        },
      ],
    },
  },
);
