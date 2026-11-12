// contracts.test.mjs — pure-STD contract validator + tests for the .claude
// layer of the Chrome extension template (native `node --test`, zero
// external dependencies).
//
// Hand-rolled frontmatter parser + body-section checker for agents, skills
// and commands. The required fields, required sections, allowed divisions and
// failure modes are HARDCODED as static lists here — this validator does NOT
// read .claude/schemas/*.schema.md at runtime. The schema docs are the source
// of truth and the bash linters (scripts/lint-agents.sh, lint-skills.sh,
// lint-commands.sh) parse them at runtime, so the linter<->schema drift
// guarantee holds via the linters. This validator is a hand-maintained mirror
// of the schema docs: when a list changes in a schema doc you MUST update the
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
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

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
function validate(text, kind, { checkPlaceholders = true } = {}) {
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

test("every agent file satisfies the agents schema", async () => {
  const files = await listFiles(path.join(ROOT, ".claude", "agents"), ".md");
  assert.ok(
    files.length > 0,
    "no agent files found under .claude/agents — the agent tree is not written yet",
  );
  for (const file of files) {
    const r = validate(await readFile(file, "utf8"), KIND_AGENT, { checkPlaceholders: RENAMED });
    assertPass(r, path.relative(ROOT, file));
  }
});

test("every skill file satisfies the skills schema", async () => {
  const files = await listFiles(path.join(ROOT, ".claude", "skills"), "SKILL.md");
  assert.ok(
    files.length > 0,
    "no SKILL.md files found under .claude/skills — the skill tree is not written yet",
  );
  for (const file of files) {
    const r = validate(await readFile(file, "utf8"), KIND_SKILL, { checkPlaceholders: RENAMED });
    assertPass(r, path.relative(ROOT, file));
  }
});

test("every command file satisfies the commands schema", async () => {
  const files = await listFiles(path.join(ROOT, ".claude", "commands"), ".md");
  assert.ok(
    files.length > 0,
    "no command files found under .claude/commands — the command tree is not written yet",
  );
  for (const file of files) {
    const text = await readFile(file, "utf8");
    const r = validate(text, KIND_COMMAND, { checkPlaceholders: RENAMED });
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

test("every command has a matching skill directory", async () => {
  const commands = await listFiles(path.join(ROOT, ".claude", "commands"), ".md");
  assert.ok(commands.length > 0, "no command files found under .claude/commands");
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

// ---- manifest posture ------------------------------------------------------

test("manifest module keeps the least-privilege permission posture", async () => {
  const mod = await import(pathToFileURL(path.join(ROOT, "src", "manifest.ts")).href);
  const manifest = mod.default;
  assert.equal(manifest.manifest_version, 3, "manifest_version must be 3 (MV3)");
  assert.deepEqual(
    manifest.host_permissions ?? [],
    [],
    "host_permissions must stay empty — host access is granted per use case",
  );
  const declared = [
    ...(manifest.permissions ?? []),
    ...(manifest.optional_permissions ?? []),
    ...(manifest.host_permissions ?? []),
    ...(manifest.optional_host_permissions ?? []),
  ];
  assert.ok(!declared.includes("debugger"), "the debugger permission must never be declared");
  assert.ok(
    !declared.some((p) => p.includes("<all_urls>")),
    "a blanket <all_urls> grant must never be declared",
  );
  const matches = (manifest.content_scripts ?? []).flatMap((cs) => cs.matches ?? []);
  assert.ok(
    !matches.some((m) => m.includes("<all_urls>")),
    "a blanket <all_urls> content-script match must never be declared",
  );
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
  },
);
