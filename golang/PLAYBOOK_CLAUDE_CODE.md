# PLAYBOOK_CLAUDE_CODE.md — Claude Code implementation kit

Specification of the **Claude Code implementation kit** that ships with this
Go backend template: the files, agents, skills, commands, hooks, permissions,
memory, collaboration protocol and improvement loop that make Claude Code
perform well, safely and consistently when **several `claude` CLI sessions work
in parallel** and the team practices **spec-driven development (SDD) +
test-driven development (TDD)**.

This document is the *guide*, not the kit. The kit itself is the `.claude/`
directory and `scripts/` in this repository; this file says what must be there,
why, and how to verify it.

The rules that carry the whole file:

1. **Context is a budget.** Everything always loaded (instruction file, agent
   descriptions, skill descriptions) is paid on every turn. Keep it small; load
   the rest on demand.
2. **Instructions guide, controls enforce.** Markdown is context Claude tries
   to follow. What must never be violated goes in permissions or hooks.
3. **Least privilege per role.** Each agent gets only the tools its job needs.
4. **Spec before code, test before implementation.** Artifacts and test runs
   are the evidence, not chat history.
5. **One work unit, one worktree, one owner.** Parallel sessions never edit the
   same files at once.
6. **Coordination state lives outside the working tree** and is shared by all
   worktrees.
7. **Everything learned is captured, triaged and promoted deliberately** up a
   ladder (memory → rule → skill → hook), never silently.
8. **The control plane is human-owned.** Agents may propose changes to hooks,
   permissions and role policies; they cannot apply them.
9. **Autodetect, then verify.** Rely on native discovery (descriptions,
   `paths`, live reload) and add a registry that validates what was discovered.
10. **Measure the kit.** Agents, skills and rules have evals, budgets and
    owners like any other code.

