// contracts.test.mjs — pure-STD contract validator + tests for the iOS kit
// (native `node --test`, zero external dependencies).
//
// Hand-rolled frontmatter parser + body-section checker for agents, skills
// and commands. The required frontmatter fields and the required body
// sections are HARDCODED as static lists here. The allowed divisions are
// NOT hardcoded: schemaDivisions() reads the `## Allowed Divisions`
// section of the matching .claude/schemas/*.schema.md at runtime, so the
// division enum cannot drift (ALLOWED_DIVISIONS is only the fallback). The
// schema docs are the source of truth and the bash linters
// (scripts/lint-agents.sh, lint-skills.sh, lint-commands.sh) parse them at
// runtime, so the linter<->schema drift guarantee holds via the linters.
// When a hardcoded list changes in a schema doc you MUST update the
// matching static list here too.
//
// The fixtures are FROZEN test data: scripts/rename.sh excludes
// evals/fixtures/ from its substitution pass, so the invalid leak fixture
// keeps its literal tokens post-rename and keeps exercising the leak
// detector.
//
// This file carries NO literal template substitution token anywhere: the
// leak-detector tokens are assembled at runtime in PLACEHOLDER_TOKENS and the
// leak error message interpolates them, so the substitution pass cannot
// defuse the detector.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.dirname(HERE); // template root

// The three template substitution tokens, assembled from pieces so this file
// never spells one out literally. scripts/rename.sh rewrites every file
// outside scripts/ and evals/fixtures/, so a literal token here would be
// substituted away and defuse the leak detector.
const PLACEHOLDER_TOKENS = ["<" + "app>", "<" + "APP>", "<" + "scope>"];

const KIND_AGENT = "agent";
const KIND_SKILL = "skill";
const KIND_COMMAND = "command";

// Required frontmatter fields — agents.schema.md and skills/commands schemas.
function requiredFrontmatter(kind) {
  if (kind === KIND_AGENT) {
    return ["name", "description", "color", "division"];
  }
  return ["name", "description", "division"];
}

// Required body sections — agents.schema.md, skills.schema.md,
// commands.schema.md.
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

// Allowed divisions — shared across agents, skills and commands in this kit.
const ALLOWED_DIVISIONS = ["client", "security", "testing", "core"];

// The six Swift packages every kit copy ships.
const PACKAGES = [
  "Packages/Core/Kernel",
  "Packages/Core/UICore",
  "Packages/Feature/Notes/Contracts",
  "Packages/Feature/Notes/Domain",
  "Packages/Feature/Notes/Data",
  "Packages/Feature/Notes/UI",
];

// The packages that must stay free of UI frameworks.
const PURE_PACKAGES = [
  "Packages/Core/Kernel",
  "Packages/Feature/Notes/Contracts",
  "Packages/Feature/Notes/Domain",
];

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
// one layer of surrounding quotes). closed=false when missing or malformed.
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
          const a = val[0];
          const b = val[val.length - 1];
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
// checkPlaceholders=false skips the literal-leak rule: the canonical template
// legitimately carries placeholder vocabulary in its agent/skill names before
// the rename pass (the same escape hatch the bash linters expose through
// ALLOW_PLACEHOLDERS=1).
function validate(text, kind, { checkPlaceholders = true, divisions = ALLOWED_DIVISIONS } = {}) {
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
    if (!divisions.includes(div)) fail(r, `bad division '${div}'`);
  }

  for (const section of requiredSections(kind)) {
    if (!hasSection(body, section)) fail(r, `missing body section '${section}'`);
  }

  if (checkPlaceholders && hasPlaceholderLeak(text)) {
    fail(
      r,
      `literal placeholder leak (${PLACEHOLDER_TOKENS.join("/")}) — rename.sh must substitute all placeholders`,
    );
  }

  return r;
}

// readFixture reads a file from the frozen fixture tree.
async function readFixture(rel) {
  return readFile(path.join(HERE, rel), "utf8");
}

// renamedReported reports whether the tree this test file lives in has been
// through scripts/rename.sh: evals/package.json carries the substituted
// scope/app in its name. Pre-rename the tree legitimately carries placeholder
// vocabulary everywhere outside the agency layer, so the sweep below skips
// itself in that state instead of failing.
const evalsPkg = JSON.parse(await readFile(path.join(HERE, "package.json"), "utf8"));
const RENAMED = !hasPlaceholderLeak(String(evalsPkg.name));

