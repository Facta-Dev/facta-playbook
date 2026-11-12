// exampletool — the platform isolation architecture test.
//
// PLAYBOOK §3.1: src/platform is the only place that touches chrome.*. Every
// other file goes through the platform adapters. This test reads the sources
// and fails when a file outside src/platform imports the extension API or
// uses the chrome global.
//
// The scan is a static read of the sources: no extension API, no bundler and
// no dependency beyond the TypeScript compiler the kit already ships.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  createSourceFile,
  forEachChild,
  isExpression,
  isTypeNode,
  ScriptKind,
  ScriptTarget,
  SyntaxKind,
  type Node,
} from "typescript";
import { describe, expect, it } from "vitest";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

/** The folders that hold code. */
const SCANNED = ["src", "entrypoints"];

/** The one folder that owns chrome.*. */
const PLATFORM = "src/platform";

/** The modules that carry the extension API. */
const CHROME_MODULES = ["chrome", "webextension-polyfill"];

function collectFiles(folder: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(folder, { withFileTypes: true })) {
    const full = join(folder, entry.name);
    if (entry.isDirectory()) files.push(...collectFiles(full));
    else if (entry.name.endsWith(".ts") || entry.name.endsWith(".tsx")) files.push(full);
  }
  return files;
}

/** True when the identifier is a property key, not a reference to the global. */
function isPropertyKey(node: Node): boolean {
  const parent = node.parent;
  if (parent === undefined) return false;
  if (!("name" in parent) || parent.name !== node) return false;
  if ("shorthand" in parent && parent.shorthand === true) return false;
  return true;
}

/**
 * True when the identifier is used in a type position: `chrome.runtime.X` as
 * a type is a compile-time reference, not a use of the chrome global.
 */
function isTypePosition(node: Node): boolean {
  let current = node.parent;
  while (current !== undefined) {
    if (isTypeNode(current)) return true;
    if (isExpression(current)) return false;
    current = current.parent;
  }
  return false;
}

/** The violations of one file: the reason and the line. */
function findViolations(file: string, text: string): string[] {
  const source = createSourceFile(file, text, ScriptTarget.Latest, true, ScriptKind.TS);
  const violations: string[] = [];

  const visit = (node: Node): void => {
    if (node.kind === SyntaxKind.ImportDeclaration) {
      const specifier = (node as unknown as { moduleSpecifier: { text: string } }).moduleSpecifier
        .text;
      if (CHROME_MODULES.some((name) => specifier === name || specifier.startsWith(`${name}/`))) {
        violations.push(`${file}: imports "${specifier}"`);
      }
    }
    if (node.kind === SyntaxKind.Identifier) {
      const identifier = node as unknown as { text: string };
      if (identifier.text === "chrome" && !isPropertyKey(node) && !isTypePosition(node)) {
        const { line } = source.getLineAndCharacterOfPosition(node.getStart(source));
        violations.push(`${file}:${line + 1}: uses the chrome global`);
      }
    }
    forEachChild(node, visit);
  };

  visit(source);
  return violations;
}

describe("architecture", () => {
  it("keeps chrome.* inside src/platform", () => {
    const violations: string[] = [];

    for (const folder of SCANNED) {
      for (const file of collectFiles(join(ROOT, folder))) {
        const relativePath = relative(ROOT, file);
        if (relativePath.startsWith(`${PLATFORM}/`)) continue;
        violations.push(...findViolations(relativePath, readFileSync(file, "utf8")));
      }
    }

    expect(
      violations,
      `chrome.* belongs to ${PLATFORM}/ — import the platform adapter instead:\n${violations.join("\n")}`,
    ).toEqual([]);
  });
});
