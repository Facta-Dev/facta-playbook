// contracts.test.mjs — pure-STD contract validator + tests for the .claude
// layer (INV-T16: native `node --test` runner, zero external deps).
//
// Hand-rolled frontmatter parser + body-section checker for agents, skills,
// and commands. The required fields, required sections, allowed divisions,
// and failure modes are HARDCODED as static lists here — this validator does
// NOT read .claude/schemas/*.md at runtime. The schema docs are the source
// of truth and the bash linters (scripts/lint-*.sh) parse them at runtime,
// so linter<->schema drift holds via the linters. This validator is a
// hand-maintained mirror of the schema docs: when a list changes in a schema
// doc you MUST update the matching static list here too.
//
// Fixtures are FROZEN test data: rename.sh excludes evals/fixtures/ from its
// substitution pass, so the invalid leak fixture keeps its literal tokens
// post-rename and keeps exercising the leak detector.
//
// This file carries NO literal template substitution token anywhere: the
// leak-detector tokens are assembled at runtime in PLACEHOLDER_TOKENS and
// the leak error message interpolates them, so the substitution pass cannot
// defuse the detector.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.dirname(HERE); // skeleton root

// The three template substitution tokens, assembled from pieces so this file
// never spells one out literally.
const PLACEHOLDER_TOKENS = ["<" + "app>", "<" + "APP>", "<" + "scope>"];

const KIND_AGENT = "agent";
const KIND_SKILL = "skill";
const KIND_COMMAND = "command";

// Which .claude module kind a document belongs to. Mirrors the schema docs:
// agents.schema.md "Required Frontmatter Fields" / skills + commands schemas.
function requiredFrontmatter(kind) {
  if (kind === KIND_AGENT) {
    return ["name", "description", "color", "division"];
  }
  return ["name", "description", "division"];
}

// Mirror of the schema docs' "Required Body Sections" lists.
function requiredSections(kind) {
  if (kind === KIND_AGENT) {
    return [
      "Core Mission",
      "Expertise Areas",
      "Output Format",
      "Quality Standards",
      "Guiding Principles",
    ];
  }
  if (kind === KIND_SKILL) {
    return ["When to Activate", "Core Principles", "Rules", "Research References", "Output Format"];
  }
  return ["Delegate"]; // command
}

// Shared across agents/skills/commands in this template.
const ALLOWED_DIVISIONS = ["backend", "security", "testing", "core"];

// result of validating one document.
function newResult() {
  return { ok: true, errs: [] };
}
function fail(r, msg) {
  r.ok = false;
  r.errs.push(msg);
}
function joined(r) {
  return r.errs.join("\n");
}

// parseFrontmatter parses a YAML frontmatter block into ordered key/value
// pairs plus the body after the closing fence. Accepts the leading "---\n"
// fence, reads until the closing "---", parses "key: value" lines (stripping
// one layer of surrounding quotes). closed=false when missing/malformed.
function parseFrontmatter(text) {
  const lines = text.split("\n");
  if (lines.length === 0 || lines[0].trim() !== "---") {
    return { fm: [], body: text, closed: false };
  }
  const fm = [];
  const bodyLines = [];
  let closed = false;
  for (const line of lines.slice(1)) {
    if (!closed) {
      if (line.trim() === "---") {
        closed = true;
        continue;
      }
      const t = line.trim();
      if (t === "" || t.startsWith("#")) continue;
      const i = line.indexOf(":");
      if (i >= 0) {
        const key = line.slice(0, i).trim();
        let val = line.slice(i + 1).trim();
        if (val.length >= 2) {
          const a = val[0],
            b = val[val.length - 1];
          if ((a === '"' || a === "'") && a === b) val = val.slice(1, -1);
        }
        fm.push({ key, val });
      }
      continue;
    }
    bodyLines.push(line);
  }
  return { fm, body: bodyLines.join("\n"), closed };
}

function fmHas(fm, key) {
  return fm.some((e) => e.key === key);
}
function fmGet(fm, key) {
  for (const e of fm) if (e.key === key) return e.val;
  return "";
}