test("evals package name carries the rename-state marker", () => {
  assert.match(
    String(evalsPkg.name),
    /^@.+\/.+-evals$/,
    `evals/package.json name must be @${PLACEHOLDER_TOKENS[2]}/${PLACEHOLDER_TOKENS[0]}-evals` +
      " (pre-rename) or the substituted form",
  );
  if (RENAMED) {
    assert.ok(
      !hasPlaceholderLeak(String(evalsPkg.name)),
      "post-rename, evals/package.json must carry the substituted name, not the placeholders",
    );
  }
});

// listFiles returns every file under dir matching the extension filter,
// recursively. Returns [] when dir does not exist.
async function listFiles(dir, ext) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  const out = [];
  for (const entry of entries) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...(await listFiles(p, ext)));
    } else if (!ext || entry.name.endsWith(ext)) {
      out.push(p);
    }
  }
  return out.sort();
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

// schemaDivisions reads the allowed divisions from the schema doc of a kind
// when it is there, so the tree tests follow the schema instead of a stale
// list. The frozen fixtures always use the static list: the invalid fixture
// has to keep failing whatever the schema allows.
const SCHEMA_DOCS = {
  [KIND_AGENT]: "agents.schema.md",
  [KIND_SKILL]: "skills.schema.md",
  [KIND_COMMAND]: "commands.schema.md",
};

async function schemaDivisions(kind) {
  const schema = path.join(ROOT, ".claude", "schemas", SCHEMA_DOCS[kind]);
  let text;
  try {
    text = await readFile(schema, "utf8");
  } catch {
    return ALLOWED_DIVISIONS;
  }
  const section = text.split(/^## /m).find((part) => part.startsWith("Allowed Divisions"));
  if (!section) return ALLOWED_DIVISIONS;
  const items = [...section.matchAll(/^-\s+(.+)$/gm)]
    .map((m) => m[1].trim().replace(/^[`"']|[`"']$/g, ""))
    .filter(Boolean);
  return items.length > 0 ? items : ALLOWED_DIVISIONS;
}

// ---- fixture tests ---------------------------------------------------------

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

test("invalid bad-division fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/bad-division.md"), KIND_SKILL);
  assertFailMatching(r, "bad division 'backend'", "bad-division");
});

test("invalid missing-body-section fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/missing-body-section.md"), KIND_SKILL);
  assertFailMatching(r, "missing body section 'Output Format'", "missing-body-section");
});

test("invalid literal-placeholder-leak fails", async () => {
  const r = validate(await readFixture("fixtures/invalid/literal-placeholder-leak.md"), KIND_SKILL);
  assertFailMatching(r, "literal placeholder leak", "literal-placeholder-leak");
});

test("invalid command-name-mismatch fails the tree rule", async () => {
  const text = await readFixture("fixtures/invalid/command-name-mismatch.md");
  const r = validate(text, KIND_COMMAND);
  assertPass(r, "command fixture body");
  const { fm } = parseFrontmatter(text);
  assert.notEqual(fmGet(fm, "name"), "command-name-mismatch", "the fixture's name must differ from its slug");
});

