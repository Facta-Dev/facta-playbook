# PLAYBOOK_CLAUDE.md — Claude Code implementation kit: what every project must have

Specification of the **Claude Code implementation kit** for any software project:
the files, directories, agents, skills, rules, hooks, permissions, switches,
memory, collaboration protocol and improvement loop that make Claude Code
perform well, safely and consistently when **several `claude` CLI sessions work
in parallel** and the team practices **spec-driven development (SDD) +
test-driven development (TDD)**.

This document is the *guide*, not the kit. It tells you what the kit must
contain, why, and how to verify it. It is language- and stack-agnostic.
Companion playbooks (backend, frontend, URLs, tests, security, audit) plug into
it as the project's "constitution"; if your project has none, the kit still
works with your own conventions.

The rules that carry the whole file:

1. **Context is a budget.** Everything always loaded (CLAUDE.md, rules without
   `paths`, skill/agent descriptions, auto-memory index) is paid on every turn.
   Keep it small; load the rest on demand.
2. **Instructions guide, controls enforce.** CLAUDE.md, rules and skills are
   context Claude *tries* to follow. What must never be violated goes in
   permissions, hooks or managed settings.
3. **Least privilege per role.** Each agent gets only the tools, paths and
   skills its job needs.
4. **Spec before code, test before implementation.** Artifacts and test runs are
   the evidence, not chat history.
5. **One work unit, one worktree, one owner.** Parallel sessions never edit the
   same files at once.
6. **Coordination state lives outside the working tree** and is shared by all
   worktrees.
7. **Everything learned is captured, triaged and promoted deliberately** up a
   ladder (memory → rule → skill → hook), never silently.
8. **The control plane is human-owned.** Agents may propose changes to hooks,
   permissions and role policies; they cannot apply them.
9. **Autodetect, then verify.** Rely on native discovery (descriptions, `paths`,
   live reload) and add a registry that validates what was discovered.
10. **Measure the kit.** Agents, skills and rules have evals, budgets and owners
    like any other code.

