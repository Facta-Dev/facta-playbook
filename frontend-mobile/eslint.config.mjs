// <app> — ESLint 9 flat config.
//
// Two layers of rules:
//   - type-aware TypeScript rules over the packages, the app and the tests;
//   - architecture guards: react-native and expo belong to the app and the
//     adapters, never to the core, the contracts, the domain, the
//     application layer or the view models.
//
// Every rule is scoped by directory, never by file name, so a new file can
// be added without touching this config.
import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";

const reactNativeGlobals = {
  ...globals.node,
  ...globals.es2021,
  __DEV__: "readonly",
  global: "readonly",
};

export default tseslint.config(
  {
    ignores: ["node_modules/**", "dist/**", ".expo/**", "coverage/**", ".claude/**", "**/*.cjs"],
  },
  js.configs.recommended,
  {
    // TypeScript files outside the type-aware set still need the TS parser.
    files: ["**/*.{ts,tsx,mts}"],
    languageOptions: { parser: tseslint.parser },
  },
  {
    // Type-aware rules: the packages, the app, the configs and the tests.
    files: [
      "packages/*/src/**/*.{ts,tsx}",
      "app/**/*.{ts,tsx}",
      "tests/**/*.{ts,tsx}",
      "vitest.config.mts",
    ],
    extends: [tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.test.json"],
        tsconfigRootDir: import.meta.dirname,
      },
      globals: reactNativeGlobals,
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
      "**/*.config.mjs",
      "vitest.config.mts",
      "tests/**/*.{ts,tsx,mts}",
      "evals/**/*.{js,mjs}",
    ],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    // The core and the contracts are pure TypeScript: no React, no React
    // Native, no Expo. A device capability or a notification payload that
    // needs a native module is a port (packages/core/src/ports.ts), not an
    // import.
    files: ["packages/core/src/**/*.ts", "packages/contracts/src/**/*.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            { group: ["react", "react/*", "react-native", "react-native/*", "expo", "expo/*"] },
          ],
        },
      ],
    },
  },
  {
    // The domain and the application layer are pure TypeScript as well
    // (PLAYBOOK_FRONTEND_MOBILE.md §4.1).
    files: [
      "packages/feature-notes/src/domain/**/*.ts",
      "packages/feature-notes/src/application/**/*.ts",
    ],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            { group: ["react", "react/*", "react-native", "react-native/*", "expo", "expo/*"] },
          ],
        },
      ],
    },
  },
  {
    // The view models are framework-free too: React enters through the hook
    // adapter in views/, React Native never enters the slice at all.
    files: ["packages/feature-notes/src/viewmodels/**/*.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [{ group: ["react-native", "react-native/*", "expo", "expo/*"] }],
        },
      ],
    },
  },
  {
    // A view never reaches for an adapter or a repository: it renders the
    // view-model state and calls the view-model commands.
    files: ["packages/feature-notes/src/views/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [{ group: ["**/adapters/**", "**/*repository*"] }],
        },
      ],
    },
  },
);