test("bad division fails", () => {
  const text =
    '---\nname: "s"\ndescription: "d"\ndivision: backend\n---\n\n## When to Activate\n\nx';
  const r = validate(text, KIND_SKILL);
  assertFailMatching(r, "bad division 'backend'", "bad-division");
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

// ---- tree tests ------------------------------------------------------------

test("every agent file satisfies the agents schema", async (t) => {
  const files = await listFiles(path.join(ROOT, ".claude", "agents"), ".md");
  if (files.length === 0) {
    t.skip("no .claude/agents tree in this kit");
    return;
  }
  const divisions = await schemaDivisions(KIND_AGENT);
  for (const file of files) {
    const r = validate(await readFile(file, "utf8"), KIND_AGENT, {
      checkPlaceholders: RENAMED,
      divisions,
    });
    assertPass(r, path.relative(ROOT, file));
  }
});

test("every skill file satisfies the skills schema", async (t) => {
  const files = await listFiles(path.join(ROOT, ".claude", "skills"), "SKILL.md");
  if (files.length === 0) {
    t.skip("no .claude/skills tree in this kit");
    return;
  }
  const divisions = await schemaDivisions(KIND_SKILL);
  for (const file of files) {
    const r = validate(await readFile(file, "utf8"), KIND_SKILL, {
      checkPlaceholders: RENAMED,
      divisions,
    });
    assertPass(r, path.relative(ROOT, file));
  }
});

test("every command file satisfies the commands schema", async (t) => {
  const files = await listFiles(path.join(ROOT, ".claude", "commands"), ".md");
  if (files.length === 0) {
    t.skip("no .claude/commands tree in this kit");
    return;
  }
  const divisions = await schemaDivisions(KIND_COMMAND);
  for (const file of files) {
    const text = await readFile(file, "utf8");
    const r = validate(text, KIND_COMMAND, { checkPlaceholders: RENAMED, divisions });
    assertPass(r, path.relative(ROOT, file));
    // commands.schema.md: the command name is the slug it delegates to.
    const slug = path.basename(file, ".md");
    const { fm } = parseFrontmatter(text);
    assert.equal(
      fmGet(fm, "name"),
      slug,
      `${path.relative(ROOT, file)}: name must equal the command slug '${slug}'`,
    );
  }
});

test("every command has a matching skill directory", async (t) => {
  const commands = await listFiles(path.join(ROOT, ".claude", "commands"), ".md");
  if (commands.length === 0) {
    t.skip("no .claude/commands tree in this kit");
    return;
  }
  for (const file of commands) {
    const slug = path.basename(file, ".md");
    const skillDir = path.join(ROOT, ".claude", "skills", slug);
    const entries = await readdir(skillDir).catch(() => null);
    assert.ok(
      entries !== null && entries.includes("SKILL.md"),
      `command '${slug}' has no matching skill at .claude/skills/${slug}/SKILL.md`,
    );
  }
});

// ---- iOS posture -----------------------------------------------------------

test("the six Swift packages exist and compile in Swift 6 mode", async () => {
  for (const pkg of PACKAGES) {
    const manifest = await readFile(path.join(ROOT, pkg, "Package.swift"), "utf8");
    assert.ok(
      manifest.includes("swiftLanguageModes: [.v6]"),
      `${pkg}/Package.swift must declare swiftLanguageModes: [.v6]`,
    );
    const sources = await listFiles(path.join(ROOT, pkg, "Sources"), ".swift");
    assert.ok(sources.length > 0, `${pkg}/Sources has no Swift source`);
  }
});

test("the pure packages import no UI framework", async () => {
  const forbidden = /^import (SwiftUI|UIKit|AppKit|WatchKit)$/;
  for (const pkg of PURE_PACKAGES) {
    for (const file of await listFiles(path.join(ROOT, pkg, "Sources"), ".swift")) {
      const text = await readFile(file, "utf8");
      for (const line of text.split("\n")) {
        assert.ok(
          !forbidden.test(line.trim()),
          `${path.relative(ROOT, file)} imports a UI framework — ${pkg} stays pure Swift`,
        );
      }
    }
  }
});

test("the UI never reaches data and data never reaches UI", async () => {
  const uiManifest = await readFile(
    path.join(ROOT, "Packages/Feature/Notes/UI/Package.swift"),
    "utf8",
  );
  assert.ok(!uiManifest.includes("NotesData"), "NotesUI/Package.swift must not depend on NotesData");

  const dataManifest = await readFile(
    path.join(ROOT, "Packages/Feature/Notes/Data/Package.swift"),
    "utf8",
  );
  assert.ok(!dataManifest.includes("NotesUI"), "NotesData/Package.swift must not depend on NotesUI");

  for (const file of await listFiles(path.join(ROOT, "Packages/Feature/Notes/UI/Sources"), ".swift")) {
    const text = await readFile(file, "utf8");
    assert.ok(
      !/^import NotesData$/m.test(text),
      `${path.relative(ROOT, file)} imports NotesData — the UI receives its state from the model`,
    );
  }
});

test("project.yml is the single application target and names every package", async () => {
  const text = await readFile(path.join(ROOT, "project.yml"), "utf8");
  assert.equal(
    (text.match(/^\s*type: application\s*$/gm) ?? []).length,
    1,
    "project.yml must declare exactly one application target",
  );
  for (const pkg of [
    "Packages/Core/Kernel",
    "Packages/Core/UICore",
    "Packages/Feature/Notes/Contracts",
    "Packages/Feature/Notes/Domain",
    "Packages/Feature/Notes/Data",
    "Packages/Feature/Notes/UI",
  ]) {
    assert.ok(text.includes(pkg), `project.yml must reference ${pkg}`);
  }
  assert.match(text, /SWIFT_VERSION: "6\.0"/, "project.yml must pin SWIFT_VERSION 6.0");
  assert.match(
    text,
    /SWIFT_STRICT_CONCURRENCY: complete/,
    "project.yml must enable complete strict concurrency",
  );
});

// credentialProblems returns every credential-looking key and every
// credential-shaped value of a YAML manifest. Both sides matter: a field
// literally named "key" is a credential, and so is a token-looking value
// parked under an innocuous key.
function credentialProblems(yamlText) {
  const suspiciousKey = /api[_-]?key|secret|password|token|private[_-]?key|credential|\bkey\b/i;
  const suspiciousValue =
    /AKIA[0-9A-Z]{16}|PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{20,}|sk_(live|test)_[A-Za-z0-9]{16,}|xox[baprs]-[A-Za-z0-9-]{10,}|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}|\.p12|\.mobileprovision/;

  const problems = [];
  const lines = yamlText.split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    const m = /^\s*(?:-\s*)?([A-Za-z0-9_.-]+):\s*(.*?)\s*$/.exec(lines[i]);
    if (!m) continue;
    const [, key, value] = m;
    if (suspiciousKey.test(key)) {
      problems.push(`line ${i + 1}: the key '${key}' looks like a credential`);
    }
    if (suspiciousValue.test(value)) {
      problems.push(`line ${i + 1}: the value of '${key}' has the shape of a secret`);
    }
  }
  return problems;
}

