// <app> — the architecture boundaries.
//
// The hexagon is a contract, not a convention: this suite reads the source
// files and fails the build when a layer reaches for a layer it must not
// know (PLAYBOOK_FRONTEND_MOBILE.md §4.1).

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { NOTES_STATUSES } from "../../../packages/feature-notes/src/viewmodels/notes-view-model";

const ROOT = fileURLToPath(new URL("../../..", import.meta.url));

/** The eight statuses of the client contract. */
const PLAYBOOK_STATUSES = [
  "loading",
  "ready",
  "empty",
  "forbidden",
  "not-found",
  "offline",
  "error",
  "policy-blocked",
];

/** Every TypeScript source file under a directory, recursively. */
function sourceFiles(directory: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...sourceFiles(path));
      continue;
    }
    if (entry.name.endsWith(".ts") || entry.name.endsWith(".tsx")) {
      files.push(path);
    }
  }
  return files;
}

/** The module specifiers a source file imports. */
function importSpecifiers(source: string): string[] {
  const specifiers: string[] = [];
  const pattern = /(?:from|import|require)\s*\(?\s*["']([^"']+)["']/g;
  let match = pattern.exec(source);
  while (match !== null) {
    if (match[1] !== undefined) specifiers.push(match[1]);
    match = pattern.exec(source);
  }
  return specifiers;
}

/** The files of a directory that import a forbidden module. */
function violations(directory: string, isForbidden: (specifier: string) => boolean): string[] {
  const found: string[] = [];
  for (const file of sourceFiles(join(ROOT, directory))) {
    const source = readFileSync(file, "utf8");
    for (const specifier of importSpecifiers(source)) {
      if (isForbidden(specifier)) {
        found.push(`${relative(ROOT, file)} imports '${specifier}'`);
      }
    }
  }
  return found;
}

const isUiFramework = (specifier: string): boolean =>
  /^(react|react-native|expo)(\/|$)/.test(specifier);

const isNativeFramework = (specifier: string): boolean =>
  /^(react-native|expo)(\/|$)/.test(specifier);

const isAdapter = (specifier: string): boolean => /adapters|repository/i.test(specifier);

describe("the layer boundaries", () => {
  it("keeps react, react-native and expo out of the core", () => {
    expect(violations("packages/core", isUiFramework)).toEqual([]);
  });

  it("keeps react, react-native and expo out of the contracts", () => {
    expect(violations("packages/contracts", isUiFramework)).toEqual([]);
  });

  it("keeps react, react-native and expo out of the domain", () => {
    expect(violations("packages/feature-notes/src/domain", isUiFramework)).toEqual([]);
  });

  it("keeps react, react-native and expo out of the application layer", () => {
    expect(violations("packages/feature-notes/src/application", isUiFramework)).toEqual([]);
  });

  it("keeps react-native and expo out of the view models", () => {
    expect(violations("packages/feature-notes/src/viewmodels", isNativeFramework)).toEqual([]);
  });

  it("keeps the adapters and the repositories out of the views", () => {
    expect(violations("packages/feature-notes/src/views", isAdapter)).toEqual([]);
  });
});

describe("the view-model contract", () => {
  it("carries the eight statuses of the playbook", () => {
    for (const status of PLAYBOOK_STATUSES) {
      expect(NOTES_STATUSES).toContain(status);
    }
  });
});