// hasSection checks whether the body contains a heading matching section
// (case-insensitive), accepting one or more # prefixes.
function hasSection(body, section) {
  const needle = section.toLowerCase();
  for (const line of body.split("\n")) {
    const t = line.replace(/^[ \t]+/, "");
    if (!t.startsWith("#")) continue;
    const heading = t.slice(1).replace(/^#+/, "");
    if (heading.trim().toLowerCase() === needle) return true;
  }
  return false;
}

// hasPlaceholderLeak checks for residual literal placeholders from the rename
// pass — the three tokens assembled in PLACEHOLDER_TOKENS.
function hasPlaceholderLeak(text) {
  return PLACEHOLDER_TOKENS.some((tok) => text.includes(tok));
}

// validate checks a single document against the rules for kind.
function validate(text, kind) {
  const r = newResult();

  // CRLF check.
  if (text.includes("\r")) {
    fail(r, "CRLF line endings — convert to LF");
  }

  const { fm, body, closed } = parseFrontmatter(text);
  if (!closed) {
    fail(r, "missing or malformed frontmatter (no closing ---)");
    return r;
  }
  if (fm.length === 0) {
    fail(r, "empty frontmatter");
  }

  for (const field of requiredFrontmatter(kind)) {
    if (!fmHas(fm, field)) fail(r, `missing frontmatter key '${field}'`);
  }

  if (fmHas(fm, "division")) {
    const div = fmGet(fm, "division");
    if (!ALLOWED_DIVISIONS.includes(div)) fail(r, `bad division '${div}'`);
  }

  for (const section of requiredSections(kind)) {
    if (!hasSection(body, section)) fail(r, `missing body section '${section}'`);
  }

  if (hasPlaceholderLeak(text)) {
    fail(
      r,
      `literal placeholder leak (${PLACEHOLDER_TOKENS.join("/")}) — rename.sh must substitute all placeholders`,
    );
  }

  return r;
}

// domainImportViolations scans the import lines of a domain.ts text and
// returns one violation per forbidden import. The domain core must import
// stdlib + shared only — no adapter layer (fastify, pg, kysely), no platform
// or generated code, no sibling slice. Mirrors the no-restricted-imports
// group in eslint.config.mjs ("domain-pure"); kept hand-maintained so this
// file needs no eslint parse.
function domainImportViolations(text) {
  const violations = [];
  for (const raw of text.split("\n")) {
    const t = raw.trim();
    // Matches "import ..." and "export ... from" clauses; strips line
    // comments, alias ("Pool as P" / `import x = ` is not ESM, skipped), and
    // captures the single- or double-quoted specifier.
    if (!/^(import|export)[\s{].*\bfrom\b/.test(t) && !/^import\s*['"]/.test(t)) continue;
    const noComment = t.split("//")[0];
    const m = noComment.match(/from\s*['"]([^'"]+)['"]/);
    if (!m) continue;
    const spec = m[1];
    if (spec === "fastify" || spec.startsWith("@fastify/")) {
      violations.push(
        `forbidden import '${spec}' — domain.ts is pure (stdlib + shared only); move it to the slice's http.ts`,
      );
    } else if (spec === "pg" || spec === "kysely") {
      violations.push(
        `forbidden import '${spec}' — persistence is a port; use interfaces in ports.ts (adapter code belongs in the slice's queries.ts)`,
      );
    } else if (spec.startsWith("#platform/")) {
      violations.push(
        `forbidden import '${spec}' — domain depends on its own ports, not platform infra`,
      );
    } else if (spec.startsWith("#gen/")) {
      violations.push(
        `forbidden import '${spec}' — generated db/api code is an adapter concern, not domain`,
      );
    } else if (spec.startsWith("#features/") || spec.includes("/features/")) {
      violations.push(`forbidden import '${spec}' — slices never depend on other slices`);
    }
  }
  return violations;
}

// evalsDir / repoRoot resolved from this file's import.meta.url, so fixture
// reads work from any working directory (and inside a staged/renamed copy,
// where verify-template.sh runs `node --test` on the copied tree in place).

// readFixture / readRootFile helpers.
async function readFixture(rel) {
  return readFile(path.join(HERE, rel), "utf8");
}

// renamedReported reports whether the tree this test file lives in has
// already been through scripts/rename.sh (the evals/package.json "name"
// carries the substituted scope/app). Pre-rename the tree legitimately
// carries placeholder vocabulary everywhere outside the agency layer, so the
// sweep below skips itself in that state instead of failing.
async function renamedReported() {
  const pkg = await readFile(path.join(HERE, "package.json"), "utf8");
  return !hasPlaceholderLeak(pkg);
}

function assertPass(r, ctx) {
  assert.deepStrictEqual(r.ok, true, `${ctx}: expected ok, got errors:\n${joined(r)}`);
}
function assertFailMatching(r, pat, ctx) {
  assert.deepStrictEqual(r.ok, false, `${ctx}: expected failure, but validator passed`);
  assert.ok(
    joined(r).includes(pat),
    `${ctx}: expected errors to contain '${pat}', got:\n${joined(r)}`,
  );
}

// ---- tests -----------------------------------------------------------------

test("valid skill fixture passes", async () => {
  const r = validate(await readFixture("fixtures/valid/skill.md"), KIND_SKILL);
  assertPass(r, "valid skill");
});

test("valid agent fixture passes", async () => {
  const r = validate(await readFixture("fixtures/valid/agent.md"), KIND_AGENT);
  assertPass(r, "valid agent");
});

test("valid command fixture passes", async () => {
  const r = validate(await readFixture("fixtures/valid/command.md"), KIND_COMMAND);
  assertPass(r, "valid command");
});

test("invalid missing-frontmatter-key fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/missing-frontmatter-key.md"), KIND_SKILL);
  assertFailMatching(r, "missing frontmatter key 'division'", "missing-frontmatter-key");
});

test("invalid missing-body-section fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/missing-body-section.md"), KIND_SKILL);
  assertFailMatching(r, "missing body section 'Output Format'", "missing-body-section");
});