test("project.yml declares no key, secret, signing profile or blanket transport grant", async () => {
  const text = await readFile(path.join(ROOT, "project.yml"), "utf8");

  assert.deepStrictEqual(
    credentialProblems(text),
    [],
    "project.yml must not declare a credential",
  );
  assert.ok(
    !/NSAllowsArbitraryLoads:\s*true/.test(text),
    "project.yml must not grant arbitrary (non-TLS) network loads",
  );
});

test("the credential detector catches a bare key and a secret-shaped value", () => {
  assert.deepStrictEqual(credentialProblems("name: notes\nversion: 1.0.0"), []);
  assert.match(credentialProblems("api_key: whatever")[0] ?? "", /looks like a credential/);
  assert.match(
    credentialProblems("note: AKIAIOSFODNN7EXAMPLE")[0] ?? "",
    /has the shape of a secret/,
  );
});

test(".gitignore ignores the generated Xcode project and DerivedData", async () => {
  const text = await readFile(path.join(ROOT, ".gitignore"), "utf8");
  assert.match(text, /^\*\.xcodeproj\/?$/m, ".gitignore must ignore *.xcodeproj");
  assert.match(text, /^DerivedData\/$/m, ".gitignore must ignore DerivedData/");
});

test("no visual-builder file is in the tree", async () => {
  const forbidden = [".storyboard", ".xib", ".nib", ".xcdatamodeld", ".rcproject"];
  const hits = [];
  async function walk(dir) {
    for (const e of await readdir(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        if (p === ROOT || [".git", ".build", "DerivedData", "node_modules"].includes(e.name)) continue;
        if (e.name.endsWith(".xcodeproj")) continue;
        await walk(p);
      } else if (forbidden.some((ext) => e.name.endsWith(ext))) {
        hits.push(path.relative(ROOT, p));
      }
    }
  }
  await walk(ROOT);
  assert.deepStrictEqual(hits, [], "views are written as Swift code, never in Interface Builder");
});

test("the evals package has no postinstall script", async () => {
  const scripts = evalsPkg.scripts ?? {};
  assert.ok(!("postinstall" in scripts), "the evals package must not run a postinstall script");
  assert.ok(!("prepare" in scripts), "the evals package must not run a prepare script");
});

// ---- the bash tripwire -----------------------------------------------------

test("the bash tripwire blocks the destructive home deletes", (t) => {
  const guard = path.join(ROOT, "scripts", "hooks", "guard-bash.sh");
  const run = (command) =>
    spawnSync("bash", [guard], {
      input: JSON.stringify({ tool_name: "Bash", tool_input: { command } }),
      encoding: "utf8",
    });

  if (run("ls").error) {
    t.skip("bash is not available on this machine");
    return;
  }

  // The home roots, with and without the trailing slash, and the two roots
  // the tripwire used to only warn about.
  for (const command of ["rm -rf ~/", "rm -rf $HOME/", "rm -rf /root", "rm -rf /Users/alice"]) {
    assert.equal(run(command).status, 2, `the tripwire must block '${command}'`);
  }

  for (const command of ["rm -rf /tmp/scratch", "rm -rf ./build"]) {
    assert.equal(run(command).status, 0, `the tripwire must not block '${command}'`);
  }
});

// ---- rename-state sweep ----------------------------------------------------

test(
  "no placeholder leaks outside the frozen fixtures, scripts and evals",
  {
    skip: RENAMED ? false : "pre-rename: the tree legitimately carries placeholder vocabulary",
  },
  async () => {
    const SKIP_DIRS = new Set([
      ".git",
      "scripts",
      "evals",
      "fixtures",
      "node_modules",
      ".build",
      "DerivedData",
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
          if (p === ROOT || SKIP_DIRS.has(e.name) || e.name.endsWith(".xcodeproj")) continue;
          await walk(p);
        } else {
          const ext = path.extname(e.name).toLowerCase();
          if (SKIP_DIRS.has(e.name) || BINARY_EXT.has(ext)) continue;
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
  },
);