> Verification note: Claude Code evolves quickly. Before adopting a feature,
> confirm it with `claude --version`, `/doctor`, and the docs
> (<https://code.claude.com/docs>). Where this guide says "documented", it is
> stated in those docs; where it says "design choice", it is this playbook's
> recommendation.

---

## 1. Anatomy of the kit

### 1.1 Seven planes

| Plane | Purpose | Artifacts in this kit |
|---|---|---|
| **Instruction** | Tell Claude how this project works | `CLAUDE.md` (symlink to `AGENTS.md`), nested `CLAUDE.md` per slice when it earns one, optional `.claude/rules/` |
| **Memory** | Persist knowledge across sessions | auto memory, `MEMORY.md`, `docs/plans/`, `.claude/learnings.jsonl` |
| **Capability** | Reusable roles and procedures | `.claude/agents/`, `.claude/skills/`, `.claude/commands/` |
| **Control** | Enforce limits regardless of what Claude decides | `.claude/settings.json`, `scripts/hooks/guard-bash.sh`, permissions |
| **Coordination** | Let many sessions work without collisions | worktrees, branch/session naming, claim protocol (scale-up) |
| **Process** | Make work verifiable | `specs/`, `docs/plans/`, `make ci`, `specs/features/*.feature` |
| **Learning** | Improve the kit over time | `/learn`, `.claude/learnings.jsonl`, registry, cadences |

### 1.2 What ships (component inventory)

| # | Component | Status | Section |
|---|---|---|---|
| 1 | Root instruction file ≤ 200 lines | **Required** | 4 |
| 2 | `.claude/settings.json` with the guard hook | **Required** | 7 |
| 3 | Agent roster across four divisions (11 agents) | **Required** | 6.3 |
| 4 | Skills + slash commands (9 of each, 1:1) | **Required** | 6.4 |
| 5 | `manifest.json` (runtime target matrix) | **Required** | 6.5 |
| 6 | `schemas/` + `lint-agents.sh` / `lint-skills.sh` / `lint-commands.sh` | **Required** | 11 |
| 7 | `scripts/hooks/guard-bash.sh` (PreToolUse) | **Required** | 7.3 |
| 8 | `scripts/leak-guard.sh` (placeholder + secret scan) | **Required** | 11 |
| 9 | `scripts/install-claude-runtime.sh` (portable kit) | **Required** | 6.5 |
| 10 | `scripts/rename.sh` / `verify-template.sh` (factory) | **Required** | 11 |
| 11 | `learnings.jsonl` + `/learn` | **Required** (created on first use) | 13 |
| 12 | `docs/plans/` + `/plan` | **Required** (created on demand) | 9 |
| 13 | `.claude/rules/*.md` with `paths` | Optional (add when the root file grows) | 4.3 |
| 14 | Nested `CLAUDE.md` per slice | Optional (when a slice earns it) | 4.4 |
| 15 | `roles.json` / `ownership.json` / launcher / claims | Optional scale-up for multi-session | 8 |
| 16 | `REGISTRY.md`, doctor script, evals dataset | Optional scale-up | 11, 13 |

A kit is complete when every **Required** row exists and passes its lint.

---

## 2. Principles of the design

### 2.1 Where each kind of knowledge goes (decision table)

| If the content is… | Put it in… | Why |
|---|---|---|
| A fact Claude needs in **every** session (commands, layout, hard rules) | `AGENTS.md` / `CLAUDE.md` | Always loaded; keep it short |
| Needed only when touching certain files | `.claude/rules/<topic>.md` with `paths:` or nested `CLAUDE.md` | Loaded on demand |
| A multi-step **procedure** | a **skill** | Body loads only when used |
| A specialized **worker** with its own tools/context | an **agent** (subagent) | Isolated context, restricted tools |
| Something that **must always** happen / never happen | a **hook** or **permission** | Enforced by the harness |
| A personal preference | `CLAUDE.local.md`, `settings.local.json`, `~/.claude/` | Not shared |
| Knowledge Claude should accumulate | auto memory / agent memory | Written by Claude |
| A decision with rationale | ADR in `docs/adr/` | Durable, reviewable |
| The *what* and *why* of a change | `specs/` or `docs/plans/` | SDD source of truth |

### 2.2 Instructions vs. controls

CLAUDE.md content arrives as context after the system prompt; there is no
guarantee of strict compliance. If an instruction must hold at a specific point
(before every commit, after every edit, never touch a path), implement it as a
**hook** (runs at fixed lifecycle events) or a **permission rule**. Use managed
settings to make controls non-overridable.

### 2.3 Cost model

Each subagent has its own context window and its own requests (they count
toward usage limits). Delegate to a subagent when output would flood the main
conversation; do small, tightly coupled edits in the main thread. Route
low-reasoning work (search, mapping) to lower-cost models.

---

## 3. Directory structure

```text
<repo>/
├── AGENTS.md                          # canonical instructions, ≤ 200 lines    [commit]
├── CLAUDE.md                          # symlink → AGENTS.md                    [commit]
├── MEMORY.md                          # product premise, actors, slices, gates  [commit]
├── CLAUDE.local.md                    # personal, per-project                   [gitignore]
│
├── .claude/
│   ├── settings.json                  # guard hook + permissions               [commit]
│   ├── settings.local.json            # personal overrides                     [gitignore]
│   ├── manifest.json                  # runtime target matrix                  [commit]
│   ├── agents/<division>/<slug>.md    # 11 subagents                           [commit]
│   ├── skills/<slug>/SKILL.md         # 9 reusable procedures                  [commit]
│   ├── commands/<slug>.md             # 9 slash shims, 1:1 with skills         [commit]
│   ├── schemas/*.schema.md            # the linters read these                 [commit]
│   ├── learnings.jsonl                # appended by /learn; created on first use [commit]
│   └── proposals/                     # agent-written change proposals (scale-up) [commit]
│
├── docs/plans/<slug>.md               # written by /plan                        [commit]
├── specs/
│   ├── openapi.yaml                   # the HTTP contract
│   ├── api.gen.yaml                   # generated subset consumed by oapi-codegen
│   └── features/*.feature             # godog behavioral contracts
├── internal/features/<slice>/         # vertical slices (see PLAYBOOK_BACKEND.md)
├── scripts/
│   ├── hooks/guard-bash.sh            # PreToolUse Bash guard                   [control plane]
│   ├── lint-agents.sh  lint-skills.sh  lint-commands.sh                           [control plane]
│   ├── leak-guard.sh                  # placeholders + secrets                  [control plane]
│   ├── install-claude-runtime.sh      # installs the kit into other runtimes     [control plane]
│   ├── rename.sh                      # template instantiation                  [control plane]
│   └── verify-template.sh             # factory smoke test                      [control plane]
└── .github/workflows/                 # CI gates                                 [commit]
```

Outside the repository (written by Claude Code, never committed):

| Path | Content |
|---|---|
| `~/.claude/CLAUDE.md`, `~/.claude/rules/`, `~/.claude/agents/`, `~/.claude/skills/`, `~/.claude/settings.json` | Personal, all projects |
| `~/.claude/projects/<project>/memory/` | **Auto memory** (`MEMORY.md` index + topic files), shared by all worktrees of a repo, machine-local |
| `~/.claude/projects/<project>/<session>.jsonl` | Transcripts (plaintext; retention `cleanupPeriodDays`, default 30) |
| `<git-common-dir>/claude-coord/` | Coordination state when the scale-up protocol is adopted (claims, sessions, lessons inbox, logs) |

`.gitignore` additions (minimum): `.claude/settings.local.json`,
`.claude/worktrees/`, `CLAUDE.local.md`.

---

## 4. Instruction plane

### 4.1 Root instruction file

Documented behavior: loaded every session; survives compaction (re-read from
disk); content is delivered as context, not enforced. Target **under 200
lines**; a file over 4 MiB is skipped; startup warns when instruction files are
too long. HTML block comments are stripped before injection, so use them for
maintainer notes at zero token cost.

This repository follows the [AGENTS.md](https://agents.md) convention:
`AGENTS.md` is the canonical file for every tool, and `CLAUDE.md` is a symlink
to it. Claude reads `AGENTS.md` natively (v2.1.277+) only when no `CLAUDE.md`
exists in the directory path, so the symlink is what makes one file serve both.

Required sections (skeleton in Appendix A):

1. **Identity**: one paragraph: what the project is, stack, languages.
2. **Commands**: build, test (single slice + full), lint, format, run.
   Concrete and copy-pastable.
3. **Layout**: where slices/packages live, where specs and tests live.
4. **Workflow**: SDD → TDD loop in five lines, with the entry skills
   (`/spec`, `/plan`, `/tdd-guide`).
5. **Roles and permissions**: who may edit what (enforced by hooks or review).
6. **Multi-session rules**: worktree per work unit, claims, messaging,
   hot files.
7. **Memory and learning rules**: when to invoke `/learn`, never store secrets.
8. **Never list**: ≤ 10 hard prohibitions (also encoded as controls).
9. **Pointers** to deeper docs (playbooks, ADRs) *without* `@`-importing them
   (imports load fully and do not save context).

Writing rules: concrete and verifiable ("run `make ci` before declaring a
change complete"), no duplicated or contradictory instructions across files, no
content Claude can derive from the code. Audit with `/doctor prompt-audit` and
`/doctor`'s CLAUDE.md checks.

### 4.2 `CLAUDE.local.md`, user memory, `AGENTS.md`

- `CLAUDE.local.md` (gitignored) holds private preferences; it only exists in
  the worktree where it was created. To share across worktrees, import a file
  from the home directory (`@~/.claude/my-project.md`; external imports require
  one-time approval).
- `~/.claude/CLAUDE.md` and `~/.claude/rules/` are personal and apply to every
  project. Keep them consistent with project rules: conflicts are resolved
  arbitrarily.
- **`AGENTS.md`**: the canonical file here. `CLAUDE.md` is a symlink so both
  tools read the same text. On Windows prefer an `@AGENTS.md` import over a
  symlink.

### 4.3 Rules (`.claude/rules/*.md`) — optional

- One topic per file, descriptive name, subfolders allowed (recursively
  discovered). Rules **without** `paths` load at launch like CLAUDE.md; rules
  **with** `paths` load when Claude reads/writes/edits a matching file.
- Frontmatter has exactly one field: `paths` (YAML list or comma string; glob
  patterns with brace expansion). Other fields are ignored.
- Add a rule only when the root file would otherwise grow past its budget.
  Standard set when that happens:

| Rule file | `paths` | Content |
|---|---|---|
| `architecture.md` | `internal/**` | Slice layering, dependency direction, depguard |
| `testing-tdd.md` | `**/*_test.go` | Pyramid, table-driven tests, fakes, no mocked DB |
| `specs.md` | `specs/**` | Spec format, IDs, status, "no code in specs" |
| `security.md` | `internal/**`, `cmd/**` | Authz in the use case, input validation, secrets |
| `api-contracts.md` | `internal/features/**/http.go`, `specs/**` | Route naming, versioning, error envelope |
| `migrations.md` | `migrations/**`, `**/queries.sql` | Expand/contract, one slice owns its prefix |
| `kit-governance.md` | `.claude/**`, `scripts/**` | How to change the kit (proposals, evals) |

- Keep each rule short (tens of lines). A rule that is a procedure belongs in a
  skill.

### 4.4 Nested `CLAUDE.md` per slice — optional

Put slice-specific invariants (ports, events emitted, tables owned, gotchas) in
`internal/features/<slice>/CLAUDE.md`. It loads once Claude reads/edits a file
in that directory, so vertical slicing gives you free, scoped context. Keep
under ~50 lines; link to the slice's plan or spec. Do not create one for a
slice that fits in the root file.

### 4.5 Large teams and monorepos

- `claudeMdExcludes` skips irrelevant ancestor CLAUDE.md/rules files (set in
  `settings.local.json` for personal exclusions).
- `--add-dir` grants file access to other directories; to also load their
  CLAUDE.md/rules set `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`.
- Organization-wide guidance: managed `CLAUDE.md` (or the `claudeMd` key in
  managed settings). Use it for guidance; use `permissions.deny` for
  enforcement.

---

## 5. Memory plane

### 5.1 Layers

| Layer | Author | Scope | Loaded | Use |
|---|---|---|---|---|
| `AGENTS.md` / `CLAUDE.md` | Humans | Team | Every session | Standards, workflow |
| `CLAUDE.local.md` | You | You + project | Every session | Personal sandbox URLs, preferences |
| **Auto memory** | Claude | Repo, shared across worktrees, **machine-local** | `MEMORY.md` first 200 lines or 25 KB each session; topic files on demand | Corrections, preferences, context not derivable from code |
| **Agent memory** (`memory:` in agent frontmatter) | The subagent | `user` / `project` / `local` | `MEMORY.md` injected into that agent | Role-specific lessons |
| `MEMORY.md` (repo root) | Humans | Repo (versioned) | On demand | Product premise, actors, slices, stack, gates |
| `docs/plans/<slug>.md` | `/plan` | Repo (versioned) | On demand | Problem, constraints, design, risks, test plan |
| `specs/` and ADRs | Humans + agents | Repo (versioned) | On demand | Why/what of features and decisions |
| `.claude/learnings.jsonl` | `/learn` | Repo (versioned) | Grep by `kind` | Raw material for promotion |

Auto memory notes are typed `user | feedback | project | reference`; Claude
skips what is derivable from the code or already in CLAUDE.md. Toggle with
`/memory`, `autoMemoryEnabled`, or `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`.
`MEMORY.md` is an index: one line per entry, details in topic files.

### 5.2 Concurrency rules for memory

- All worktrees of a repo **share one auto-memory directory**. Several sessions
  may write at once: instruct agents to create **new topic files** with unique
  names and touch the index minimally.
- Project-scope agent memory is versioned: configure `merge=union` for its
  `MEMORY.md` indexes, and review memory changes in PRs like code.
- Prefer `memory: project` for curated roles (reviewers, architect) and
  `memory: local` for noisy, personal ones.
- Memory is **not** the system of record: a rule that must hold is promoted
  (section 13), a decision is an ADR, a requirement is a spec.

### 5.3 Hygiene

- Never store secrets, tokens or personal data in any memory layer. Transcripts
  and memory are plaintext on disk.
- Review auto memory monthly (`/memory`): delete stale entries; promote stable
  ones.
- Memory files are not deleted by the retention sweep; clean them deliberately.
- `claude purge <path>` removes a project's local state when needed.

---

## 6. Capability plane: agents, skills and commands

### 6.1 How subagents work (documented facts the design depends on)

- Each subagent has its own context window, system prompt (the markdown body),
  tool allowlist, permission mode and optional hooks/MCP/skills/memory. It does
  **not** see the conversation; the delegation message carries the task.
- A non-fork subagent starts with: its prompt, the delegation message, the
  CLAUDE.md hierarchy (unless `omitClaudeMd: true`), git status, preloaded
  skills. Built-in `Explore` and `Plan` skip CLAUDE.md.
- Scope priority (highest first): managed settings, `--agents` CLI JSON,
  `.claude/agents/` (project), `~/.claude/agents/`, plugin agents. Same `name`
  → higher priority wins.
- Files are watched; new/edited agents apply within seconds. Exception:
  creating the **first** file in a brand-new `agents/` directory needs a
  restart.
- Subagents run in the background by default when spawned from the main
  conversation. Concurrent subagent default cap is 20; nesting depth default is
  three layers.
- `claude --agent <name>` runs the **whole session** as that agent; its prompt
  **replaces** the default Claude Code system prompt. Use it for focused,
  self-contained roles, not for general coding sessions.

### 6.2 Agent frontmatter

`name` (required), `description` (required), `tools`, `disallowedTools`,
`model`, `permissionMode`, `maxTurns`, `skills`, `mcpServers`, `hooks`,
`memory`, `background`, `effort`, `isolation`, `color`, `initialPrompt`,
`omitClaudeMd`. This template's linters additionally require `division` and a
matching directory. Unknown fields are ignored silently, so validate spelling.

Design rules:

1. **One job per agent**; description states *when* to delegate, with
   "use proactively" where appropriate.
2. **Least privilege**: always set `tools` (or `disallowedTools`). Read-only
   roles get `Read, Grep, Glob` (+ `Bash` only if they must run git/tests).
3. **Preload skills** the agent needs (`skills:`).
4. **Memory**: `memory: project` for roles that accumulate reusable knowledge.
5. **Model/effort**: reasoning-heavy roles `opus`/high effort; execution roles
   `sonnet`; mapping/search `haiku`.
6. **Isolation**: `isolation: worktree` for agents that make broad edits in
   parallel.
7. **Roles map to policy**: when `roles.json` exists, every agent `name` has an
   entry; unknown agents fall to a restrictive `_default`.

### 6.3 Agent roster (shipped)

Four divisions, eleven agents. The directory must equal the `division:`
frontmatter value.

| Agent | Division | Mission | Tools |
|---|---|---|---|
| `architect` | core | Slice/kernel/platform boundaries, ports wired in `cmd/api/main.go`, depguard rules, INV-T7 construction choices | Read, Grep, Glob |
| `planner` | core | Decompose feature work into ordered, gate-aware slices ending at `make ci` | Read, Grep, Glob |
| `code-explorer` | core | Trace a request through main → `Register` → `authx` → handler, citing `file:line` | Read, Grep, Glob |
| `code-reviewer` | core | Review against `gofmt`/`vet`/`golangci-lint`/`go test`/`make ci`, sqlc discipline, envelope redaction | Read, Grep, Glob, Bash |
| `build-resolver` | core | Fix build/vet/lint/sqlc/testcontainer failures surgically | Read, Grep, Glob, Write, Edit, Bash |
| `tdd-guide` | core | Red-green-refactor with `go test` + slice fakes, 80% floor | Read, Grep, Glob, Write, Edit, Bash |
| `api-designer` | backend | net/http handler + OpenAPI entry + route registration + Deps, as one change | Read, Grep, Glob, Write, Edit |
| `migration-reviewer` | backend | goose up/down pairing, numbering, sqlc `internal/gen/db` freshness | Read, Grep, Glob |
| `ownership-reviewer` | backend | Aliasing, locking, context propagation, mutex placement, Deps wiring | Read, Grep, Glob |
| `depguard-reviewer` | security | Import boundaries: domain purity, no slice→slice, kernel-only shared, no otel, no unsafe | Read, Grep, Glob |
| `test-author` | testing | Table-driven testimy unit tests against in-test fakes, hermetic | Read, Grep, Glob, Write, Edit, Bash |

Add a domain agent only when a job repeats ≥ 3 times with the same
instructions. A new agent lands with its file, its `division`, three positive
and three negative trigger rows in `.claude/evals/`, and a `roles.json` entry
when that file exists.

### 6.4 Skills and commands (shipped)

A skill is `SKILL.md` + optional supporting files. Slash commands
(`.claude/commands/*.md`) are thin shims delegating 1:1 to the skill of the
same slug. Descriptions are always in context; the body loads on invocation and
stays in the conversation (re-attached after compaction within a token budget).

Frontmatter fields: `name`, `description`, `when_to_use`, `argument-hint`,
`arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`,
`disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`,
`hooks`, `paths`, `shell`, `metadata`, `license`, `compatibility`. This
template's linter additionally requires `division` and a set of body sections
(see `schemas/skills.schema.md`).

| Skill / command | Purpose | Invocation |
|---|---|---|
| `/add-handler` | Scaffold an HTTP route: handler, envelope, OpenAPI entry, `Register`, Deps | model + user |
| `/add-module` | Scaffold a vertical slice: `ports.go`, `usecase.go`, `module.go`, wiring, test gate | model + user |
| `/ownership-author` | Review aliasing, locking, context propagation, Deps wiring in a slice | model + user |
| `/depguard-guard` | Keep the import-boundary law green; block any depguard relaxation | model + user |
| `/review` | Pre-landing checklist: unsafe imports, error shadowing, sqlc audit, lint cleanliness | model + user |
| `/spec` | Interrogate an idea into a scoped, redaction-scanned GitHub issue | model + user |
| `/plan` | Write `docs/plans/<slug>.md`: problem, constraints, design, files, risks, tests | model + user |
| `/ship` | Version bump + `CHANGELOG.md` + release PR; never `go mod tidy`/`go.sum` | user only |
| `/learn` | Append one lesson to `.claude/learnings.jsonl` | model + user |

Design rules:

1. `description` leads with the key use case and trigger phrases.
2. `SKILL.md` < 500 lines; move reference material to supporting files.
3. Side-effect procedures (`/ship`) → `disable-model-invocation: true`.
4. `paths:` makes a skill auto-activate only when working on matching files.
5. `allowed-tools` pre-approves tools for the invoking turn; review it as a
   privilege grant.
6. Put rules that must hold every time in a **hook**, not in the skill body.

### 6.5 Manifest and portable install

`.claude/manifest.json` is the target matrix: which runtime config dirs
(`.claude`, `.cursor`, `.codex`, `.gemini`, …) receive the canonical
agents/skills/commands. `scripts/install-claude-runtime.sh` reads it and
installs the kit into one or more targets, generating the format adapters
(TOML for codex, JSON for opencode/zed) at install time. It is idempotent,
never clobbers a user-owned file, and writes `.claude/install-state.json`
provenance.

Rules:

- The canonical source is always `.claude/`. Never hand-maintain a mirror.
- A new runtime target is a manifest row plus an installer branch, reviewed as
  code.
- `claude plugin validate <dir>` checks frontmatter of agents/skills.

### 6.6 MCP servers and plugins (optional)

- Team MCP servers in `.mcp.json`. To keep a server's tool descriptions out of
  the main context, define it inline in the subagent that needs it.
- Treat every MCP server, plugin and skill source as code: pin, review, and
  allowlist. Prefer plugins for distributing the kit across repositories.

### 6.7 Optional extras

Output styles, status line, custom keybindings, dynamic workflows for scripted
orchestration of many subagents. Add them when a need appears, not by default.

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
settings such as `permissions` merge across layers. Project hooks are subject
to **workspace trust**.

Baseline keys for the project file: `permissions`, `hooks`, `worktree.baseRef`
(`fresh` | `head`), `cleanupPeriodDays`; consider `skillOverrides`,
`enabledPlugins`.

### 7.2 Permissions

- Use `allow` for safe, frequent, deterministic commands (read-only git, `go
  test`, `make ci`, the kit scripts); use `deny` for destructive or
  secret-reading operations (`git push --force`, `git reset --hard`, `rm -rf`,
  piping `curl` to a shell, `Read(./.env*)`, keys and secrets directories).
- Permissions and hooks are the enforcement layer; the `if` filter on hooks and
  Bash pattern matching are **best-effort** (shell can be obfuscated). Hard
  isolation needs the sandbox (`sandbox.*` settings) or containers.
- Never run unattended sessions in `bypassPermissions` outside an isolated
  container/VM.

### 7.3 Hooks

Hooks are shell/HTTP/MCP/prompt/agent handlers fired at lifecycle events;
command hooks receive JSON on stdin and communicate with exit codes and JSON
stdout.

- Exit `0` success (JSON fields apply); exit `2` blocking error on events that
  can block (stderr is the reason); any other exit code is a **non-blocking
  error**. Exit `1` is *not* a block.
- `PreToolUse` decisions via `hookSpecificOutput.permissionDecision`
  (`allow|deny|ask|defer`) and `permissionDecisionReason`.
- `Stop` can block via top-level `{"decision":"block","reason":"…"}`.
- `onFailure: "block"` makes a hook **fail closed**; use it on policy hooks.
- Prefer **exec form** with `${CLAUDE_PROJECT_DIR}` for paths.

Shipped in this template:

| Hook | Event / matcher | Responsibility |
|---|---|---|
| `guard-bash.sh` | `PreToolUse` (`Bash`) | Flags high-risk shell patterns (force-push, hard reset, `rm -rf`, secret reads) before they run |

Wired by `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/scripts/hooks/guard-bash.sh\"" }
        ]
      }
    ]
  }
}
```

Scale-up hooks worth adding when the team grows: `SessionStart` (inject role,
slice, spec, open task), `Stop` (block ending a turn with production changes
and no recorded test run), `SessionEnd` (release claims). Each needs its
interpreter declared and checked by the doctor.

### 7.4 Role policy and ownership map (scale-up, control plane)

When several sessions edit slices in parallel, two JSON files make the guard
hook data-driven:

**`roles.json`**

```json
{
  "kit_control": [".claude/hooks/**", ".claude/settings.json", ".claude/roles.json",
                  ".claude/ownership.json", "scripts/**"],
  "roles": {
    "free":        { "allow": ["**"], "deny": [] },
    "slice-lead":  { "allow": ["specs/**", "docs/**", ".claude/proposals/**"], "deny": [] },
    "test-writer": { "allow": ["**/*_test.go", "specs/**"], "deny": [] },
    "implementer": { "allow": ["internal/**", "cmd/**", "migrations/**"], "deny": ["**/*_test.go"] },
    "_default":    { "allow": ["internal/**", "cmd/**"], "deny": ["**/*_test.go", ".claude/**"] }
  }
}
```

**`ownership.json`**

```json
{
  "slice_patterns": ["^internal/features/([^/]+)/", "^specs/([^/]+)/"],
  "non_slice_names": ["features", "platform", "shared", "gen", "test"],
  "shared": ["internal/shared/**", "internal/platform/**", "internal/gen/**",
             "go.mod", "go.sum", "specs/openapi.yaml", ".github/**", "migrations/**"],
  "shared_roles": ["integrator"],
  "tdd_roles": ["implementer", "free", "_default"],
  "code_extensions": [".go"],
  "test_patterns": ["_test\\.go$"],
  "tdd_exempt": ["**/*.md", "**/*.json", "**/*.yaml", "**/*.yml", "**/*.sql"]
}
```

The same patterns drive claims, scope checks, TDD classification and the Stop
gate. Adapt the capture group to the layout; the capture group is the slice
name.

### 7.5 What hooks can't do (document it)

- Bash is not a sandbox: obfuscated shell can bypass path checks. Pair hooks
  with permissions (`deny`), the sandbox, container isolation and CI checks.
- A hook that crashes is a non-blocking error unless `onFailure: "block"`.
- Hooks cannot approve on behalf of a message from another session; no agent
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
| **Cross-session messaging** | Text messages between sessions | Hand-offs, breaking-change warnings, status |

Default topology: **1 slice-lead session per work unit** (own worktree), each
delegating the TDD loop to subagents; plus on-demand reviewer and integrator
sessions. Start with 2–3 concurrent sessions; grow only with a working
integration flow and cost visibility.

### 8.2 Work unit and naming

- **Work unit = (slice, spec)**. Branch `slice/<slice>/<spec-id>`; worktree
  directory outside the checkout; session name `<slice>:<spec-id>`
  (`claude --name`).
- A work unit touches **one slice**. Cross-slice needs go through the other
  slice's published contracts and its owner session.
- Shared/hot files (lockfiles, generated API specs, shared kernel, CI config,
  migrations) are edited **only** by the integrator role.

### 8.3 Launcher (required when scaling up)

The launcher is the single entrance for starting sessions. It must:

1. Validate mode and arguments; derive `slice`, `spec-id`, branch, worktree
   path, session name.
2. Create or reuse the worktree from the right base (`origin/<main>`).
3. Export the kit switches from the chosen profile and
   `<PROJECT>_ROLE`, `<PROJECT>_SLICE`, `<PROJECT>_SPEC`.
4. Start `claude` with `--name`, and `--agent <role>` for focused modes.
5. Print the switches in effect and how to release the claim.

Suggested modes: `slice`, `spec` (agent `architect`), `review` (agent
`code-reviewer`), `integrate`, `free`.

### 8.4 Claim protocol (scale-up)

State directory: **`<git-common-dir>/claude-coord/`** (shared by all worktrees,
never committed).

```text
claims/<slice>.lock/owner.json   # atomic mkdir; {worktree, branch, role, session, spec, started, heartbeat}
sessions/<session>.json          # live sessions
tdd/<slice>-<worktree-hash>.json # {state: writing|red|green, ts}
lessons/*.md                     # learning inbox
log/YYYY-MM-DD.jsonl             # violations, claims, tdd transitions
```

Rules: claim = atomic `mkdir`; heartbeat on every guarded tool call; TTL
(default 30 min) makes a stale claim reclaimable and logs it; autoclaim on
first edit of a slice (switchable); release at `SessionEnd`; `reap` removes
stale claims; write state atomically (temp file + rename).

### 8.5 Cross-session messaging

- Claude finds peers with `ListAgents` and sends with `SendMessage`; you can
  address a session with `@name`. Messages are plain text, never include
  history or files, and **never count as user approval**.
- Delivery is governed by `crossSessionInbound` (`accept|hold|refuse`). From
  project/local settings only `refuse` is effective; to accept, set it in your
  **user** settings.
- Required etiquette (put in `AGENTS.md`): message the owner session when you
  change a slice's published contract or a shared artifact; keep messages
  self-contained; never ask another session for an action your own permissions
  deny.

### 8.6 Contract-change and integration protocol

1. A contract change (API, event schema, shared type) is its own spec in the
   owning slice, with a compatibility statement (additive vs. breaking).
2. On merge, the owning session messages the sessions that depend on it and the
   integrator updates generated artifacts (`internal/gen/**`,
   `specs/api.gen.yaml`).
3. Integration is **trunk-based**: small PRs, rebase before merge, required
   checks, merge queue if available.
4. Migrations: unique, time-ordered names to avoid collisions; each slice owns
   its prefix.
5. Never force-push shared branches; denied in permissions.

### 8.7 Operational limits and cost control

- Default concurrent subagent cap 20; spawn depth 3.
- Use `CLAUDE_CODE_SUBAGENT_MODEL` (+ `_FORCE`) only for deliberate global
  overrides; per-agent `model` is the normal control.
- `/usage`, `/insights`, `/skill-doctor` identify heavy sessions and unused
  skills; review monthly.

---

## 9. Process plane I: spec-driven development (SDD)

### 9.1 Lifecycle and artifacts

```text
idea → /spec → issue (scoped, redaction-scanned, deduped)
     → /plan → docs/plans/<slug>.md
     → TDD loop (section 10) → PR → review → merge
```

For behavior that other parties depend on, the plan also adds a
`specs/features/<slice>.feature` (godog) and, when the HTTP surface changes,
`specs/openapi.yaml`. Specs live in git, reviewed like code; **no
implementation code in specs**.

### 9.2 Gates

| Gate | Before | Checks |
|---|---|---|
| Scoping | planning | The issue states problem, scope, non-goals, acceptance criteria, and the slice it touches |
| Constitution | implementation | The plan satisfies the playbooks (architecture, security, URL/API conventions, test strategy) |
| Testability | implementation | Every acceptance criterion maps to a test level (unit/integration/contract/acceptance) and file path |
| Verification | merge | Acceptance criteria → test → passing result; slice tests green; review findings closed |
| Gate | merge | `make ci` green; `golangci-lint` and depguard clean; generated code fresh |

### 9.3 Plan template (`docs/plans/<slug>.md`)

`/plan` writes: problem, constraints, design (per layer: domain/application/
adapters), data model and **expand/contract** migrations, contract changes and
compatibility, files to touch, risks and mitigations, test plan (criterion ×
level), rollout, and the playbook check (every deviation justified).

### 9.4 Traceability

- IDs: issue, acceptance criterion, task. Test names cite the criterion;
  commits cite the task.
- The reviewer checks the plan's test matrix against the diff.
- Specs and plans are SDLC evidence for audits (requirements, design, test
  evidence, approvals).

### 9.5 Changing a plan mid-flight

Edit the plan first, mark affected tasks, re-run the gates; never "fix" a test
to match behavior that contradicts the plan without recording the change.

---

## 10. Process plane II: test-driven development (TDD)

### 10.1 Cycle and role separation

```text
test-author: write failing test(s) for the next task          state: writing
test-author: run `go test ./internal/features/<slice>/...` → RED
implementer: minimum code to pass                              (allowed only if red|green)
implementer: run the suite → GREEN
implementer: refactor (tests untouched, stay green)
repeat; commit as test: / feat: / refactor:
```

- **The implementer cannot edit tests; the test-author cannot edit source**
  (role policy when `roles.json` exists). This prevents "fixing" tests to go
  green.
- **State machine** per (worktree, slice): any test edit sets `writing`; the
  runner sets `red` (non-zero) or `green` (zero). The Stop gate refuses to end
  a turn with production changes lacking test changes or with RED tests.
- A test that passes immediately (no RED) is suspicious: confirm it exercises
  new behavior.

### 10.2 Runner

One entry point `scripts/tdd/run-tests <slice> [-- args]` when the scale-up is
adopted. In this template the inner loop is plain `make test` /
`go test ./internal/features/<slice>/...`; integration is
`go test -tags=integration ./...` (testcontainers), acceptance is
`make accept` (godog).

The runner must resolve the slice, use a project override when present,
otherwise detect `go.mod` and run `go test` on the slice's packages, pass
through the exit code, and record `red`/`green` in the coordination state.

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

Mutation testing on domain/use-case code (nightly), coverage as a floor not a
goal, contract/integration/acceptance levels per `PLAYBOOK_TESTS.md`, and
`code-reviewer` checking that tests assert behavior, not implementation.

---

## 11. Autodetection, registry and kit CI

### 11.1 What Claude Code already does (documented)

| Mechanism | Detects | Notes |
|---|---|---|
| Directory discovery | `.claude/agents/`, `.claude/skills/`, `.claude/commands/`, plugin dirs, `~/.claude/*` | Project dirs discovered by walking up from the working directory |
| **Live reload** | Edits/new files in watched dirs | Agents within seconds; skills in current session; first-ever dir needs restart |
| Description-based routing | Which agent/skill fits the task | Descriptions are the matching surface |
| `paths` on skills and rules | Which skills/rules apply to touched files | Native file-based autodetection |
| Nested loading | Subdirectory `CLAUDE.md`, `.claude/skills/`, `.claude/rules/` | Load when Claude touches files there |
| Inspection commands | `/agents`, `/skills`, `/memory`, `/context`, `/status`, `/doctor`, `/hooks`, `/tasks` | Confirm what is actually loaded |

### 11.2 The kit's validators

This template uses **schema-driven bash linters** instead of a generated
`REGISTRY.md`:

- `.claude/schemas/agents.schema.md`, `skills.schema.md`,
  `commands.schema.md`, `install-state.schema.md` declare the required
  frontmatter fields, body sections, allowed divisions and failure modes.
- `scripts/lint-agents.sh [root]`, `lint-skills.sh`, `lint-commands.sh` parse
  the schema doc and validate the tree, so the linter and the schema **cannot
  drift**. They check: required fields present, `division` allowed and equal to
  the directory, required `##` sections present, name matches directory/slug,
  description length, side-effect skills marked `disable-model-invocation`.
- `scripts/leak-guard.sh` fails on a surviving `<app>`/`<APP>`/`<scope>`
  placeholder after rename, and on high-signal secret patterns in tracked text
  files.
- `scripts/verify-template.sh` is the factory smoke test (staged rename +
  throwaway Postgres); it belongs to the template, not to day-to-day work.

Run all of them before opening a PR that touches `.claude/` or `scripts/`.

### 11.3 Adding a new agent or skill (workflow)

1. Open an issue or a lesson explaining the repeated job.
2. Draft the file under `.claude/agents/<division>/` or
   `.claude/skills/<slug>/`; add the 1:1 command shim.
3. Add three positive and three negative trigger prompts to `.claude/evals/`.
4. Run `lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh`, and `/doctor`.
5. PR with review; teammates see the new files on their next session.

### 11.4 Evals

- Dataset format: `{ "prompt": "...", "expect": {"skill": "x"} | {"agent": "y"} | {"none": true} }`.
- Re-run evals whenever a description changes or a new skill/agent could
  overlap with an existing one; fix overlaps by sharpening descriptions or
  merging.

### 11.5 Diagnosing "it isn't being used"

| Symptom | Check |
|---|---|
| Agent never delegated | `/agents`; description too vague; combined descriptions over budget; name collision in a higher-priority scope; first-ever `agents/` dir requires restart |
| Skill missing in `/` menu | `/skills`; frontmatter parse error (`claude --debug`); `user-invocable: false`; `skillOverrides: off` |
| Skill triggers too often | Make description specific; `disable-model-invocation: true` |
| Skill stops being followed | Rule must be a hook; compaction trimmed the body: put key instructions first; re-invoke |
| Rule not applied | `/context` memory files; `paths` glob syntax |
| CLAUDE.md ignored | Too long or contradictory; `/memory` |

---

## 12. Switches and profiles

### 12.1 Kit switches (project-defined)

Use one prefix per project: this template uses `<APP>_`. Each switch has modes
`off | warn | enforce` unless noted; the default when unset must be the
**safest non-blocking** value for ad-hoc sessions, while the launcher profiles
escalate.

| Switch | Values | Default (unset) | Effect |
|---|---|---|---|
| `<APP>_ROLE_GUARD` | off/warn/enforce | enforce | Role path policy and control-plane protection |
| `<APP>_OWNERSHIP` | off/warn/enforce | warn | Slice scope, shared-path and claim checks |
| `<APP>_TDD` | off/warn/enforce | warn | Block production edits without a recorded test run |
| `<APP>_STOP_GATE` | off/warn/enforce | warn | Block ending a turn without tests / with RED tests |
| `<APP>_AUTOCLAIM` | 0/1 | 1 | Claim a slice on first edit |
| `<APP>_CLAIM_TTL` | seconds | 1800 | Stale-claim threshold |
| `<APP>_ROLE` | role name | free | Role of the main thread (subagents use `agent_type`) |
| `<APP>_SLICE`, `<APP>_SPEC` | strings | – | Scope of the session (set by the launcher) |
| `<APP>_KIT_EDIT` | 0/1 | 0 | Human override to edit control-plane files |

`<APP>_AUTOCLAIM` and `<APP>_CLAIM_TTL` configure the deferred launcher; the
shipped hook does not read them, so claims stay manual until the launcher
lands.

### 12.2 Profiles (set by the launcher)

| Profile | Ownership | Roles | TDD | Stop gate | Autoclaim | TTL | Use |
|---|---|---|---|---|---|---|---|
| `strict` | enforce | enforce | enforce | enforce | on | 15 min | CI-like discipline, regulated work |
| `standard` | enforce | enforce | warn | warn | on | 30 min | Default for slice sessions |
| `explore` | warn | warn | warn | off | off | 60 min | Spikes, onboarding, pair sessions |

### 12.3 Native Claude Code switches to know

| Switch | Where | Purpose |
|---|---|---|
| `--agent <name>` / `agent` setting | CLI / settings | Run the session as an agent |
| `--worktree`/`-w <name>`, `--name` | CLI | Isolated checkout; session name for messaging |
| `--permission-mode`, `--allowedTools`, `--disallowedTools` | CLI | Per-session permissions |
| `--settings`, `--setting-sources` | CLI | Layer or restrict settings sources |
| `--add-dir` | CLI | Extra directories (+ `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD`) |
| `--bare` (with `-p`) | CLI | Skip hooks, skills, plugins, MCP, auto memory, CLAUDE.md |
| `autoMemoryEnabled` / `CLAUDE_CODE_DISABLE_AUTO_MEMORY` | settings / env | Auto memory |
| `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` | env | Concurrency and nesting |
| `CLAUDE_CODE_SUBAGENT_MODEL` (+ `_FORCE`) | env | Global subagent model |
| `crossSessionInbound`, `isolatePeerMachines` | settings | Cross-session messaging controls |
| `disableBundledSkills`, `skillOverrides`, `disableSkillShellExecution` | settings | Skill surface and shell injection |
| `cleanupPeriodDays`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY` | settings / env | Transcript retention / no persistence |

---

## 13. Learning and improvement plane

### 13.1 The promotion ladder

```text
conversation correction
   → auto memory (Claude's own note; machine-local)           [automatic]
   → .claude/learnings.jsonl (/learn; versioned)               [explicit capture]
   → agent memory (role-specific, versioned if project scope)  [curated]
   → rule (path-scoped instruction)                            [reviewed PR]
   → skill (procedure) or agent prompt change                  [reviewed PR + evals]
   → hook / permission (must-always / must-never)              [human-applied, control plane]
```

Promote **up** only when evidence repeats (≥ 2–3 occurrences or one severe
incident). Each rung has a higher cost and review bar.

### 13.2 Capture

`/learn` appends one JSON line to `.claude/learnings.jsonl` (created on first
use). Invoke it when: the user corrects Claude; a review finding recurs; a
test/CI failure reveals a missing convention; a slice quirk is discovered.

Entry fields: `kind` (correction | pitfall | convention | tooling | process),
statement, evidence (file/PR), suggested target (rule/skill/agent/hook), slice,
timestamp. Never record secrets, personal data or speculation.

### 13.3 Triage (`retro`, weekly or per milestone)

1. List entries; cluster duplicates.
2. For each cluster decide: discard, memory, rule, skill/agent edit, hook (via
   proposal), or ADR.
3. Draft the change with the **smallest** context cost; prefer editing an
   existing file to adding one; delete obsolete instructions in the same PR.
4. Run the kit linters and relevant evals before opening the PR.
5. Archive processed entries (the log must not grow unbounded).

### 13.4 Governance of self-improvement

- An agent may edit agents, skills, rules, evals, agent memory, CLAUDE.md and
  docs; it **cannot** edit hooks, settings, role policy, ownership or scripts
  unattended: it writes `.claude/proposals/*.md` (problem, evidence, proposed
  diff, risk, rollback) for a human to apply.
- CODEOWNERS requires human review for `.claude/{settings.json,manifest.json,schemas/**}`
  and `scripts/**`.
- Every kit change is a PR with: reason, evidence, eval results, context-cost
  delta (lines/tokens), rollback plan.

### 13.5 Metrics (review monthly)

| Metric | Source |
|---|---|
| Unused skills, context cost of skill listing | `/skill-doctor`, `/doctor` |
| Skill/agent trigger precision/recall | evals |
| Guard violations by kind | coordination log |
| Stop-gate blocks and escapes | coordination log |
| Review findings repeating across PRs | review notes → lessons |
| Learnings log size and age | `.claude/learnings.jsonl` |
| CLAUDE.md/rules size and contradictions | `/doctor prompt-audit` |
| Session cost and subagent counts | `/usage`, `/insights` |
| Time from plan to merge, rework rate | PR data |

### 13.6 Retirement

Remove skills/rules/agents with no usage for a quarter or superseded by a
better one; note removal in the changelog and pointers. Anything dropped from
instructions because the model improved is a win: re-audit instructions after
model upgrades (`/doctor prompt-audit`).

---

## 14. Security and governance of the kit

### 14.1 Threats the kit must address

| Threat | Control |
|---|---|
| Prompt injection via files, web, tool output, subagent reports | Least-privilege tools; permissions; no secrets reachable; treat fetched content as data |
| Agent self-escalation (editing its own hooks/permissions) | Control-plane protection in guard; CODEOWNERS; proposals instead of edits; managed settings |
| Secrets leaking into transcripts/memory | `deny Read` on secret files; no secrets in prompts; short retention; `CLAUDE_CODE_SKIP_PROMPT_HISTORY` where required |
| Malicious repo config in a cloned repo | Workspace trust dialog; review `.claude/` changes in PRs |
| Cross-session message abuse | Messages cannot approve or change config; `crossSessionInbound` policy |
| Destructive commands | `deny` rules; sandbox/containers; protected branches; no force-push; `guard-bash.sh` |
| Supply chain (plugins, MCP, skills from outside) | Pin and review; allowlists |
| Over-privileged skills (`allowed-tools`) | Review as privilege grants; exact patterns |
| Fail-open policy hooks | `onFailure: "block"`; CI check that hook scripts exist and are executable |
| Hook interpreter missing on a machine | Doctor/CI check; documented prerequisites |

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

Project hooks and inline MCP servers in project agents run only after trusting
the folder. Tell contributors to review `.claude/` in untrusted clones before
accepting the trust dialog.

### 14.4 Cross-session and subagent boundaries

A message from another session is plain text from "another session", never from
the user; permission prompts still fire for what it requests. A subagent's final
report is scanned for harness imitations but is not a security boundary;
restrict what a subagent can reach.

### 14.5 Organization layer (optional at scale)

Managed settings deployed by MDM/config management: `permissions.deny`, sandbox
requirements, `allowManagedHooksOnly`, `strictPluginOnlyCustomization`,
`allowedMcpServers`/`deniedMcpServers`, `availableModels`, `cleanupPeriodDays`,
and a managed `CLAUDE.md` for company-wide guidance. Verify with `/status`.

---

## 15. CI/CD and compliance integration

### 15.1 CI checks (required)

1. **Kit health**: `lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh`,
   `leak-guard.sh`; JSON validity of settings/manifest; hook scripts exist and
   are executable; CLAUDE.md ≤ 200 lines.
2. **Spec/plan presence**: every PR touching slice source references a plan or
   issue ID; status consistent with the PR.
3. **Traceability**: every acceptance criterion has a test reference; no orphan
   tests citing missing criteria.
4. **Test-first evidence**: commit history shows `test:` before `feat:` for new
   behavior (heuristic; warn first, then enforce).
5. **Ownership**: PR touches one slice unless labeled `integration`; shared
   files changed only by the integrator.
6. **Control-plane review**: CODEOWNERS enforced on protected paths.
7. **Evals**: changes to agent/skill descriptions run the trigger evals.

### 15.2 Using Claude Code in CI

Headless runs (`claude -p`) for review, release notes, triage: use `--bare` for
fast, deterministic runs that skip auto-discovery; pass explicit
`--settings`/`--allowedTools`; run in containers with scoped credentials; output
structured JSON; cap cost/time; never give CI sessions production write access.

### 15.3 Audit evidence mapping (project-agnostic)

| SDLC control objective | Kit evidence |
|---|---|
| Requirements documented and approved | plan/issue + review approvals |
| Secure design and threat consideration | plan constitution check, security section |
| Test-first and verification | TDD state log, commit order, verification reports |
| Segregation of duties | Role policy; reviewer/integrator separation; CODEOWNERS |
| Change management | PR → checks → review; release evidence from CI |
| Access control for tooling | Permissions, managed settings, trust reviews |
| Logging and monitoring of tooling | Coordination logs; ConfigChange hooks (optional) |
| Continuous improvement | Retro records, CAPA-style lessons → promoted changes, eval results |

Map these to the framework controls in `PLAYBOOK_AUDIT.md`; retain logs and
reports per policy; keep transcript retention aligned with data-protection
requirements.

---

## 16. Operating cadences

| Cadence | Activity | Owner |
|---|---|---|
| Per session | Profile, claim, plan, TDD loop, handoff | Session owner |
| Daily | Review claims (`claim list`), reap stale, merge queue health | Integrator |
| Weekly | `retro` on learnings; kit PRs; guard-violation review | Kit-curator + tech lead |
| Monthly | `kit-audit`: doctor, `/doctor prompt-audit`, `/skill-doctor`, metrics, memory hygiene | Tech lead |
| Quarterly | Retire unused components; review roles/ownership; permissions review; eval refresh | Tech lead + security |
| On Claude Code upgrade | Re-run doctor and evals; read changelog for hook/settings/frontmatter changes; re-audit instructions | Kit owner |
| Annually | Threat-model review of the kit; compliance mapping review | Security |

---

## 17. Adoption roadmap and maturity levels

| Level | Description | Exit criteria |
|---|---|---|
| **L0 Ad hoc** | Plain `claude`, no shared config | – |
| **L1 Shared context** | AGENTS.md/CLAUDE.md, settings permissions, `.gitignore` | `/context` shows expected files; deny rules tested |
| **L2 Capabilities** | Agents, skills, commands, schema linters, leak guard | All linters green; ≥ 80% of trigger evals pass |
| **L3 SDD + TDD enforced** | Plans, TDD loop, `make ci` gate, guard in `warn` | Plans precede code on all new work; violations visible |
| **L4 Parallel and governed** | Launcher, claims, roles, messaging etiquette, profiles, CODEOWNERS | 3+ concurrent sessions without file collisions for a month |
| **L5 Self-improving** | Retro cadence, curator, metrics, `enforce` modes, CI gates | Learnings log triaged weekly; kit PRs shipped with evals |

Suggested rollout: **Week 0** (L1) → **Weeks 1–2** (L2) → **Weeks 3–4** (L3 in
`warn`) → **Month 2** (L4, escalate to `enforce` per team) → **Month 3+** (L5).
Pilot with one slice; collect violations and friction; adjust the roles and
ownership files before generalizing.

---

## 18. Troubleshooting

| Problem | Likely cause / action |
|---|---|
| Hook doesn't run | `/hooks` to list; trust dialog not accepted; wrong path or non-executable; `claude --debug`; `disableAllHooks` set somewhere |
| Hook ran but didn't block | Exit code 1 is non-blocking (use 2 or JSON deny); `if` filter mismatch; `permissionDecision` omitted |
| Policy hook silently off after a typo | Add `onFailure: "block"`; run the linters in CI |
| Guard blocks legitimate edit | Check role (`agent_type` vs launcher role), `roles.json` allow/deny, slice scope env, claim owner; use `explore` profile for spikes |
| Two sessions edited the same slice | Autoclaim off, TTL too long, or edits through Bash; tighten profile; add CI ownership check |
| Claim stuck after crash | `reap` (TTL) or manual release; check `SessionEnd` ran |
| TDD gate deadlock | Run `go test` for the slice; check state file; verify slice detection |
| Stop gate loops | Confirm loop breaker counter; lower blocks; switch to `warn` |
| Messages not received | `/list-agents`; `crossSessionInbound`; version; deny rules for `SendMessage` |
| Subagent lacks a tool | Background tool filter; `tools` list typo; MCP tools denied |
| Skill content lost after `/compact` | Re-invoke; move key instructions to the top of the file |
| Context bloated | `/context`; shrink CLAUDE.md/rules; convert procedures to skills; `/skill-doctor` |
| Settings seem ignored | `/status` for active sources; managed settings win; JSON error (`/doctor`); env precedence |
| Worktree errors | Symlinked `.claude`, LFS filters; use manual `git worktree add` for exact branch naming |

---

## 19. Kit acceptance checklist

### Instruction plane
- [ ] Root `AGENTS.md` ≤ 200 lines with commands, layout, SDD/TDD workflow,
      roles, multi-session rules, never-list, pointers.
- [ ] `CLAUDE.md` is a symlink to `AGENTS.md` (or an `@AGENTS.md` import).
- [ ] No contradictory instructions across user, project and local files.
- [ ] Nested `CLAUDE.md` exists only where a slice has invariants worth
      loading separately.

### Memory plane
- [ ] Auto memory enabled or deliberately disabled; hygiene cadence set.
- [ ] `MEMORY.md` states product premise, actors, slices, stack and gates.
- [ ] Agent memory scopes chosen per agent; `merge=union` for project indexes.
- [ ] `docs/plans/` and `specs/` treated as durable memory.
- [ ] No secrets in memory/transcripts; retention configured.

### Capability plane
- [ ] Agent roster present across `core`, `backend`, `security`, `testing`,
      with least-privilege `tools` and a matching `division`.
- [ ] Skills and commands present 1:1; side-effect skills marked
      `disable-model-invocation`.
- [ ] `manifest.json` target matrix reviewed; installer idempotent.
- [ ] Combined agent/skill descriptions within budget.

### Control plane
- [ ] `settings.json` with allow/deny permissions and the `guard-bash.sh` hook.
- [ ] Control-plane paths protected by review (CODEOWNERS when applicable).
- [ ] Switches and profiles documented; defaults safe for ad-hoc sessions.
- [ ] Hook interpreter prerequisites documented and checked by CI.
- [ ] Managed settings considered for organizations.

### Coordination plane (scale-up)
- [ ] Launcher creates worktree + branch + env + session name per mode.
- [ ] Claim library with atomic claims, heartbeat, TTL, reap, release on end.
- [ ] Shared/hot files assigned to the integrator role.
- [ ] Messaging etiquette and `crossSessionInbound` documented.
- [ ] Contract-change and integration protocol written.

### Process plane
- [ ] `/spec` and `/plan` produce scoped, testable artifacts.
- [ ] TDD loop documented; `make ci` is the gate.
- [ ] Acceptance criteria → test → result traceability produced by `/review`.
- [ ] Exceptions policy (spikes, emergency) with logging.

### Learning plane
- [ ] `/learn` and the promotion ladder documented.
- [ ] Kit linters run locally and in CI.
- [ ] Eval dataset and procedure for trigger quality.
- [ ] Agents constrained from unattended control-plane edits; proposals folder.
- [ ] Metrics and cadences scheduled.

### Governance and CI
- [ ] The threats in section 14 are mitigated; secrets denied; sandbox/container
      strategy for unattended runs.
- [ ] Compliance evidence mapping reviewed with `PLAYBOOK_AUDIT.md`.
- [ ] Rollout plan with pilot slice and success criteria.

---

## Appendix A — Reference templates

### A.1 `AGENTS.md` skeleton

```markdown
# <Project name>
<One paragraph: purpose, stack, languages, data sensitivity.>

## Commands
- Test one slice: `go test ./internal/features/<slice>/...`
- Full test: `make test`    - Gate: `make ci`
- Integration: `make test-int`    - Acceptance: `make accept`
- Lint/format: `make lint` / `gofmt -w .`
- Run: `make run`    - Kit health: `bash scripts/lint-agents.sh && bash scripts/lint-skills.sh && bash scripts/lint-commands.sh`

## Layout
- `internal/features/<slice>/` vertical slices · `specs/` · `test/` · `docs/plans/`
- Conventions: see `.claude/` and the playbooks; playbooks: <list, without @-imports>

## Workflow (spec → test → code)
1. `/spec` → `/plan` → `/add-module` or `/add-handler`
2. Red → green → refactor with `go test`; `/tdd-guide` when in doubt
3. `/review` → PR; `make ci` before declaring done

## Who edits what
architect/planner: plans and specs · test-author: `*_test.go` ·
implementers: `internal/**`, `cmd/**` · integrator: shared files ·
control plane (`.claude/settings.json`, `scripts/**`) = human only

## Multi-session rules
- One worktree per (slice, spec); start sessions with the launcher.
- Don't edit a slice claimed by another session; message its owner instead.
- Changed a contract or shared artifact? Message dependents and tell the integrator.

## Memory and learning
- Corrected twice or a review finding repeats? Run `/learn`. Never store secrets.

## Never
- Commit secrets · force-push · edit hooks/settings unattended · skip failing
  tests · disable depguard to "make it work".
<!-- maintainer notes (stripped from context): keep ≤ 200 lines; move procedures to skills -->
```

### A.2 Agent template

```markdown
---
name: "<app> <Role>"
description: "<When to delegate. One or two sentences. 'Use proactively when …'>"
division: backend                    # core | backend | security | testing
color: "#2563EB"
tools: Read, Grep, Glob              # least privilege; add Write/Edit/Bash only if needed
model: sonnet                        # sonnet|opus|haiku|fable|inherit
effort: medium
memory: project
---

# <app> <Role>

## Core Mission
<one paragraph>

## When invoked
1. Read the plan (`docs/plans/<slug>.md`) and the slice's files.
2. <role-specific procedure, numbered>
3. Report: <exact output format, concise>.

## Boundaries
<paths you may edit / must not edit>; if blocked by a guard, report it, don't work around it.
```

### A.3 Skill template

```markdown
---
name: <skill-name>
description: <What it does and when to use it. Key use case first. ≤ ~300 chars.>
division: backend
when_to_use: <trigger phrases / example requests>
argument-hint: <args>
disable-model-invocation: true     # for side-effect procedures
paths: ["internal/features/**"]    # optional: auto-activate by file
allowed-tools: Bash(go test *)
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
  - "internal/features/**/http.go"
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

### A.6 `settings.json` skeleton (scale-up)

```json
{
  "permissions": {
    "allow": ["Bash(git status *)", "Bash(git diff *)", "Bash(git log *)",
              "Bash(go test *)", "Bash(make ci)", "Bash(make test *)",
              "Bash(bash scripts/lint-*.sh)"],
    "deny":  ["Bash(git push --force *)", "Bash(git reset --hard *)", "Bash(rm -rf *)",
              "Read(./.env)", "Read(./.env.*)", "Read(./secrets/**)", "Read(**/*.pem)"]
  },
  "hooks": {
    "SessionStart": [{ "matcher": "startup|resume|clear|compact",
      "hooks": [{ "type": "command", "command": "bash",
                  "args": ["${CLAUDE_PROJECT_DIR}/scripts/hooks/session-start.sh"], "timeout": 20 }] }],
    "PreToolUse":  [{ "matcher": "Edit|Write|NotebookEdit|Bash",
      "hooks": [{ "type": "command", "command": "bash",
                  "args": ["${CLAUDE_PROJECT_DIR}/scripts/hooks/guard.sh"],
                  "timeout": 15, "onFailure": "block" }] }],
    "Stop":        [{ "hooks": [{ "type": "command", "command": "bash",
                  "args": ["${CLAUDE_PROJECT_DIR}/scripts/hooks/stop-gate.sh"], "timeout": 20 }] }],
    "SessionEnd":  [{ "hooks": [{ "type": "command", "command": "bash",
                  "args": ["${CLAUDE_PROJECT_DIR}/scripts/hooks/session-end.sh"], "timeout": 5 }] }]
  },
  "worktree": { "baseRef": "fresh" },
  "cleanupPeriodDays": 30
}
```

### A.7 Lesson entry format

```json
{"kind":"correction","slice":"authsession","target":"rule",
 "ts":"2026-11-08T10:00:00Z",
 "statement":"Read the actor from authx.Identity, never from the request body.",
 "evidence":"internal/features/authsession/http.go:42","promoted":false}
```

### A.8 Proposal format (for control-plane changes)

```markdown
# Proposal: <title>
Problem and evidence · Proposed change (diff) · Risk · Rollback · Eval/validation done · Requested reviewer
```

### A.9 Eval dataset entry

```json
{ "prompt": "Add a PATCH route for widgets", "expect": { "skill": "add-handler" } }
{ "prompt": "What does this function do?", "expect": { "none": true } }
```

---

## Appendix B — Mapping to this vertical-slice, playbook-driven template

| Kit element | Mapping |
|---|---|
| Slice | `internal/features/<slice>/` ↔ `specs/features/<slice>.feature` ↔ route prefix in `PLAYBOOK_URLS.md` |
| Constitution | `PLAYBOOK_BACKEND.md`, `PLAYBOOK_URLS.md`, `PLAYBOOK_TESTS.md`, `PLAYBOOK_SECURITY.md`, `PLAYBOOK_AUDIT.md`, referenced from `AGENTS.md` and path-scoped rules (never `@`-imported) |
| Plan content | Layers per the backend playbook (domain/application/adapters), batch idempotency, expand/contract migrations |
| Test strategy | Matrix from `PLAYBOOK_TESTS.md` (unit, integration, contract, acceptance, security, concurrency, migration) per criterion |
| Shared files | `internal/shared/**`, `internal/platform/**`, `internal/gen/**`, `go.mod`, `specs/openapi.yaml`, `.github/**` → integrator |
| Evidence | Plan/test traceability, release evidence bundle, guard logs → SOC 2 / ISO SDLC controls |