> Verification note: Claude Code evolves quickly. Version numbers cited here
> come from the official documentation at the time of writing (baseline:
> Claude Code v2.1.309 or later). Before adopting a feature, confirm it with
> `claude --version`, `/doctor`, and the docs
> (<https://code.claude.com/docs>). Where this guide says "documented", it is
> stated in those docs; where it says "design choice", it is this playbook's
> recommendation.

---

## 1. Anatomy of the kit: seven planes

| Plane | Purpose | Main artifacts |
|---|---|---|
| **Instruction** | Tell Claude how this project works | `CLAUDE.md`, nested `CLAUDE.md`, `.claude/rules/*.md`, `CLAUDE.local.md`, optional `AGENTS.md` |
| **Memory** | Persist knowledge across sessions | auto memory, agent memory, specs, ADRs, lessons inbox |
| **Capability** | Reusable roles and procedures | `.claude/agents/*.md`, `.claude/skills/*/SKILL.md`, optional MCP / plugins / output styles |
| **Control** | Enforce limits regardless of what Claude decides | `settings.json` permissions, hooks, role/ownership policy, managed settings |
| **Coordination** | Let many sessions work without collisions | worktrees, launcher, claims, cross-session messaging, ownership map |
| **Process** | Make work verifiable | SDD artifacts (`specs/`), TDD runner and state, CI gates |
| **Learning** | Improve the kit over time | lessons, retros, evals, registry, kit-curator, audit cadence |

A kit is **complete** only when all seven planes exist (section 19 is the
acceptance checklist).

### 1.1 Component inventory (what must exist)

| # | Component | Level | Section |
|---|---|---|---|
| 1 | Root `CLAUDE.md` (≤ 200 lines) | **Required** | 4 |
| 2 | `.claude/settings.json` (permissions, hooks, worktree, retention) | **Required** | 7 |
| 3 | `.claude/rules/*.md` with `paths` frontmatter | **Required** | 4.3 |
| 4 | Nested `CLAUDE.md` per vertical slice / package | Recommended | 4.4 |
| 5 | Core agents: spec-architect, test-writer, implementer, code-reviewer, security-reviewer, integrator, kit-curator, mapper | **Required** (names may vary) | 6 |
| 6 | Core skills: spec-new/plan/tasks, tdd-cycle, spec-verify, handoff, learn, retro, kit-audit, scaffolds | **Required** | 6.4 |
| 7 | Role policy (`roles.json`) + ownership map (`ownership.json`) | **Required** for multi-session | 7.4 |
| 8 | Hooks: SessionStart, PreToolUse guard, Stop gate, SessionEnd | **Required** | 7.3 |
| 9 | Launcher script (worktree + role + env + name) | **Required** for multi-session | 8.3 |
| 10 | Claim/coordination library (state outside the working tree) | **Required** for multi-session | 8.4 |
| 11 | TDD runner + state | **Required** | 10 |
| 12 | `specs/` tree + templates | **Required** | 9 |
| 13 | Registry/validator script + generated `REGISTRY.md` | **Required** | 11 |
| 14 | Doctor script (kit health) + CI job | **Required** | 15 |
| 15 | Switch profiles (`strict`, `standard`, `explore`) | **Required** | 12 |
| 16 | Lessons pipeline (capture → inbox → retro) | **Required** | 13 |
| 17 | Skill/agent evals dataset | Recommended | 11.5 |
| 18 | `.worktreeinclude`, `.gitignore`/`.gitattributes` entries | **Required** | 3 |
| 19 | `CODEOWNERS` entries for control-plane files | **Required** | 14 |
| 20 | MCP servers (`.mcp.json`), plugins | Optional | 6.6 |
| 21 | Managed settings / managed CLAUDE.md (organizations) | Optional (org level) | 14.5 |
| 22 | Output styles, status line | Optional | 6.7 |
| 23 | `kit/CLAUDE.md` — user-scope instructions (installed at `~/.claude/CLAUDE.md`) | Required (user scope) | 20 |
| 24 | `kit/settings.fragment.json` — model-routing env, default model and the deny list, merged into `~/.claude/settings.json` | Required (user scope) | 20 |

---

## 2. Principles of the design

### 2.1 Where each kind of knowledge goes (decision table)

| If the content is… | Put it in… | Why |
|---|---|---|
| A fact Claude needs in **every** session (commands, layout, hard rules) | `CLAUDE.md` | Always loaded; keep it short |
| Needed only when touching certain files | `.claude/rules/<topic>.md` with `paths:` or nested `CLAUDE.md` | Loaded on demand |
| A multi-step **procedure** | a **skill** | Body loads only when used |
| A specialized **worker** with its own tools/context | an **agent** (subagent) | Isolated context, restricted tools |
| Something that **must always** happen / never happen | a **hook** or **permission** | Enforced by the harness |
| A personal preference | `CLAUDE.local.md`, `settings.local.json`, `~/.claude/` | Not shared |
| Knowledge Claude should accumulate | auto memory / agent memory | Written by Claude |
| A decision with rationale | ADR in `docs/adr/` | Durable, reviewable |
| The *what* and *why* of a change | `specs/` | SDD source of truth |

### 2.2 Instructions vs. controls

CLAUDE.md content arrives as context after the system prompt; there is no
guarantee of strict compliance. If an instruction must hold at a specific
point (before every commit, after every edit, never touch a path), implement it
as a **hook** (runs at fixed lifecycle events) or **permission rule**. Use
managed settings to make controls non-overridable.

### 2.3 Cost model

Each subagent has its own context window and its own requests (they count
toward usage limits). Delegate to a subagent when output would flood the main
conversation; do small, tightly coupled edits in the main thread. Route
low-reasoning work (search, mapping) to lower-cost models via the agent's
`model` field.

A fresh machine pays this budget before any work starts: the installation
runbook ([PLAYBOOK_INSTALL.md](PLAYBOOK_INSTALL.md), §0) keeps the
always-loaded surface under target and measures it with `kit_check.py`.
Installation is shell-only, so it costs zero model tokens; the model is
consulted only for judgment.

### 2.4 Language policy

All generated artifacts are in **English**: source code, identifiers, comments,
docstrings, test names, commit messages, specs and ADRs, `CLAUDE.md` and rules,
agent and skill definitions, and documentation. State it once in the root
`CLAUDE.md` so every session and subagent inherits it; non-English text appears
only in user-facing localization resources. Mixed-language code, comments and
prompts cost context, break grep/search, and produce inconsistent review
findings.

---

## 3. Directory structure

```text
<repo>/
├── CLAUDE.md                          # team instructions, ≤ 200 lines            [commit]
├── CLAUDE.local.md                    # personal, per-project                      [gitignore]
├── AGENTS.md                          # optional, shared with other tools          [commit]
├── .mcp.json                          # team-shared MCP servers (optional)         [commit]
├── .worktreeinclude                   # gitignored files copied to new worktrees   [commit]
├── .gitattributes                     # merge=union for agent-memory indexes       [commit]
├── CODEOWNERS (or .github/CODEOWNERS) # kit control plane requires human review    [commit]
│
├── .claude/
│   ├── settings.json                  # permissions, hooks, worktree, retention    [commit]
│   ├── settings.local.json            # personal overrides                         [gitignore]
│   ├── roles.json                     # role → allowed/denied paths (kit-defined)  [commit, control plane]
│   ├── ownership.json                 # slice patterns, shared paths, tdd config   [commit, control plane]
│   ├── REGISTRY.md                    # generated index of agents/skills/rules     [commit, generated]
│   ├── agents/                        # subagent definitions (*.md)                [commit]
│   ├── skills/<name>/SKILL.md         # skills (+ scripts/, reference.md)          [commit]
│   ├── rules/*.md                     # topic/path-scoped rules                    [commit]
│   ├── hooks/                         # hook scripts (+ coordination library)      [commit, control plane]
│   ├── agent-memory/<agent>/          # project-scope agent memory                 [commit]
│   ├── agent-memory-local/<agent>/    # local-scope agent memory                   [gitignore]
│   ├── proposals/                     # agent-written change proposals for humans  [commit]
│   ├── evals/                         # trigger/behavior datasets for skills/agents[commit]
│   └── worktrees/                     # Claude-created worktrees (default path)    [gitignore]
│
├── specs/                             # spec-driven development
│   ├── _templates/{spec,plan,tasks}.md
│   └── <slice>/<NNNN-feature>/{spec.md, plan.md, tasks.md, contracts/}
├── docs/adr/                          # architecture decision records
├── src/ … tests/ …                    # your code; vertical slices recommended
├── src/<slice>/CLAUDE.md              # nested, loaded on demand when working there
└── scripts/
    ├── claude/{launcher, claim, registry, doctor, profiles/*.env}               [control plane]
    └── tdd/{run-tests, slice-test (project override)}                           [control plane]
```

Outside the repository (written by Claude Code, never committed):

| Path | Content |
|---|---|
| `~/.claude/CLAUDE.md`, `~/.claude/rules/`, `~/.claude/agents/`, `~/.claude/skills/`, `~/.claude/settings.json` | Personal, all projects |
| `~/.claude/projects/<project>/memory/` | **Auto memory** (`MEMORY.md` index + topic files), shared by all worktrees of a repo, machine-local |
| `~/.claude/projects/<project>/<session>.jsonl` | Transcripts (plaintext; retention `cleanupPeriodDays`, default 30) |
| `<git-common-dir>/claude-coord/` | **Kit coordination state**: claims, sessions, TDD state, lessons inbox, logs (shared across worktrees, not versioned) |
| `/etc/claude-code/` (Linux/WSL), `/Library/Application Support/ClaudeCode/` (macOS), `C:\Program Files\ClaudeCode\` (Windows) | Managed policy: `CLAUDE.md`, `managed-settings.json`, managed agents/skills |

`.gitignore` additions (minimum): `.claude/settings.local.json`,
`.claude/agent-memory-local/`, `.claude/worktrees/`, `CLAUDE.local.md`,
`__pycache__/`.

`.gitattributes` additions (minimum):
`.claude/agent-memory/**/MEMORY.md merge=union` (indexes are edited from many
worktrees), `.claude/REGISTRY.md linguist-generated=true`.

---

## 4. Instruction plane

### 4.1 Root `CLAUDE.md`

Documented behavior: loaded every session; survives compaction (re-read from
disk); content is delivered as context, not enforced. Target **under 200
lines**; a file over 4 MiB is skipped; startup warns when instruction files are
too long. HTML block comments are stripped before injection, so use them for
maintainer notes at zero token cost.

Required sections (skeleton in Appendix A):

1. **Identity**: one paragraph: what the project is, stack, languages.
2. **Commands**: build, test (single slice + full), lint, format, run.
   Concrete and copy-pastable.
3. **Layout**: where slices/packages live, where specs and tests live.
4. **Workflow**: SDD → TDD loop in five lines, with the entry skills
   (`/spec-new`, `/tdd-cycle`, …).
5. **Roles and permissions**: who may edit what (enforced by hooks).
6. **Multi-session rules**: worktree per work unit, claims, messaging,
   hot files.
7. **Memory and learning rules**: when to invoke `/learn`, never store secrets.
8. **Never list**: ≤ 10 hard prohibitions (also encoded as controls).
9. **Pointers** to deeper docs (playbooks, ADRs) *without* `@`-importing them
   (imports load fully and do not save context).

Writing rules: concrete and verifiable ("run `make test-slice SLICE=x` before
committing"), no duplicated or contradictory instructions across files, no
content Claude can derive from the code (directory listings, dependency lists).
Audit with `/doctor prompt-audit` (v2.1.283+) and `/doctor`'s CLAUDE.md checks.

### 4.2 `CLAUDE.local.md`, user memory, `AGENTS.md`

* `CLAUDE.local.md` (gitignored) holds private preferences; it only exists in
  the worktree where it was created. To share across worktrees, import a file
  from the home directory (`@~/.claude/my-project.md`; external imports require
  one-time approval).
* `~/.claude/CLAUDE.md` and `~/.claude/rules/` are personal and apply to every
  project. Keep them consistent with project rules: conflicts are resolved
  arbitrarily.
* **`AGENTS.md`**: Claude reads it (v2.1.277+) only when no `CLAUDE.md` exists
  in the directory path. To share one file with other tools, make `CLAUDE.md`
  start with `@AGENTS.md` and add Claude-specific lines below. On Windows prefer
  the import over a symlink.

### 4.3 Rules (`.claude/rules/*.md`)

* One topic per file, descriptive name, subfolders allowed (recursively
  discovered). Rules **without** `paths` load at launch like CLAUDE.md; rules
  **with** `paths` load when Claude reads/writes/edits a matching file.
* Frontmatter has exactly one field: `paths` (YAML list or comma string; glob
  patterns with brace expansion). Other fields are ignored.
* Standard rule set (adapt names to the project):

| Rule file | `paths` | Content |
|---|---|---|
| `architecture.md` | source roots | Layering and dependency direction, naming |
| `testing-tdd.md` | test roots | Test pyramid, naming, fixtures, no mocking the database |
| `specs.md` | `specs/**` | Spec format, IDs, status machine, "no code in specs" |
| `security.md` | source roots, infra | Authz in the use case, input validation, secrets |
| `api-contracts.md` | API/adapter dirs | Route naming, versioning, error envelope |
| `frontend.md` | UI roots | View/ViewModel/SDK boundaries |
| `migrations.md` | migration dirs | Expand/contract, one slice owns its schema |
| `kit-governance.md` | `.claude/**`, `scripts/**` | How to change the kit (proposals, evals) |

* Keep each rule short (tens of lines). A rule that is a procedure belongs in a
  skill. Rules for on-demand loading have a per-file size cap (characters; 40,000
  for a 200K context window) after which Claude only receives the start.

### 4.4 Nested `CLAUDE.md` per slice/package

Put slice-specific invariants (ports, events emitted, tables owned, gotchas) in
`src/<slice>/CLAUDE.md`. It loads once Claude reads/edits a file in that
directory, so vertical slicing gives you free, scoped context. Keep under ~50
lines; link to the slice's spec folder.

### 4.5 Large teams and monorepos

* `claudeMdExcludes` skips irrelevant ancestor CLAUDE.md/rules files (set in
  `settings.local.json` for personal exclusions).
* `--add-dir` grants file access to other directories; to also load their
  CLAUDE.md/rules set `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`.
* Organization-wide guidance: managed `CLAUDE.md` (or the `claudeMd` key in
  managed settings). Use it for guidance; use `permissions.deny` etc. for
  enforcement.

---

## 5. Memory plane

### 5.1 Layers

| Layer | Author | Scope | Loaded | Use |
|---|---|---|---|---|
| `CLAUDE.md` / rules | Humans | Team | Every session / on match | Standards, workflow |
| `CLAUDE.local.md` | You | You + project | Every session | Personal sandbox URLs, preferences |
| **Auto memory** | Claude | Repo, shared across worktrees, **machine-local** | `MEMORY.md` first 200 lines or 25 KB each session; topic files on demand | Your corrections, preferences, context not derivable from code |
| **Agent memory** (`memory:` in agent frontmatter) | The subagent | `user` / `project` / `local` | `MEMORY.md` (200 lines / 25 KB) injected into that agent | Role-specific lessons (reviewer patterns, test idioms) |
| Specs and ADRs | Humans + agents | Repo (versioned) | On demand | Why/what of features and decisions |
| Lessons inbox | `/learn` | Machine, all worktrees (kit state dir) | Only by `/retro` | Raw material for promotion |

Auto memory notes are typed `user | feedback | project | reference`; Claude
skips what is derivable from the code or already in CLAUDE.md. It is on by
default locally; toggle with `/memory`, `autoMemoryEnabled`, or
`CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`; relocate with `autoMemoryDirectory`.
`MEMORY.md` is an index: one line per entry, details in topic files.

### 5.2 Concurrency rules for memory

* All worktrees of a repo **share one auto-memory directory**. Several sessions
  may write at once: instruct agents to create **new topic files** with unique
  names and touch the index minimally.
* Project-scope agent memory is versioned: configure `merge=union` for its
  `MEMORY.md` indexes, and review memory changes in PRs like code.
* Prefer `memory: project` for curated roles (reviewers, architect) and
  `memory: local` for noisy, personal ones.
* Memory is **not** the system of record: a rule that must hold is promoted
  (section 13), a decision is an ADR, a requirement is a spec.

### 5.3 Hygiene

* Never store secrets, tokens or personal data in any memory layer. Transcripts
  and memory are plaintext on disk.
* Review auto memory monthly (`/memory`): delete stale entries; promote
  stable ones.
* Memory files are not deleted by the retention sweep; clean them deliberately.
* `claude purge <path>` removes a project's local state when needed.

---

## 6. Capability plane: agents, skills and friends

### 6.1 How subagents work (documented facts the design depends on)

* Each subagent has its own context window, system prompt (the markdown body),
  tool allowlist, permission mode and optional hooks/MCP/skills/memory. It does
  **not** see the conversation; the delegation message carries the task.
* A non-fork subagent starts with: its prompt, the delegation message, the
  CLAUDE.md hierarchy (unless `omitClaudeMd: true`), git status, preloaded
  skills. Built-in `Explore` and `Plan` skip CLAUDE.md.
* Scope priority (highest first): managed settings, `--agents` CLI JSON,
  `.claude/agents/` (project), `~/.claude/agents/`, plugin agents. Same `name`
  → higher priority wins.
* Files are watched; new/edited agents apply within seconds. Exception:
  creating the **first** file in a brand-new `agents/` directory needs a
  restart; directories added with `--add-dir` are not watched.
* Subagents run in the background by default when spawned from the main
  conversation (fork mode is on by default from v2.1.302). Background
  subagents get a smaller built-in tool set. Concurrent subagent default cap is
  20 (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`); nesting depth default is three
  layers (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`).
* `claude --agent <name>` runs the **whole session** as that agent; its prompt
  **replaces** the default Claude Code system prompt. Use it for focused,
  self-contained roles (spec writing, kit curation, review), not for general
  coding sessions.
* Plugin agents ignore `hooks`, `mcpServers` and `permissionMode`.

### 6.2 Agent frontmatter (all documented fields)

`name` (required), `description` (required), `tools`, `disallowedTools`,
`model` (`sonnet|opus|haiku|fable|inherit|<full id>`), `permissionMode`,
`maxTurns`, `autoCompactWindow` (v2.1.296+), `skills` (preloaded, full content),
`mcpServers`, `hooks`, `memory` (`user|project|local`), `background`, `effort`
(`low|medium|high|xhigh|max`), `isolation` (`worktree`), `color`,
`initialPrompt`, `omitClaudeMd` (v2.1.271+), `experimental`. Unknown fields are
ignored silently, so validate spelling (`--debug` reports near-misses on
v2.1.296+).

Design rules:

1. **One job per agent**; description states *when* to delegate, with
   "use proactively" where appropriate. Descriptions share a budget: a startup
   warning appears when combined agent descriptions exceed 15,000 tokens.
2. **Least privilege**: always set `tools` (or `disallowedTools`). Read-only
   roles get `Read, Grep, Glob` (+ `Bash` only if they must run git/tests).
3. **Preload skills** the agent needs (`skills:`); skills with
   `disable-model-invocation: true` cannot be preloaded.
4. **Memory**: `memory: project` for roles that accumulate reusable knowledge;
   tell the agent in its prompt to consult and update memory.
5. **Model/effort**: reasoning-heavy roles (architecture, security, curation)
   `opus`/high effort; execution roles `sonnet`; mapping/search `haiku`.
   Never hard-code a model the organization may disallow without `inherit`
   as the fallback (`availableModels` can substitute). A worked three-tier
   mapping (escalation / workhorse / cheap) with an `advisor` agent that
   reproduces the native advisor pattern on any Anthropic-compatible backend
   is in [CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md](CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md).
6. **Isolation**: `isolation: worktree` for agents that make broad edits in
   parallel; their worktree is auto-removed when unchanged.
7. **Hooks inside agents** (e.g., read-only SQL validator) only run after the
   folder is trusted; project agent hooks need the workspace trust dialog.
8. **Roles map to policy**: every agent `name` has an entry in `roles.json`
   (section 7.4); unknown agents fall to a restrictive `_default`.

### 6.3 Standard agent roster (any project)

| Agent | Mission | Tools | Model/effort | Memory | Role policy (paths) |
|---|---|---|---|---|---|
| `spec-architect` | Turn intent into spec, plan, tasks; surface ambiguity; check against project constitution | Read, Grep, Glob, Write, Edit | opus / high | project | `specs/**`, `docs/adr/**` |
| `test-writer` | Write failing tests from acceptance criteria; run them to prove RED | Read, Grep, Glob, Write, Edit, Bash | sonnet / medium | project | tests only |
| `implementer` | Minimum code to pass tests; refactor on green | Read, Grep, Glob, Write, Edit, Bash | sonnet / medium | project | source only; **cannot edit tests** |
| `code-reviewer` | Review diff vs spec, rules, conventions; traceability | Read, Grep, Glob, Bash | inherit | project | read-only |
| `security-reviewer` | Review authz, input handling, secrets, dependencies | Read, Grep, Glob, Bash | opus / high | project | read-only |
| `integrator` | Merge/rebase, contracts, migrations order, lockfiles, shared files | Read, Grep, Glob, Write, Edit, Bash | inherit | project | shared paths |
| `kit-curator` | Improve agents/skills/rules from lessons and evals | Read, Grep, Glob, Write, Edit, Bash | opus / high | project | kit content (not control plane) |
| `mapper` | Fast read-only map of a slice/module for context | Read, Grep, Glob | haiku / low | none | read-only |

Add domain agents only when a job repeats ≥ 3 times with the same
instructions (e.g., `migration-writer`, `perf-analyst`). Don't override the
built-in `Explore`/`Plan` names unless you mean to.

### 6.4 Skills

Documented behavior: a skill is `SKILL.md` + optional supporting files. Slash
commands (`.claude/commands/*.md`) are merged into skills (same mechanism).
Descriptions are always in context (listing budget ≈ 1% of context window; each
entry capped at 1,536 characters of `description` + `when_to_use`); the body
loads on invocation and stays in the conversation (re-attached after compaction
within a token budget: first 5,000 tokens per skill, 25,000 total).

Frontmatter fields: `name`, `description` (recommended), `when_to_use`,
`argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`,
`allowed-tools`, `disallowed-tools`, `model`, `effort`, `context` (`fork`),
`agent`, `background`, `hooks`, `paths`, `shell`, `metadata`, `license`,
`compatibility`. Substitutions: `$ARGUMENTS`, `$N`, `$name`,
`${CLAUDE_SKILL_DIR}`, `${CLAUDE_PROJECT_DIR}`, `${CLAUDE_SESSION_ID}`.
`` !`command` `` injects command output *before* Claude sees the skill (a failed
command aborts the invocation; can be disabled by `disableSkillShellExecution`).

Design rules:

1. `description` leads with the key use case and trigger phrases.
2. `SKILL.md` < 500 lines; move reference material to supporting files and link
   them with when-to-load hints.
3. Side-effect procedures (deploy, release, scaffold, retro) →
   `disable-model-invocation: true`. Reference knowledge Claude should apply on
   its own → default invocation. Background knowledge users shouldn't call →
   `user-invocable: false`.
4. `paths:` makes a skill **auto-activate** only when working on matching files:
   the native "autodetect by file" mechanism.
5. `allowed-tools` pre-approves tools for the turn that invokes the skill; use
   exact patterns (`Bash(${CLAUDE_SKILL_DIR}/scripts/x.sh *)`), and review it as
   a privilege grant.
6. Put rules that must hold every time in a **hook**, not in the skill body.
7. Only the Agent Skills spec fields (`name`, `description`, `license`,
   `compatibility`, `metadata`, `allowed-tools`) survive upload to claude.ai /
   the Skills API; Claude Code-only fields make packaging fail.

### 6.5 Standard skill roster (any project)

| Skill | Invocation | Purpose |
|---|---|---|
| `spec-new` | model + user | Create the next spec folder from the template; ask ≤ 3 blocking questions; map to routes/permissions/controls |
| `spec-clarify` | model + user | Resolve every `[NEEDS CLARIFICATION]` before planning |
| `spec-plan` | model + user | Produce `plan.md`: architecture per layers, data, contracts, test strategy, constitution check |
| `spec-tasks` | model + user | Produce `tasks.md`: ordered, test-first, parallel-marked, AC-referenced tasks |
| `tdd-cycle` | model + user | Red → Green → Refactor protocol with role separation and evidence |
| `spec-verify` | model + user | Traceability AC → test → result; gates; verification report; status transition |
| `slice-handoff` | user only | Handoff note, contract-change notifications, claim release |
| `learn` | model + user | Capture a lesson (correction, repeated review finding) into the inbox |
| `retro` | user only | Triage lessons, propose promotions, open PR/proposal |
| `kit-audit` | user only | Run doctor, registry validation, `/doctor prompt-audit`, `/skill-doctor`, budgets report |
| `new-slice` / `new-module` | user only | Scaffold directories, nested CLAUDE.md, ownership entry, test folders |
| `new-endpoint` / `new-<artifact>` | model, `paths`-scoped | Procedure for the project's most frequent artifact (route, command, component) |

Bundled skills worth knowing: `/doctor`, `/code-review`, `/batch`, `/debug`,
`/loop`, `/verify`, `/run`, `/run-skill-generator`, `/claude-api`.
`/run-skill-generator` records how to launch your app so `/run` and `/verify`
are reliable; commit the result.

### 6.6 MCP servers and plugins

* Team MCP servers in `.mcp.json`. To keep a server's tool descriptions out of
  the main context, define it inline in the subagent that needs it
  (`mcpServers:` in the agent frontmatter).
* Treat every MCP server, plugin, marketplace and skill source as code: pin,
  review, and allowlist (`allowedMcpServers` / managed settings). Prefer
  plugins for distributing the kit across repositories; plugin skills are
  namespaced (`/plugin:skill`).
* `claude plugin validate <dir>` (v2.1.233+) checks frontmatter of
  agents/skills; `claude plugin eval` (plugins) measures delegation/trigger
  quality with and without the plugin.

### 6.7 Optional extras

Output styles (`.claude/output-styles/`), status line, custom keybindings,
dynamic workflows (`.claude/workflows/*.js` saved from `/workflows`) for
scripted orchestration of many subagents. Add them when a need appears, not by
default.

---

## 7. Control plane

### 7.1 Settings layers

| File | Scope | Shared |
|---|---|---|
| Managed policy settings | Organization | Yes, cannot be overridden by user/project |
| `--settings`, CLI flags | One session | No |
| `.claude/settings.local.json` | You, this project | No (gitignored) |
| `.claude/settings.json` | Team, this project | Yes |
| `~/.claude/settings.json` | You, all projects | No |

Managed settings win over everything (with narrow documented exceptions); array
settings such as `permissions` merge across layers. Project hooks and some
settings are subject to **workspace trust**: don't rely on them running in a
folder you haven't trusted.

Baseline keys for the project file: `permissions`, `hooks`, `worktree.baseRef`
(`fresh` | `head`), `cleanupPeriodDays`; consider `skillOverrides`,
`claudeMdExcludes` (local), `enabledPlugins`. Decide the place for **each kit
switch** (shell/launcher vs. `settings.env`) and test it: precedence between
process environment and a settings `env` entry varies per variable (see the
environment-variables reference), so verify with a probe hook instead of
assuming.

### 7.2 Permissions

* Use `allow` for safe, frequent, deterministic commands (read-only git, the
  TDD runner, the kit scripts); use `deny` for destructive or secret-reading
  operations (`git push --force`, `git reset --hard`, `rm -rf`, piping `curl`
  to a shell, `Read(./.env*)`, keys and secrets directories).
* Permissions and hooks are the enforcement layer; the `if` filter on hooks and
  Bash pattern matching are **best-effort** (shell can be obfuscated). Hard
  isolation needs the sandbox (`sandbox.*` settings) or containers.
* Subagents inherit the main session's permission rules; `permissionMode` in
  an agent is ignored when the main session is in `bypassPermissions`,
  `acceptEdits` or auto mode.
* Never run unattended sessions in `bypassPermissions` outside an isolated
  container/VM.

### 7.3 Hooks: the required set

Hooks are shell/HTTP/MCP/prompt/agent handlers fired at lifecycle events;
command hooks receive JSON on stdin and communicate with exit codes and JSON
stdout. Documented contract highlights:

* Exit `0` success (JSON fields apply); exit `2` blocking error on events that
  can block (stderr is the reason); any other exit code is a **non-blocking
  error** (the action proceeds). Exit `1` is *not* a block.
* `PreToolUse` decisions via `hookSpecificOutput.permissionDecision`
  (`allow|deny|ask|defer`) and `permissionDecisionReason`.
* `Stop` can block via top-level `{"decision":"block","reason":"…"}` to make
  Claude continue.
* Universal JSON fields: `continue`, `stopReason`, `systemMessage`,
  `terminalSequence`, `additionalContext` (inside `hookSpecificOutput`).
* `onFailure: "block"` (v2.1.295+) makes a hook **fail closed** (script missing,
  crash, timeout, invalid JSON) for `command`/`http` hooks; use it on policy
  hooks. Without it, a mistyped path silently disables the policy.
* Prefer **exec form** (`command` + `args`) with `${CLAUDE_PROJECT_DIR}` for
  paths; hooks run in parallel per event; identical handlers dedupe; default
  timeout 600 s (30 s for `UserPromptSubmit`, 1.5 s budget for `SessionEnd`).
* Hooks need an interpreter available on every developer machine and in CI;
  the kit must declare it (e.g., `python3 ≥ 3.9`, stdlib only). On Windows run
  under WSL/Git Bash or provide PowerShell variants.
* Hook output over 10,000 characters is spilled to a file; keep injected
  context small.
* `${CLAUDE_PROJECT_DIR}` does not follow a worktree entered mid-session; hooks
  receive the live directory in the `cwd` field of their JSON input.

| Hook | Event / matcher | Responsibilities | Required behaviors |
|---|---|---|---|
| **session-start** | `SessionStart` (`startup\|resume\|clear\|compact`) | Register session; auto-claim assigned slice; inject ≤ 1.5 KB of context: role, slice, spec, next open task, switches in effect, other live claims, registry drift (new/removed agents/skills) | Fast (< 2 s); `additionalContext` only; re-injects after `compact` |
| **guard** | `PreToolUse` (`Edit\|Write\|NotebookEdit\|Bash`) | (1) control-plane protection, (2) role path policy via `agent_type`, (3) shared-path and slice ownership/claims, (4) TDD gate, (5) heartbeat | `deny` with an actionable reason; modes `off\|warn\|enforce`; `onFailure: block`; best-effort Bash write detection for protected paths |
| **stop-gate** | `Stop` | Block ending a turn when production code changed without tests, or tests are RED | At most N (e.g., 2) blocks per session to avoid loops; `warn` mode only informs |
| **session-end** | `SessionEnd` | Release claims, unregister session, write log entry | Idempotent; < 1.5 s |

Optional hooks worth considering: `PostToolUse` (format/lint touched files),
`InstructionsLoaded` (audit which instruction files load and why),
`ConfigChange` (alert on settings changes), `WorktreeCreate`/`WorktreeRemove`
(non-git VCS or custom locations), `SubagentStart`/`SubagentStop` (telemetry),
`PreCompact` (block compaction in critical phases), `FileChanged`.

### 7.4 Role policy and ownership map (project-defined, control plane)

These two JSON files make the guard hook data-driven and project-agnostic.

**`roles.json`**

```json
{
  "kit_control": [".claude/hooks/**", ".claude/settings.json", ".claude/roles.json",
                  ".claude/ownership.json", "scripts/claude/**", "scripts/tdd/**"],
  "roles": {
    "free":        { "allow": ["**"], "deny": [] },
    "slice-lead":  { "allow": ["specs/**", "docs/**", ".claude/proposals/**"], "deny": [] },
    "test-writer": { "allow": ["tests/**", "**/*.test.*", "**/*_test.go", "**/test_*.py"], "deny": [] },
    "implementer": { "allow": ["src/**", "specs/**/tasks.md"], "deny": ["tests/**", "**/*.test.*"] },
    "_default":    { "allow": ["src/**"], "deny": ["tests/**", "**/*.test.*"] }
  }
}
```

The **role** of a tool call is: `agent_type` from the hook input (set inside
subagents and for `--agent` sessions) → else a launcher-provided environment
variable for the main thread → else `free`. `kit_control` paths are editable
only by a human decision (an explicit switch).

**`ownership.json`**

```json
{
  "slice_patterns": ["^src/([^/]+)/", "^tests/([^/]+)/", "^specs/([^/]+)/"],
  "non_slice_names": ["_shared", "_platform", "_templates"],
  "shared": ["src/shared/**", "**/package-lock.json", "**/go.sum", "**/Cargo.lock", "openapi/**", ".github/**"],
  "shared_roles": ["integrator"],
  "tdd_roles": ["implementer", "free", "_default"],
  "code_extensions": [".py", ".ts", ".tsx", ".js", ".go", ".rs", ".java", ".kt"],
  "test_patterns": ["(^|/)tests?/", "\\.(test|spec)\\.[a-z0-9]+$", "_test\\.(go|py|rs)$", "(^|/)test_[^/]+\\.py$"],
  "tdd_exempt": ["**/*.md", "**/*.json", "**/*.yaml", "**/*.yml", "**/*.toml", "**/*.lock"]
}
```

Adapt `slice_patterns` to your layout; the capture group is the slice name. The
same patterns drive claims, scope checks, TDD classification and the Stop gate.

### 7.5 What hooks can't do (document it)

* Bash is not a sandbox: obfuscated shell can bypass path checks. Pair hooks
  with permissions (`deny`), the sandbox, container isolation and CI checks.
* A hook that crashes is a non-blocking error unless `onFailure: "block"`.
* Hooks cannot approve on behalf of a message from another session; no agent
  message is user consent.

---

## 8. Coordination plane: many `claude` sessions at once

### 8.1 Parallelism options and when to use each

| Mechanism | Isolation | Use for |
|---|---|---|
| **Worktrees** (`git worktree`, `claude --worktree`) | Files and branch per session | The unit of parallel work (one slice/feature per session) |
| **Subagents** | Context per task, same checkout | Delegating tests/implementation/review inside one session |
| **Subagents with `isolation: worktree`** | Files per subagent | Parallel mechanical edits in one session |
| **Background agents / sessions** | Separate sessions you monitor from one view | Long tasks without blocking |
| **Cross-session messaging** (v2.1.224+) | Text messages between sessions | Hand-offs, breaking-change warnings, status |
| **Agent teams** (experimental) | Coordinated sessions Claude supervises | Large investigations; evaluate before relying |

Default topology: **1 slice-lead session per work unit** (own worktree), each
delegating the TDD loop to subagents; plus on-demand `integrator`, `reviewer`
and `kit-curator` sessions. Start with 2–3 concurrent sessions; grow only with
a working integration flow and cost visibility.

### 8.2 Work unit and naming

* **Work unit = (slice, spec)**. Branch `slice/<slice>/<spec-id>`; worktree
  directory outside `.claude/worktrees` or the default location; session name
  `<slice>:<spec-id>` (`claude --name`).
* A work unit touches **one slice**. Cross-slice needs go through the other
  slice's published contracts and its owner session.
* Shared/hot files (lockfiles, generated API specs, shared kernel, CI config,
  infrastructure) are edited **only** by the integrator role.

### 8.3 Launcher (required script)

The launcher is the single entrance for starting sessions. It must:

1. Validate mode and arguments; derive `slice`, `spec-id`, branch, worktree
   path, session name.
2. Create or reuse the worktree from the right base (`origin/<main>`), copy the
   files listed in `.worktreeinclude`, and optionally run project setup.
3. Export the kit switches from the chosen **profile** (`strict`, `standard`,
   `explore`) and `<PROJECT>_ROLE`, `<PROJECT>_SLICE`, `<PROJECT>_SPEC`.
4. Start `claude` with `--name`, and `--agent <role>` for focused modes
   (spec writing, review, integration, curation) or plain for slice-lead mode;
   pass through extra flags (`--model`, `--permission-mode`).
5. Print the switches in effect and how to release the claim.

Suggested modes: `slice <slice> <spec>`, `spec <slice> <spec>` (agent
`spec-architect`, plan permission mode for exploration), `review <ref>`
(agent `code-reviewer`), `integrate`, `curate` (agent `kit-curator`),
`free` (human pair session, relaxed profile).

### 8.4 Claim protocol (required library)

State directory: **`<git-common-dir>/claude-coord/`** (shared by all worktrees
of the repo, never committed). Layout:

```text
claims/<slice>.lock/owner.json   # atomic mkdir; {worktree, branch, role, session, spec, started, heartbeat}
sessions/<session>.json          # live sessions
tdd/<slice>-<worktree-hash>.json # {state: writing|red|green, ts}
lessons/*.md                     # learning inbox
log/YYYY-MM-DD.jsonl             # violations, claims, tdd transitions, stop-gate events
stop-counts/<session>            # loop breaker for the Stop gate
registry.seen.json               # names seen, for "new agents/skills" notices
```

Rules:

* **Claim = atomic `mkdir`** of the lock directory; a missing `owner.json` for
  < 5 s means "claim in progress", not "stale".
* **Heartbeat** on every guarded tool call; **TTL** (default 30 min, tunable) →
  a claim whose heartbeat is older or whose worktree no longer exists is stale
  and can be taken over (logged).
* Auto-claim on first edit of a slice (switchable); explicit claim at session
  start when the launcher assigns a slice; release at `SessionEnd`; `reap`
  command removes stale claims.
* A session scoped to slice A (`<PROJECT>_SLICE`) cannot edit slice B; the
  denial message points to messaging the owner.
* Write all coordination state atomically (temp file + rename).

### 8.5 Cross-session messaging

* Claude finds peers with `ListAgents` and sends with `SendMessage`; you can
  address a session with `@name`. Messages are plain text, never include
  history or files, and **never count as user approval**.
* Delivery is governed by `crossSessionInbound` (`accept|hold|refuse`). From
  project/local settings only `refuse` is effective; to accept, set it in your
  **user** settings. Default behavior: a session that prompts for permissions
  accepts; a session bypassing permissions holds messages for approval.
* `isolatePeerMachines: true` requires approval before any message leaves the
  machine. Managed settings can disable messaging organization-wide.
* Required etiquette (put in CLAUDE.md): message the owner session when you
  change a slice's published contract or a shared artifact; keep messages
  self-contained; never ask another session for an action your own permissions
  deny.

### 8.6 Contract-change and integration protocol

1. A contract change (API, event schema, shared type) is its own spec in the
   owning slice, with a compatibility statement (additive vs. breaking).
2. On merge, the owning session messages the sessions that depend on it and the
   integrator updates generated artifacts (API specs, SDKs, lockfiles).
3. Integration is **trunk-based**: small PRs, rebase before merge, required
   checks, merge queue if available; the integrator resolves conflicts in
   shared files only.
4. Migrations: unique, time-ordered names to avoid collisions; each slice owns
   its own schema.
5. Never force-push shared branches; denied in permissions.

### 8.7 Operational limits and cost control

* Default concurrent subagent cap 20; spawn depth 3; tune with the documented
  variables. Cap parallel sessions to what your integration flow and review
  capacity can absorb.
* Use `CLAUDE_CODE_SUBAGENT_MODEL` (+ `_FORCE`) only for deliberate global
  overrides; per-agent `model` is the normal control.
* `/usage`, `/insights`, `/skill-doctor` identify heavy sessions and unused
  skills; review monthly.

---

## 9. Process plane I: spec-driven development (SDD)

### 9.1 Lifecycle and artifacts

```text
idea → /spec-new → spec.md (draft) → /spec-clarify → spec.md (clarified)
     → /spec-plan → plan.md → /spec-tasks → tasks.md → TDD loop (section 10)
     → /spec-verify → verification report → status: verified → merge → closed
```

Status field in `spec.md` frontmatter:
`draft → clarified → planned → in-progress → verified → closed` (+ `superseded`).
A transition is allowed only when its gate passes (below). Specs live in git,
reviewed like code; **no implementation code in specs**.

### 9.2 Gates

| Gate | Before | Checks |
|---|---|---|
| Clarity | planning | No `[NEEDS CLARIFICATION]`; every requirement has ≥ 1 acceptance criterion; non-goals stated |
| Constitution | planning | Plan satisfies project principles (architecture, security, URL/API conventions, test strategy, compliance mapping) |
| Testability | tasking | Every AC maps to a test level (unit/integration/contract/e2e/security) and file path |
| Coverage-of-intent | implementation | Task list covers 100% of ACs; tasks are test-first and ordered by dependency |
| Verification | merge | AC → test → passing result traceability; slice tests green; review findings closed |

### 9.3 Templates (skeletons)

**`spec.md`**

```markdown
---
id: <slice>-<NNNN>
title: <short title>
slice: <slice>
status: draft            # draft|clarified|planned|in-progress|verified|closed|superseded
owner: <person or role>
links: { issue: "", adr: [], controls: [] }
---
# <title>
## Context and problem
## Goals / Non-goals
## Users and scenarios
## Requirements
- FR-001 …
- NFR-001 (performance, availability, accessibility, privacy)
## Acceptance criteria
- AC-001 (FR-001): Given … When … Then …
## Security, privacy and compliance
(permissions, tenant isolation, data classification, abuse cases, control IDs)
## Interfaces
(routes/events/contracts — names only; details in plan.md)
## Open questions
- [NEEDS CLARIFICATION] …
```

**`plan.md`**: architecture per layer (domain/application/adapters),
ports and adapters, data model and **expand/contract** migrations, contract
changes and compatibility, test strategy matrix (AC × level), rollout/feature
flags, risks and mitigations, constitution check (checked boxes with
justification for every deviation).

**`tasks.md`**

```markdown
| ID | AC | Test (path) | Implementation target | Depends | [P] | Status | Session |
|----|----|-------------|-----------------------|---------|-----|--------|---------|
| T-001 | AC-001 | tests/<slice>/…/x.test | src/<slice>/…/x | – | | [ ] | |
```

Tasks are small (≈ ≤ 1 hour), each starts with a failing test; `[P]` marks
tasks safe to run in parallel (disjoint files). A task is `[x]` only with a
green run recorded.

### 9.4 Spec ↔ code ↔ evidence traceability

* IDs: `FR-`, `NFR-`, `AC-`, `T-`; test names/comments cite the AC
  (`AC-003`); commits cite the task (`T-007`).
* `spec-verify` produces a table AC → tests → last result and appends it to the
  spec; CI can fail a PR whose spec has an AC without a test reference.
* These artifacts are SDLC evidence for audits (requirements, design, test
  evidence, approvals).

### 9.5 Changing a spec mid-flight

Edit the spec first, mark affected tasks, re-run the gates; never "fix" a test
to match behavior that contradicts the spec without a spec change recorded.

---

## 10. Process plane II: test-driven development (TDD)

### 10.1 Cycle and role separation

```text
test-writer: write failing test(s) for the next task          state: writing
test-writer: run scripts/tdd/run-tests <slice>  → expect RED   state: red
implementer: minimum code to pass                              (allowed only if red|green)
implementer: run runner → GREEN                                state: green
implementer: refactor (tests untouched, stay green)
repeat for the next task; commit as test: / feat: / refactor:
```

* **The implementer cannot edit tests; the test-writer cannot edit source**
  (role policy). This prevents "fixing" tests to go green.
* **State machine** per (worktree, slice): any test edit sets `writing`; the
  runner sets `red` (non-zero) or `green` (zero). The guard allows production
  edits only when a run is recorded (`red`/`green`), and the **Stop gate**
  refuses to end a turn with production changes lacking test changes or with
  RED tests.
* A test that passes immediately (no RED) is suspicious: the test-writer must
  confirm it exercises new behavior.

### 10.2 Runner abstraction (language-agnostic)

One entry point `scripts/tdd/run-tests <slice> [-- args]` that:

1. Resolves the slice (argument, env, or branch name).
2. Uses a **project override** (`scripts/tdd/slice-test`) when present;
   otherwise detects the runner from the nearest marker file walking up from
   the slice directory: `go.mod` → `go test` on the slice's packages;
   `Cargo.toml` → `cargo test <slice>`; `pyproject.toml`/`pytest.ini` →
   `pytest tests/<slice>`; `package.json` → `npm test -- tests/<slice>`
   (vitest/jest path filter); otherwise exit with a clear message.
3. Passes through the runner's exit code and records `red`/`green` in the
   coordination state (`--no-state` for CI).

In polyglot monorepos each service provides its own override.

### 10.3 Exceptions (explicit, logged)

| Case | Rule |
|---|---|
| Spike / exploration | Separate branch, label `spike`, never merged; or `explore` profile |
| Generated code, config, docs, lockfiles | Exempt by pattern (`tdd_exempt`) |
| Pure refactor with existing tests | Allowed in `green`; no test changes |
| Bug fix | First write a failing regression test reproducing it |
| Migrations | Need migration tests (apply from scratch, from previous version) |
| Emergency fix | Human sets mode `warn`, adds the test within the same PR; logged |

### 10.4 Quality complements

Mutation testing on domain/application code (nightly), coverage as a floor not a
goal, contract/integration/e2e levels per the project's test playbook, and the
`code-reviewer` checking that tests assert behavior (not implementation).

---

## 11. Autodetection of agents and skills

### 11.1 What Claude Code already does (documented)

| Mechanism | Detects | Notes |
|---|---|---|
| Directory discovery | `.claude/agents/`, `.claude/skills/`, `.claude/commands/`, plugin dirs, `~/.claude/*` | Project dirs discovered by walking up from the working directory to the repo root |
| **Live reload** | Edits/new files in watched dirs | Agents within seconds; skills in current session; first-ever `agents/` or `skills/` dir needs restart or `/reload-skills` |
| Description-based routing | Which agent/skill fits the task | Descriptions are the matching surface; write "use proactively" and trigger phrases |
| `paths` on skills and rules | Which skills/rules apply to the files being touched | Native file-based autodetection |
| Nested loading | Subdirectory `CLAUDE.md`, `.claude/skills/`, `.claude/rules/` | Load when Claude touches files there |
| Worktree read-through | Main checkout's untracked skills/agents/commands | v2.1.277+ for skills |
| Inspection commands | `/agents`, `/skills`, `/memory`, `/context`, `/status`, `/doctor`, `/hooks`, `/tasks` | Use them to confirm what is actually loaded |

### 11.2 What the kit adds: the registry

A stdlib-only script (`registry`) with four commands:

* `scan`: print agents, skills, rules with key attributes.
* `validate`: exit non-zero on errors (see 11.3).
* `write`: generate `.claude/REGISTRY.md` (human index, includes a content
  hash).
* `hash` / `names`: used by the SessionStart hook.

**SessionStart drift notice**: compares the current names with the last-seen
set on this machine and prints "new/removed agents, skills, rules since your
last session", and warns when `REGISTRY.md` is outdated. This is how a teammate
learns a new agent or skill exists.

### 11.3 Validation rules (the registry must enforce)

Agents: `name` and `description` present; `name` kebab-case ≤ 64 chars and
unique; `tools` or `disallowedTools` set (warn if inheriting all tools);
`model` valid; every `skills:` entry exists and is not
`disable-model-invocation`; recognized frontmatter keys only; has an entry in
`roles.json`; role write permissions consistent with tools; combined
description size under budget (warn at ~12,000 tokens, limit 15,000).
Skills: `SKILL.md` exists; `name` matches directory; `description` present and
`description + when_to_use` ≤ 1,536 chars; body < 500 lines; linked supporting
files exist; side-effect names (deploy/release/publish/migrate) must set
`disable-model-invocation`; frontmatter hooks flagged (trust requirement).
Rules: `paths` parse; non-empty; file size sane. `CLAUDE.md`: ≤ 200 lines.
Settings/policy: valid JSON; hook scripts exist and are executable.

### 11.4 Adding a new agent or skill (workflow)

1. Open an issue or a lesson explaining the repeated job.
2. `kit-curator` (or a human) drafts the file under `.claude/agents|skills`.
3. Add a `roles.json` entry via **proposal** (control plane) for new agents.
4. Add ≥ 3 positive and ≥ 3 negative trigger prompts to `.claude/evals/`.
5. Run `registry validate`, `/doctor prompt-audit`, and the trigger evals.
6. PR with CODEOWNERS review; merge; teammates see the SessionStart notice.

### 11.5 Evals

* Dataset format (kit-defined): `{ "prompt": "...", "expect": {"skill": "x"} | {"agent": "y"} | {"none": true} }`.
* Use the `skill-creator` plugin for with/without-skill comparisons and
  description tuning, and `claude plugin eval` for plugin-packaged components.
* Re-run evals whenever a description changes or a new skill/agent could
  overlap with an existing one; fix overlaps by sharpening descriptions or
  merging.

### 11.6 Diagnosing "it isn't being used"

| Symptom | Check |
|---|---|
| Agent never delegated | `/agents`; description too vague; combined descriptions over budget; name collision in a higher-priority scope; first-ever `agents/` dir requires restart |
| Skill missing in `/` menu | `/skills`; frontmatter parse error (`claude --debug`); `user-invocable: false`; `skillOverrides: off`; name refused (characters); skill in a nested dir not yet loaded |
| Skill triggers too often | Make description specific; `disable-model-invocation: true` |
| Skill stops being followed | Rule must be a hook; compaction trimmed the body: put key instructions first; re-invoke |
| Rule not applied | `/context` memory files; `paths` glob syntax; `InstructionsLoaded` hook to log loads |
| CLAUDE.md ignored | Too long or contradictory; `/memory`; instruction competes with built-in git instructions (`includeGitInstructions`) |

---

## 12. Switches and profiles

### 12.1 Kit switches (project-defined)

Use one prefix per project (`<PROJECT>_`). Each switch has modes
`off | warn | enforce` unless noted; the default when unset must be the
**safest non-blocking** value for ad-hoc sessions, while the launcher profiles
escalate.

| Switch | Values | Default (unset) | Effect |
|---|---|---|---|
| `<P>_ROLE_GUARD` | off/warn/enforce | enforce | Role path policy and control-plane protection |
| `<P>_OWNERSHIP` | off/warn/enforce | warn | Slice scope, shared-path and claim checks |
| `<P>_TDD` | off/warn/enforce | warn | Block production edits without a recorded test run |
| `<P>_STOP_GATE` | off/warn/enforce | warn | Block ending a turn without tests / with RED tests |
| `<P>_AUTOCLAIM` | 0/1 | 1 | Claim a slice on first edit |
| `<P>_CLAIM_TTL` | seconds | 1800 | Stale-claim threshold |
| `<P>_ROLE` | role name | free | Role of the main thread (subagents use `agent_type`) |
| `<P>_SLICE`, `<P>_SPEC` | strings | – | Scope of the session (set by the launcher) |
| `<P>_KIT_EDIT` | 0/1 | 0 | Human override to edit control-plane files |

### 12.2 Profiles (set by the launcher)

| Profile | Ownership | Roles | TDD | Stop gate | Autoclaim | TTL | Use |
|---|---|---|---|---|---|---|---|
| `strict` | enforce | enforce | enforce | enforce | on | 15 min | CI-like discipline, regulated work |
| `standard` | enforce | enforce | enforce | warn | on | 30 min | Default for slice sessions |
| `explore` | warn | warn | warn | off | off | 60 min | Spikes, onboarding, pair sessions |

### 12.3 Native Claude Code switches to know

| Switch | Where | Purpose |
|---|---|---|
| `--agent <name>` / `agent` setting | CLI / settings | Run the session as an agent (replaces default system prompt) |
| `--worktree`/`-w <name>`, `--name` | CLI | Isolated checkout; session name for messaging |
| `--permission-mode`, `--allowedTools`, `--disallowedTools` | CLI | Per-session permissions |
| `--settings`, `--setting-sources` | CLI | Layer or restrict settings sources |
| `--add-dir` | CLI | Extra directories (+ `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD`) |
| `--bare` (with `-p`) | CLI | Skip hooks, skills, plugins, MCP, auto memory, CLAUDE.md (scripted runs) |
| `--agents '{…}'` | CLI | Session-only agent definitions (JSON file form v2.1.281+, `-p`) |
| `autoMemoryEnabled` / `CLAUDE_CODE_DISABLE_AUTO_MEMORY` | settings / env | Auto memory |
| `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` | env | Concurrency and nesting |
| `CLAUDE_CODE_SUBAGENT_MODEL` (+ `_FORCE`) | env | Global subagent model |
| `CLAUDE_CODE_FORK_SUBAGENT=0` | env | Disable fork mode (lets Claude wait for subagents) |
| `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` | env | Disable background tasks |
| `crossSessionInbound`, `isolatePeerMachines` | settings | Cross-session messaging controls |
| `disableBundledSkills`, `skillOverrides`, `disableSkillShellExecution` | settings | Skill surface and shell injection |
| `cleanupPeriodDays`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY` | settings / env | Transcript retention / no persistence |
| `CLAUDE_CODE_NEW_INIT=1` | env | Interactive multi-phase `/init` that can set up CLAUDE.md, skills, hooks |
| `allowManagedHooksOnly`, `strictPluginOnlyCustomization`, `allowManagedPermissionRulesOnly` | managed settings | Org-level lockdown of customization |

---

## 13. Learning and improvement plane

### 13.1 The promotion ladder

```text
conversation correction
   → auto memory (Claude's own note; machine-local)           [automatic]
   → lessons inbox (/learn; shared by worktrees; unversioned)  [explicit capture]
   → agent memory (role-specific, versioned if project scope)  [curated]
   → rule (path-scoped instruction)                            [reviewed PR]
   → skill (procedure) or agent prompt change                  [reviewed PR + evals]
   → hook / permission (must-always / must-never)              [human-applied, control plane]
```

Promote **up** only when evidence repeats (≥ 2–3 occurrences or one severe
incident). Each rung has a higher cost and review bar.

### 13.2 Capture

* `learn` skill (model-invocable): when the user corrects Claude, a review
  finding recurs, a test/CI failure reveals a missing convention, or a slice
  quirk is discovered. Entry = `kind` (correction | pitfall | convention |
  tooling | process), concise statement, evidence (file/PR), suggested target
  (rule/skill/agent/hook), slice.
* Never record secrets, personal data or speculation. Entries are deduplicated
  by `retro`.

### 13.3 Triage (`retro`, weekly or per milestone)

1. List inbox entries; cluster duplicates.
2. For each cluster decide: discard, memory, rule, skill/agent edit, hook (via
   proposal), or ADR.
3. Draft the change with the **smallest** context cost; prefer editing an
   existing file to adding one; delete obsolete instructions in the same PR.
4. Run registry validation and relevant evals (section 11.5) before opening the PR.
5. Archive processed entries (the inbox must trend to empty).

### 13.4 Governance of self-improvement

* `kit-curator` may edit agents, skills, rules, evals, agent memory,
  CLAUDE.md and docs; it **cannot** edit hooks, settings, role policy,
  ownership or scripts: it writes `.claude/proposals/*.md` (problem, evidence,
  proposed diff, risk, rollback) for a human to apply.
* CODEOWNERS requires human review for `.claude/{hooks,settings.json,roles.json,ownership.json}`
  and `scripts/{claude,tdd}`.
* Every kit change is a PR with: reason, evidence, eval results, context-cost
  delta (lines/tokens), rollback plan.

### 13.5 Metrics (review monthly)

| Metric | Source |
|---|---|
| Unused skills, context cost of skill listing | `/skill-doctor`, `/doctor` |
| Skill/agent trigger precision/recall | evals |
| Guard violations by kind (role, claim, scope, TDD) | coordination log |
| Stop-gate blocks and escapes | coordination log |
| Review findings repeating across PRs | review notes → lessons |
| Lessons inbox size and age | inbox |
| CLAUDE.md/rules size and contradictions | `/doctor prompt-audit` |
| Session cost and subagent counts | `/usage`, `/insights` |
| Time from spec to merge, rework rate | PR data |

### 13.6 Retirement

Remove skills/rules/agents with no usage for a quarter or superseded by a
better one; note removal in the registry and CLAUDE.md pointers. Anything
dropped from instructions because the model improved is a win: re-audit
instructions after model upgrades (`/doctor prompt-audit`).

---

## 14. Security and governance of the kit

### 14.1 Threats the kit must address

| Threat | Control |
|---|---|
| Prompt injection via files, web, tool output, subagent reports | Least-privilege tools; permissions; subagent output scanning (built-in); no secrets reachable; treat fetched content as data |
| Agent self-escalation (editing its own hooks/permissions) | Control-plane protection in guard; CODEOWNERS; proposals instead of edits; managed settings |
| Secrets leaking into transcripts/memory | `deny Read` on secret files; no secrets in prompts; short retention; `CLAUDE_CODE_SKIP_PROMPT_HISTORY` where required; transcripts are plaintext |
| Malicious repo config (hooks/MCP/agents in a cloned repo) | Workspace trust dialog; review `.claude/` changes in PRs; `allowManagedHooksOnly` / `strictPluginOnlyCustomization` in orgs |
| Cross-session message abuse | Messages cannot approve or change config; `crossSessionInbound` policy; no commands executed from messages |
| Destructive commands | `deny` rules; sandbox/containers; protected branches; no force-push |
| Supply chain (plugins, MCP, skills from outside) | Pin and review; allowlists; run third-party plugins only after code review |
| Over-privileged skills (`allowed-tools`) | Review as privilege grants; exact patterns; managed policy can ignore repo `allowed-tools` |
| Fail-open policy hooks | `onFailure: "block"`; doctor verifies hooks exist and are executable; CI check |
| Hook interpreter missing on a machine | Doctor checks; documented prerequisites |

### 14.2 Rules

1. No secrets in the repo, prompts, CLAUDE.md, skills or memory.
2. Unattended or CI sessions run in isolated environments with scoped
   credentials; never `bypassPermissions` on a developer machine with
   production credentials.
3. Skill `` !`cmd` `` injection and hook scripts are code: review and test.
4. Control-plane changes are never applied by an agent that benefits from them.
5. Auditability: coordination logs (violations, claims, TDD transitions, stop
   gate) are append-only JSON lines; ship them to your log store if the project
   needs evidence.

### 14.3 Workspace trust

Project hooks, project-agent frontmatter hooks and inline MCP servers in project
agents run only after trusting the folder; `-p` sessions don't confer trust for
agent hooks. Tell contributors to review `.claude/` in untrusted clones before
accepting the trust dialog.

### 14.4 Cross-session and subagent boundaries

A message from another session is plain text from "another session", never from
the user; permission prompts still fire for what it requests. A subagent's final
report is scanned for text that imitates harness output but is not a security
boundary; restrict what a subagent can reach.

### 14.5 Organization layer (optional but recommended at scale)

Managed settings deployed by MDM/config management: `permissions.deny`, sandbox
requirements, `allowManagedHooksOnly`, `strictPluginOnlyCustomization`,
`allowedMcpServers`/`deniedMcpServers`, `availableModels`, `cleanupPeriodDays`,
and a managed `CLAUDE.md` for company-wide guidance. Verify with `/status`
(shows active settings sources).

---

## 15. CI/CD and compliance integration

### 15.1 CI checks (required)

1. **Kit health**: doctor (JSON validity of settings/roles/ownership; hook and
   script syntax and executable bits; registry validation; `REGISTRY.md` up to
   date; CLAUDE.md ≤ 200 lines; `claude plugin validate .claude/agents` and
   `.claude/skills` where available).
2. **Spec presence**: every PR touching a slice's source references a spec ID;
   spec status consistent with the PR (`in-progress` or later).
3. **Traceability**: every AC has a test reference; no orphan tests citing
   missing ACs.
4. **Test-first evidence**: commit history shows `test:` before `feat:` for new
   behavior (heuristic; warn first, then enforce).
5. **Ownership**: PR touches one slice unless labeled `integration`; shared files
   changed only by the integrator.
6. **Control-plane review**: CODEOWNERS enforced on protected paths.
7. **Evals**: changes to agent/skill descriptions run the trigger evals.

### 15.2 Using Claude Code in CI

Headless runs (`claude -p`) for review, release notes, triage: use `--bare` for
fast, deterministic runs that skip auto-discovery; pass explicit
`--settings`/`--allowedTools`; run in containers with scoped credentials;
output structured JSON; cap cost/time; never give CI sessions production
write access.

### 15.3 Audit evidence mapping (project-agnostic)

| SDLC control objective | Kit evidence |
|---|---|
| Requirements documented and approved | `spec.md` + review approvals |
| Secure design and threat consideration | `plan.md` constitution check, security section |
| Test-first and verification | TDD state log, commit order, `spec-verify` reports |
| Segregation of duties | Role policy; reviewer/integrator separation; CODEOWNERS |
| Change management | PR → checks → review; release evidence from CI |
| Access control for tooling | Permissions, managed settings, trust reviews |
| Logging and monitoring of tooling | Coordination logs; ConfigChange/InstructionsLoaded hooks (optional) |
| Continuous improvement | Retro records, CAPA-style lessons → promoted changes, eval results |

Map these to your framework controls (SOC 2, ISO 27001/9001) in your audit
playbook; retain logs and reports per policy; keep transcripts' retention
aligned with data-protection requirements.

---

## 16. Operating cadences

| Cadence | Activity | Owner |
|---|---|---|
| Per session | Launcher profile, claim, spec, TDD loop, handoff | Session owner |
| Daily | Review claims (`claim list`), reap stale, merge queue health | Integrator |
| Weekly | `retro` on lessons inbox; kit PRs; guard-violation review | Kit-curator + tech lead |
| Monthly | `kit-audit`: doctor, `/doctor prompt-audit`, `/skill-doctor`, metrics, memory hygiene | Tech lead |
| Quarterly | Retire unused components; review roles/ownership map; permissions and managed settings review; eval refresh | Tech lead + security |
| On Claude Code upgrade | Re-run doctor and evals; read changelog for hook/settings/frontmatter changes; re-audit instructions for obsolete workarounds | Kit owner |
| Annually | Threat-model review of the kit; compliance mapping review | Security |

---

## 17. Adoption roadmap and maturity levels

| Level | Description | Exit criteria |
|---|---|---|
| **L0 Ad hoc** | Plain `claude`, no shared config | – |
| **L1 Shared context** | `CLAUDE.md`, rules, `settings.json` permissions, `.gitignore`/`.gitattributes` | `/context` shows expected files; deny rules tested |
| **L2 Capabilities** | Core agents and skills, registry, doctor, evals seed | Registry validates; ≥ 80% of trigger evals pass |
| **L3 SDD + TDD enforced** | `specs/`, TDD runner/state, guard + stop-gate in `warn` | Specs precede code on all new work; violations visible |
| **L4 Parallel and governed** | Launcher, claims, roles, messaging etiquette, profiles `standard`, CODEOWNERS | 3+ concurrent sessions without file collisions for a month |
| **L5 Self-improving** | Retro cadence, curator, metrics, `enforce` modes, CI gates | Lessons inbox near zero; kit PRs shipped weekly with evals |

Suggested rollout: **Week 0** (L1) → **Weeks 1–2** (L2) → **Weeks 3–4** (L3 in
`warn`) → **Month 2** (L4, escalate to `enforce` per team) → **Month 3+** (L5).
Pilot with one team/slice; collect violations and friction; adjust the roles and
ownership files before generalizing.

---

## 18. Troubleshooting

| Problem | Likely cause / action |
|---|---|
| Hook doesn't run | `/hooks` to list; trust dialog not accepted; wrong path or non-executable; interpreter missing; `claude --debug` log; `disableAllHooks` set somewhere |
| Hook ran but didn't block | Exit code 1 is non-blocking (use 2 or JSON deny); invalid JSON; `if` filter mismatch; `permissionDecision` omitted |
| Policy hook silently off after a typo | Add `onFailure: "block"`; run doctor in CI |
| Guard blocks legitimate edit | Check role (`agent_type` vs launcher role), `roles.json` allow/deny, slice scope env, claim owner (`claim list`); use `explore` profile for spikes |
| Two sessions edited the same slice | Autoclaim off, TTL too long, or edits through Bash; tighten profile; add CI ownership check |
| Claim stuck after crash | `reap` (TTL) or manual release; check `SessionEnd` ran |
| TDD gate deadlock (can't edit prod, no test run) | Run the TDD runner for the slice; check state file; verify slice detection (branch name/env) |
| Stop gate loops | Confirm loop breaker counter; lower blocks; switch to `warn` |
| Messages not received | `/list-agents`; `crossSessionInbound`; version; deny rules for `SendMessage`; receiver bypassing permissions holds messages |
| Subagent lacks a tool | Background tool filter; `tools` list typo ("zero tools" error); MCP tools denied |
| Skill content lost after `/compact` | Re-invoke; move key instructions to the top of the file |
| Context bloated | `/context`; shrink CLAUDE.md/rules; convert procedures to skills; set rarely used skills to `name-only` in `skillOverrides`; `/skill-doctor` |
| Settings seem ignored | `/status` for active sources; managed settings win; JSON error (`/doctor`); env precedence for that variable |
| Worktree errors | Symlinked `.claude`, LFS filters, resume from inside worktree; see worktree docs; use manual `git worktree add` for exact branch naming |

---

## 19. Kit acceptance checklist

### Instruction plane
- [ ] Root `CLAUDE.md` ≤ 200 lines with commands, layout, SDD/TDD workflow, roles, multi-session rules, never-list, pointers.
- [ ] `.claude/rules/` has scoped rules (architecture, testing, specs, security, API/contracts, frontend if any, migrations, kit-governance).
- [ ] Nested `CLAUDE.md` for each slice/package that has its own invariants.
- [ ] `CLAUDE.local.md` documented as personal and gitignored; `AGENTS.md` interop decided.
- [ ] No contradictory instructions across user, project, local and rules (prompt-audit clean).

### Memory plane
- [ ] Auto memory enabled or deliberately disabled; hygiene cadence set.
- [ ] Agent memory scopes chosen per agent; `merge=union` configured for project-scope indexes.
- [ ] Specs and ADRs treated as durable memory; lessons inbox defined.
- [ ] No secrets in memory/transcripts; retention configured.

### Capability plane
- [ ] Agent roster defined (spec-architect, test-writer, implementer, reviewers, integrator, kit-curator, mapper) with least-privilege `tools`.
- [ ] Combined agent descriptions under the 15,000-token budget; skill listing within budget.
- [ ] Core skills present with correct invocation control; `paths` set where auto-activation by file is desired.
- [ ] Every agent has a role policy entry.
- [ ] MCP/plugins reviewed, pinned and allowlisted (if used).

### Control plane
- [ ] `settings.json` with allow/deny permissions and the four required hooks (`onFailure: block` on the guard).
- [ ] `roles.json` and `ownership.json` adapted to the project layout and tested.
- [ ] Control-plane paths protected in guard and CODEOWNERS.
- [ ] Switches and profiles documented, defaults safe for ad-hoc sessions.
- [ ] Hook interpreter prerequisites documented and checked by doctor.
- [ ] Managed settings considered for organizations.

### Coordination plane
- [ ] Launcher creates worktree + branch + env + session name for each mode.
- [ ] Claim library with atomic claims, heartbeat, TTL, reap, release on end.
- [ ] Shared/hot files assigned to the integrator role.
- [ ] Messaging etiquette and `crossSessionInbound` configuration documented.
- [ ] Contract-change and integration protocol written; limits (parallel sessions, subagents) set.

### Process plane
- [ ] `specs/` with templates, IDs, status machine and gates.
- [ ] TDD runner with detection and override, recorded red/green state, role separation, Stop gate.
- [ ] Traceability AC → test → result produced by `spec-verify`.
- [ ] Exceptions policy (spikes, emergency) with logging.

### Learning plane
- [ ] `learn` and `retro` skills, lessons inbox, promotion ladder documented.
- [ ] Registry validator, generated `REGISTRY.md`, SessionStart drift notice.
- [ ] Eval dataset and procedure for trigger quality.
- [ ] `kit-curator` constrained to non-control-plane edits; proposals folder.
- [ ] Metrics and cadences scheduled.

### Governance and CI
- [ ] Doctor runs locally and in CI; CI checks for spec presence, traceability, ownership, evals.
- [ ] Threats in section 14 mitigated; secrets denied; sandbox/container strategy for unattended runs.
- [ ] Compliance evidence mapping reviewed with the audit playbook.
- [ ] Rollout plan with pilot team and success criteria.

### Installation
- [ ] `install-user-kit.sh` ran; `kit_check.py` exits 0, or 1 with known warnings.
- [ ] `settings.json` is `chmod 600` and holds no plaintext secret.
- [ ] The working set was cloned by group (`clone-repos.sh --group=...`).
- [ ] Every active repo passes `configure-repo.sh --check`.

---

## 20. Installation and bootstrap

This section says what an install needs; [PLAYBOOK_INSTALL.md](PLAYBOOK_INSTALL.md)
is the full runbook (flags, groups, verification, rollback). Installation is
shell-only and therefore costs **zero model tokens**; the model is consulted
only for judgment calls.

**`kit/` ships the user-scope layer** — the files installed into `~/.claude`:
`kit/CLAUDE.md` (instructions), `kit/agents/` (`advisor`, `kit-curator`),
`kit/skills/` (the SDD and audit skills), `kit/templates/sdd/` and
`kit/settings.fragment.json` (model-routing env, default model and the deny
list — merged into `settings.json`, never a replacement for it).

**`scripts/claude-kit/` ships the install scripts:**

| Script | Role |
|---|---|
| `install-user-kit.sh` | Installs `kit/` into `~/.claude`; skips user-owned files unless `--force`, backs up every overwrite, merges the settings fragment |
| `clone-repos.sh` | Clones the working set by group (`core`, `products`, `tools`, `clients`, `reference`, or `all` — the default); `--dry-run` prints the plan |
| `configure-repo.sh` | Wires one repo: routing block, `.claude/README.md`, deny list, `.gitignore`, optional language kit; `--check` verifies without writing |
| `kit_check.py` | Read-only validator: agents, skills, rules, `CLAUDE.md`, settings, command collisions |
| `dedupe_plugins.py` | Moves user copies of plugin components aside (dry run by default, `--restore` puts them back) |

**A new machine runs three commands:**

```bash
bash scripts/claude-kit/install-user-kit.sh     # user scope
bash scripts/claude-kit/clone-repos.sh --group=all --dir ~/work
bash scripts/claude-kit/configure-repo.sh ~/work/<repo> --kit=<lang>
```

Every step is idempotent: re-running skips what is already there and never
duplicates the `model-routing:ollama` marker. Note that under
`permissions.defaultMode: "bypassPermissions"` the **deny** list is the
enforced lever — an `allow` list is inert — so user-scope denies must anchor
`//` or `~/`, never `./`.

---

## Appendix A — Reference templates

### A.1 `CLAUDE.md` skeleton

```markdown
# <Project name>
<One paragraph: purpose, stack, languages, multi-tenant/data sensitivity.>

## Commands
- Test one slice: `scripts/tdd/run-tests <slice>`
- Full test: `<cmd>`    - Lint/format: `<cmd>`    - Run: `<cmd>`
- Kit health: `scripts/claude/doctor`

## Layout
- `src/<slice>/` vertical slices (domain/application/adapters) · `tests/<slice>/` · `specs/<slice>/`
- Conventions: see `.claude/rules/` and `docs/adr/`; playbooks: <list, without @-imports>

## Workflow (spec → test → code)
1. `/spec-new` → `/spec-clarify` → `/spec-plan` → `/spec-tasks`
2. `/tdd-cycle`: test-writer (RED) → implementer (GREEN) → refactor
3. `/spec-verify` → PR → `/slice-handoff`
Never write production code before a recorded failing test (enforced by hooks).

## Who edits what (enforced)
spec-architect: specs/ · test-writer: tests/ · implementer: src/ · integrator: shared files ·
kit-curator: agents/skills/rules (control plane = human only)

## Multi-session rules
- One worktree per (slice, spec); start sessions with `scripts/claude/launcher`.
- Don't edit a slice claimed by another session; message its owner instead.
- Changed a contract or shared artifact? Message dependents (SendMessage) and tell the integrator.

## Memory and learning
- Corrected twice or a review finding repeats? Run `/learn`. Never store secrets.

## Never
- Commit secrets · force-push · edit hooks/settings/roles yourself · skip failing tests · disable guards to "make it work".
<!-- maintainer notes (stripped from context): keep ≤ 200 lines; move procedures to skills -->
```

### A.2 Agent template

```markdown
---
name: <kebab-case>
description: <When to delegate. One or two sentences. "Use proactively when …">
tools: Read, Grep, Glob            # least privilege; add Write/Edit/Bash only if needed
model: sonnet                      # sonnet|opus|haiku|fable|inherit
effort: medium
memory: project
skills: [<preloaded-skill>]        # must not be disable-model-invocation
---
You are <role>. Mission: <one sentence>.

When invoked:
1. Read the active spec (`specs/<slice>/<id>/spec.md`) and the slice's nested CLAUDE.md.
2. <role-specific procedure, numbered>
3. Report: <exact output format, concise>.

Boundaries: <paths you may edit / must not edit>; if blocked by a guard, report it, don't work around it.
Memory: consult your agent memory first; save durable, non-obvious learnings (new topic file, one line in MEMORY.md).
```

### A.3 Skill template

```markdown
---
name: <skill-name>
description: <What it does and when to use it. Key use case first. ≤ ~300 chars.>
when_to_use: <trigger phrases / example requests>
argument-hint: <args>
disable-model-invocation: true     # for side-effect procedures
paths: ["src/<slice>/**"]          # optional: auto-activate by file
allowed-tools: Bash(scripts/tdd/run-tests *)
---
## Context (optional dynamic injection)
!`git status --short`

## Procedure
1. …
2. …

## Done when
- <observable criteria>

See [reference.md](reference.md) for details.
```

### A.4 Rule template

```markdown
---
paths:
  - "src/**/adapters/in/http/**"
---
# API adapter rules
- <verifiable instruction>
- <verifiable instruction>
```

### A.5 Hook contract (guard, language-agnostic pseudocode)

```text
input  = JSON from stdin {tool_name, tool_input, cwd, session_id, agent_type?, …}
role   = input.agent_type or $P_ROLE or "free"
path   = relpath(tool_input.file_path, toplevel(cwd))          # Edit|Write|NotebookEdit
if path in kit_control and not $P_KIT_EDIT:           deny("control plane: write a proposal")
if not role_allows(role, path):                       deny("role may not edit path")
slice = slice_of(path)
if path in shared and role not in shared_roles:       deny("shared path: ask integrator")
if slice and scoped_slice and slice != scoped_slice:  deny("session scoped to another slice")
if slice and not try_claim(slice):                    deny("claimed by <branch>/<worktree>")
if classify(path) == "test":  tdd_state[slice] = writing
if classify(path) == "prod" and role in tdd_roles and tdd_state[slice] not in (red, green):
                                                      deny("run a failing test first")
output deny as {"hookSpecificOutput":{"hookEventName":"PreToolUse",
                "permissionDecision":"deny","permissionDecisionReason": "<actionable>"}}
warn mode: {"systemMessage": "..."}; always log violations to the coordination log.
```

### A.6 `settings.json` skeleton

```json
{
  "permissions": {
    "allow": ["Bash(git status *)", "Bash(git diff *)", "Bash(git log *)",
              "Bash(scripts/tdd/run-tests *)", "Bash(scripts/claude/doctor)"],
    "deny":  ["Bash(git push --force *)", "Bash(git reset --hard *)", "Bash(rm -rf *)",
              "Read(./.env)", "Read(./.env.*)", "Read(./secrets/**)", "Read(**/*.pem)", "Read(**/*.key)"]
  },
  "hooks": {
    "SessionStart": [{ "matcher": "startup|resume|clear|compact",
      "hooks": [{ "type": "command", "command": "python3",
                  "args": ["${CLAUDE_PROJECT_DIR}/.claude/hooks/session_start.py"], "timeout": 20 }] }],
    "PreToolUse":  [{ "matcher": "Edit|Write|NotebookEdit|Bash",
      "hooks": [{ "type": "command", "command": "python3",
                  "args": ["${CLAUDE_PROJECT_DIR}/.claude/hooks/guard.py"],
                  "timeout": 15, "onFailure": "block" }] }],
    "Stop":        [{ "hooks": [{ "type": "command", "command": "python3",
                  "args": ["${CLAUDE_PROJECT_DIR}/.claude/hooks/stop_gate.py"], "timeout": 20 }] }],
    "SessionEnd":  [{ "hooks": [{ "type": "command", "command": "python3",
                  "args": ["${CLAUDE_PROJECT_DIR}/.claude/hooks/session_end.py"], "timeout": 5 }] }]
  },
  "worktree": { "baseRef": "fresh" },
  "cleanupPeriodDays": 30
}
```

### A.7 Lesson entry format

```markdown
---
kind: correction | pitfall | convention | tooling | process
slice: <slice or empty>
target: rule | skill | agent | hook | memory | adr
ts: <ISO-8601>
---
<One or two sentences: what to do or avoid, and why.>
Evidence: <PR / file / test / session note>
```

### A.8 Proposal format (for control-plane changes)

```markdown
# Proposal: <title>
Problem and evidence · Proposed change (diff) · Risk · Rollback · Eval/validation done · Requested reviewer
```

### A.9 Eval dataset entry

```json
{ "prompt": "Write the spec for lead stage transitions", "expect": { "skill": "spec-new" } }
{ "prompt": "What does this function do?", "expect": { "none": true } }
```

---

## Appendix B — Mapping to a vertical-slice, playbook-driven project (example)

If the project follows hexagonal vertical slicing with companion playbooks:

| Kit element | Mapping |
|---|---|
| Slice | `src/slices/<slice>/` ↔ `tests/<slice>/{backend,frontend}` ↔ `specs/<slice>/` ↔ URL `{module}` ↔ permission prefix `module.resource.action` |
| Constitution | Backend, frontend, URL, test, security and audit playbooks referenced from `CLAUDE.md` and path-scoped rules (never `@`-imported) |
| Spec content | Routes follow the URL playbook; permissions listed; tenant isolation and abuse cases in the security section; control IDs from the audit playbook |
| Plan content | Layers per the backend playbook (domain/application/adapters), batch idempotency, expand/contract migrations; frontend View/ViewModel/SDK |
| Test strategy | Matrix from the test playbook (unit, integration, contract, e2e, security, concurrency, migration) per AC |
| Shared files | Shared kernel, SDK, generated API specs, lockfiles, CI/infra → integrator |
| Evidence | Spec/test traceability, release evidence bundle, guard logs → SOC 2 / ISO SDLC controls |

---

## Appendix C — Facta-Dev companion repositories

Companion repositories that plug into this kit: Agent Skill packs per stack,
a cost tracker for the metrics plane, and skill packs for the capability plane.
Treat every clone as external code: review, pin and allowlist it (§6.6, §14.1),
and accept workspace trust deliberately (§14.3).

### C.1 Stack kits and cost tracking

```bash
gh repo clone Facta-Dev/beautifulpython              # Python backend
gh repo clone Facta-Dev/beautifulrust                # Rust
gh repo clone Facta-Dev/beautifulgo                  # Go
gh repo clone Facta-Dev/beautifulnodejs              # Node.js + TypeScript
gh repo clone Facta-Dev/claude-code-cli-cost-tracker # agent cost tracking
```

| Repository | Stack / concern | What it provides | Upstream |
|---|---|---|---|
| `beautifulpython` | Python backend | Agent Skills plus a lint config that teach agents to write, review, port and enforce idiomatic Python | main project |
| `beautifulrust` | Rust | Agent Skills plus a tested clippy config for idiomatic Rust | `morganlinton/BeautifulRust` (fork) |
| `beautifulgo` | Go | Agent Skills for idiomatic Go | main project |
| `beautifulnodejs` | Node.js + TypeScript | Agent Skills plus lint config for a Fastify/Zod/tsyringe/Drizzle/BullMQ/Pino/OTel stack | main project |
| `claude-code-cli-cost-tracker` | Cost tracking | Reports what agent work actually costs; feeds the cost metrics (§13.5) and session budgets (§8.7) | main project |

Language kits install as project or user skills (`.claude/skills/`,
`~/.claude/skills/`); `paths`-scoped skills auto-activate on matching files
(§6.4). Their lint config belongs to project tooling, not to a rule file.

### C.2 Skill packs

```bash
gh repo clone Facta-Dev/skills
gh repo clone Facta-Dev/claude-skill
gh repo clone Facta-Dev/garden-skills
gh repo clone Facta-Dev/ponytail
gh repo clone Facta-Dev/ui-ux-pro-max-skill
gh repo clone Facta-Dev/superpowers
gh repo clone Facta-Dev/awesome-claude-skills
```

| Repository | Content | Upstream (fork) |
|---|---|---|
| `skills` | "Skills for Real Engineers": small, composable engineering skills | `mattpocock/skills` |
| `claude-skill` | MotionSites Web Builder: cinematic landing pages | `Motionsites-AI/claude-skill` |
| `garden-skills` | Curated production-ready skills: web design, knowledge retrieval, image generation | `ConardLi/garden-skills` |
| `ponytail` | Lazy-senior-dev behavior rules: the best code is the code never written | `DietrichGebert/ponytail` |
| `ui-ux-pro-max-skill` | Design intelligence for professional UI/UX across platforms | `nextlevelbuilder/ui-ux-pro-max-skill` |
| `superpowers` | Agentic skills framework and software-development methodology | `obra/superpowers` |
| `awesome-claude-skills` | Curated index of Claude Skills, resources and tools (default branch `master`) | `ComposioHQ/awesome-claude-skills` |

### C.3 Fork handling

The seven skill repositories above plus `beautifulrust` are Facta-Dev forks
with an `upstream` remote configured; `beautifulpython`, `beautifulgo`,
`beautifulnodejs` and `claude-code-cli-cost-tracker` are the main projects.
To refresh a fork that has no local commits:

```bash
git fetch --all --prune
git merge --ff-only upstream/main        # master for awesome-claude-skills
```

If a fork has local commits, integrate upstream through a PR; never rewrite or
force-push (`git push --force` is denied in §7.2). Agents may run the fetch and
fast-forward commands; they never push from the kit.