test("invalid literal-placeholder-leak fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/literal-placeholder-leak.md"), KIND_SKILL);
  assertFailMatching(r, "literal placeholder leak", "literal-placeholder-leak");
});

test("invalid bad-slice-import flags net/fastify", async () => {
  const text = await readFixture("fixtures/invalid/bad-slice-import.ts.txt");
  const v = domainImportViolations(text);
  assert.ok(v.length > 0, "bad-slice-import: expected violations, got none");
  assert.ok(
    v.join("\n").includes("forbidden import 'fastify'"),
    `bad-slice-import: wanted a fastify violation, got:\n${v.join("\n")}`,
  );
});

test("bad division fails", () => {
  const text =
    '---\nname: "s"\ndescription: "d"\ndivision: frontend\n---\n\n## When to Activate\n\nx';
  const r = validate(text, KIND_SKILL);
  assertFailMatching(r, "bad division 'frontend'", "bad-division");
});

test("empty frontmatter fails", () => {
  const r = validate("---\n---\n\n# body\n", KIND_SKILL);
  assertFailMatching(r, "empty frontmatter", "empty-frontmatter");
});

test("no frontmatter fails", () => {
  const r = validate("# No frontmatter here\n\n## When to Activate\n", KIND_SKILL);
  assertFailMatching(r, "missing or malformed frontmatter", "no-frontmatter");
});

test("CRLF line endings fail", async () => {
  const text = (await readFixture("fixtures/valid/skill.md")).replaceAll("\n", "\r\n");
  const r = validate(text, KIND_SKILL);
  assertFailMatching(r, "CRLF line endings", "crlf");
});

test("pure domain imports pass", async () => {
  // A pure domain.ts: node builtins + kernel only. "demoapp" is inert
  // post-rename display vocabulary (frozen fixture style), never one of the
  // substitution tokens.
  const text = `import type { AppError } from "../../shared/apperr";

export const WidgetKinds = ["sprocket", "cog", "flange"] as const;
export type Widget = { id: string; kind: (typeof WidgetKinds)[number] };
`;
  if (domainImportViolations(text).length !== 0) {
    assert.fail("pure domain.ts flagged: " + domainImportViolations(text).join("\n"));
  }
});

test("cross-slice import flagged", async () => {
  const text = 'import { Signup } from "../../features/authsignup/domain";\n';
  const v = domainImportViolations(text);
  assert.ok(v.length > 0, "cross-slice import not flagged");
});

// Sweep the tree (post-rename) for residual literal placeholders OUTSIDE the
// agency layer: scripts/ keeps its substitution-token pattern strings by
// design, evals/ is skipped (this file assembles its tokens at runtime) and
// fixtures/ is frozen leak-detector data, .git + binaries + lockfiles are not
// text. Mirrors scripts/leak-guard.sh's exclusion semantics; anything else
// carrying one of the tokens is a rename bug. verify-template.sh's own
// survivor grep is the second, slightly weaker copy of this check.
test("no placeholder leaks outside agency layer (post-rename sweep)", async () => {
  if (!(await renamedReported())) {
    return; // pre-rename: placeholders outside the agency layer are expected
  }
  const SKIP_DIRS = new Set([
    ".git",
    "scripts",
    "evals",
    "fixtures",
    "node_modules",
    "dist",
    "coverage",
  ]);
  const BINARY_EXT = new Set([
    ".png",
    ".jpg",
    ".jpeg",
    ".ico",
    ".gif",
    ".woff",
    ".woff2",
    ".ttf",
    ".otf",
    ".eot",
  ]);
  const files = [];
  async function walk(dir) {
    for (const e of await readdir(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        if (p === ROOT || SKIP_DIRS.has(e.name)) continue;
        await walk(p);
      } else {
        const ext = path.extname(e.name).toLowerCase();
        if (
          SKIP_DIRS.has(e.name) ||
          e.name === "pnpm-lock.yaml" ||
          e.name === "package-lock.json" ||
          e.name === "yarn.lock" ||
          BINARY_EXT.has(ext)
        )
          continue;
        files.push(p);
        if (files.length > 20000) throw new Error("walk exploded");
      }
    }
  }
  await walk(ROOT);
  const leaks = [];
  for (const f of files) {
    const buf = await readFile(f);
    if (buf.includes(0)) continue; // binary
    if (PLACEHOLDER_TOKENS.some((tok) => buf.includes(tok))) leaks.push(path.relative(ROOT, f));
  }
  assert.deepStrictEqual(
    leaks,
    [],
    "literal placeholder leak outside the agency layer (rename.sh must substitute all of " +
      PLACEHOLDER_TOKENS.join("/") +
      ") in: " +
      leaks.join("\n"),
  );
});
