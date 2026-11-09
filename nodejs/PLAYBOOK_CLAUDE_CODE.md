# PLAYBOOK_CLAUDE_CODE.md — Claude Code implementation kit (Node backend)

How the `<app>` Node repository turns Claude Code (or any AGENTS.md-aware
agent) into a controlled engineering harness: what ships in `.claude/`, what
each plane is responsible for, which gates it must pass, and how the harness
is governed when several sessions work at once.

Premise for product, roles and slices: `MEMORY.md`. Architecture, routes,
tests and security contracts: the other five playbooks.

**The one rule that carries the whole file: the kit is code.** It is linted,
contract-tested, versioned and reviewed like the TypeScript under `src/` —
not a folder of prompts that drifts silently.

---

## 1. Anatomy of the kit

### 1.1 Seven planes

| Plane                 | Question it answers             | Files                                                                    |
| --------------------- | ------------------------------- | ------------------------------------------------------------------------ |
| Instruction           | What every session is told      | `AGENTS.md`, `CLAUDE.md` (symlink)                                       |
| Memory                | What persists across sessions   | `MEMORY.md`, `.claude/learnings.jsonl`                                   |
| Capability            | What the agent can do on demand | `.claude/agents/`, `.claude/skills/`, `.claude/commands/`                |
| Control               | What it may not do              | `.claude/settings.json`, `scripts/hooks/guard-bash.sh`, tool permissions |
| Coordination          | How sessions share the repo     | worktrees, ownership map, messaging conventions                          |
| Process               | How work moves                  | `/spec` → `/plan` → red/green → `/review` → `/ship`                      |
| Learning / governance | How the kit improves safely     | `/learn`, evals, lint scripts, manifest                                  |

### 1.2 What ships

| Component          | Count    | Detail                                                                                                                                                                                          |
| ------------------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agents             | 11       | `backend` (3), `security` (1), `testing` (1), `core` (6)                                                                                                                                        |
| Skills             | 9        | one `SKILL.md` per slug under `.claude/skills/<slug>/`                                                                                                                                          |
| Commands           | 9        | thin 1:1 shims under `.claude/commands/`                                                                                                                                                        |
| Schemas            | 4        | `agents.schema.md`, `skills.schema.md`, `commands.schema.md`, `install-state.schema.md`                                                                                                         |
| Validators         | 3        | `scripts/lint-agents.sh`, `lint-skills.sh`, `lint-commands.sh`                                                                                                                                  |
| Contract evals     | 14 tests | `evals/contracts.test.mjs` via Node's built-in `node --test`; zero installed packages (INV-T16)                                                                                                 |
| Live evals         | 2 tiers  | `evals/run-live.sh` — manually triggered, real API budget                                                                                                                                       |
| Hooks              | 1        | `PreToolUse` on `Bash` → `scripts/hooks/guard-bash.sh`                                                                                                                                          |
| Installed runtimes | 14       | `claude`, `claude-project`, `cursor`, `antigravity`, `codex`, `gemini`, `opencode`, `codebuddy`, `joycode`, `qwen`, `zed`, `hermes`, `openclaw`, `kimi`; the hook targets `claude-project` only |
| Scripts            | 8        | rename, verify-template, leak-guard, 3 linters, installer, hook                                                                                                                                 |

The kit needs no package installation of its own. Contract evals run on the
Node 24 runtime already required by the app (`evals/package.json` only sets
`"type": "module"`); the validators are pure bash plus POSIX `grep`/`sed`.
Frontmatter is parsed by hand, not by a YAML library. That is deliberate: a
kit that needs its own dependency tree is a second supply chain.

---

## 2. Principles of the design

### 2.1 Where each kind of knowledge goes

| Knowledge                                       | Home                          | Why                                    |
| ----------------------------------------------- | ----------------------------- | -------------------------------------- |
| Invariant every session must know               | `AGENTS.md` (short)           | always in context                      |
| Product premise, roles, data model              | `MEMORY.md`                   | stable, loaded by hand or by reference |
| Deep contract (architecture, tests, security)   | `PLAYBOOK_*.md`               | read on demand, not injected wholesale |
| Reusable procedure                              | `SKILL.md`                    | invoked by name, versioned             |
| One-shot delegation with its own context window | agent under `.claude/agents/` | keeps the main context clean           |
| A hard "never do this"                          | hook in `settings.json`       | enforced by the harness, not by trust  |
| What went wrong last time                       | `.claude/learnings.jsonl`     | append-only, greppable                 |
| A decision that affects the release             | PR + `CHANGELOG.md`           | durable, reviewable                    |

### 2.2 Instructions vs. controls

Instructions are advisory: an agent that ignores `AGENTS.md` still runs.
Controls are mechanical: a hook that exits `2` stops the tool call. Put a rule
in the instruction layer when it needs judgment ("prefer a two-variant union
over a boolean parameter"); put it in the control layer when the cost of one
mistake is high ("never force-push `main`"). Do not duplicate: a rule stated
twice gets maintained once.

### 2.3 Cost model

- The root instruction file is paid on every request. Keep it under ~200
  lines; anything longer is a playbook.
- Agents and skills are paid only when invoked. Deep, specific instructions
  belong there.
- Subagents pay their own context; delegate exploration and review rather
  than filling the main window with file dumps.
- The live eval tier costs real API budget; it never runs on push or PR.

---

## 3. Directory structure

```text
<app>/
├── AGENTS.md                     # root instruction file (canonical)
├── CLAUDE.md -> AGENTS.md        # symlink; one source of truth
├── MEMORY.md                     # product premise, actors, slices, invariants
├── PLAYBOOK_*.md                 # 6 deep contracts
├── docs/                         # numbered design docs 00–14 + architecture/*.json
├── .claude/
│   ├── settings.json             # PreToolUse hook wiring
│   ├── manifest.json             # module/target matrix for the installer
│   ├── schemas/
│   │   ├── agents.schema.md
│   │   ├── skills.schema.md
│   │   ├── commands.schema.md
│   │   └── install-state.schema.md
│   ├── agents/
│   │   ├── backend/   api-designer · migration-reviewer · ownership-reviewer
│   │   ├── core/      architect · build-resolver · code-explorer
│   │   │              code-reviewer · planner · tdd-guide
│   │   ├── security/  depguard-reviewer
│   │   └── testing/   test-author
│   ├── skills/        add-handler · add-module · depguard-guard · learn
│   │                  ownership-author · plan · review · ship · spec
│   └── commands/      one <slug>.md per skill (delegation shim)
├── scripts/
│   ├── rename.sh                  # instantiate the template
│   ├── verify-template.sh         # staged rename + full gate in a temp copy
│   ├── leak-guard.sh              # placeholder + secret scan
│   ├── lint-agents.sh
│   ├── lint-skills.sh
│   ├── lint-commands.sh
│   ├── install-claude-runtime.sh  # materialize kit for other agent runtimes
│   └── hooks/guard-bash.sh        # PreToolUse tripwire
└── evals/
    ├── package.json               # private, "type": "module", no dependencies
    ├── contracts.test.mjs         # 14 fixture-driven contract tests
    ├── fixtures/{valid,invalid}/  # one fixture per failure mode
    └── run-live.sh                # tier 2 smoke + tier 3 LLM judge
```

`AGENTS.md` follows the open [AGENTS.md](https://agents.md) standard. Codex,
Cursor, Windsurf, Copilot, Aider and Gemini read it natively; `CLAUDE.md` is a
symlink so there is exactly one file to edit.

---

## 4. Instruction plane

### 4.1 Root instruction file

`AGENTS.md` carries only rules that prevent a real, recurring mistake:

- English only, everywhere (code, comments, UI copy, test names, docs).
- A `.gitignore` comment goes on its own line, never on a pattern line.
- `make ci` green before declaring a change complete.
- The eslint `no-restricted-imports` groups are the architecture law: never
  weaken them, never silence them with `eslint-disable` or `@ts-expect-error`.
- No unexplained suppressions; `@ts-ignore`/`@ts-nocheck` forbidden.
- Never fabricate paths, exports, env vars, test results or commit hashes.
- Touch only what the request needs.
- INV-T3: the template ships no `pnpm-lock.yaml`; decide deliberately after
  instantiation.
- INV-T7: `<APP>_OFFLINE=true` is a hard offline guard.
- The slice layout and the allowed dependency edges.
- Commands (`make ci`, `make accept`, `make migrate-up`, `make vuln`).
- Who edits what and the multi-session rules.
- The playbook table.

Do not paste whole playbooks into it. It is an index plus hard rules, and it
is pruned: a line that would not have prevented an actual mistake is deleted.

### 4.2 `CLAUDE.local.md`, user memory, `AGENTS.md`

- `CLAUDE.local.md` is for machine-local overrides (a local database port, a
  personal API base URL). Git-ignore it.
- User-level `~/.claude/CLAUDE.md` is not part of this repo and must not be
  required for the repo to work.
- If the project also targets Codex/Gemini, `AGENTS.md` is the only source;
  `CLAUDE.md` and `GEMINI.md` are symlinks, never copies.

### 4.3 Rules directory — not shipped

`.claude/rules/*.md` is optional in Claude Code. This kit does not ship it:
the root file plus the six playbooks already cover the rules, and a rules
directory would be a third place to keep in sync. Add it only when a second
product in the same repo needs a different rule set loaded by path.

### 4.4 Nested `CLAUDE.md` per slice — not shipped

A nested instruction file per slice is useful when each slice has a genuinely
different workflow. Here every slice follows the same eight-file playbook, so
a nested file would repeat the root. The playbook is the per-slice guide.

### 4.5 Large teams and monorepos

If the repo becomes a multi-product monorepo:

1. Keep one root `AGENTS.md` with only cross-product rules.
2. Add a nested `CLAUDE.md` per product directory with its own commands.
3. Keep one `.claude/` at the root; do not duplicate the agent roster per
   product — agents take the product as input.
4. Split the playbooks by product only when the contracts actually diverge.

---

## 5. Memory plane

### 5.1 Layers

| Layer                     | Lifetime                  | Content                                                                                                                                             |
| ------------------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MEMORY.md`               | product lifetime          | premise, actors/roles, slice→table map, stack, invariants, gate commands, contract pointers                                                         |
| `.claude/learnings.jsonl` | append-only               | one JSON object per lesson, created on the first `/learn`                                                                                           |
| `docs/plans/<slug>.md`    | per change                | the written plan; created on the first `/plan`                                                                                                      |
| PR body + `CHANGELOG.md`  | permanent                 | what shipped and why                                                                                                                                |
| `pnpm-lock.yaml`          | decision at instantiation | the template ships none (INV-T3). If committed after rename, it is the resolved dependency set; `pnpm audit` and reproducible installs depend on it |

`MEMORY.md` is not a dump of the README. It holds the facts an agent needs to
reason correctly and cannot cheaply derive: who the actors are, what the roles
mean, which table each slice owns, which invariants are load-bearing, and
which command proves a change. It must not invent tables or routes — verify
against `migrations/` and the slices' `module.ts` before writing them down.

### 5.2 Concurrency rules for memory

- `learnings.jsonl` is append-only; two sessions may append, none rewrites.
- `MEMORY.md` is edited through a normal PR; a session that learned a new
  invariant proposes the diff rather than editing in place on a shared branch.
- A plan file is owned by its `<slug>`; do not edit a plan another session
  claims.

### 5.3 Hygiene

- One lesson per line, concrete and dated:
  `{"date":"2026-11-08","kind":"pitfall","text":"...","scope":"widgets"}`.
- Grep by `kind` before repeating work:
  `grep '"kind":"pitfall"' .claude/learnings.jsonl`.
- Delete a lesson when the underlying issue is fixed (model upgrade, refactor,
  removed feature). A stale learning is worse than none.
- Never store a secret, a token, a customer name or a connection string.

---

## 6. Capability plane: agents, skills and commands

### 6.1 How subagents work

- An agent is a markdown file with YAML frontmatter and a fixed body shape.
  Claude Code discovers it by walking `.claude/agents/`; no registry edit.
- A subagent gets its own context window and returns a summary. Use one when
  the work is a broad search or an independent review and the main session
  should keep the conclusion, not the file dumps.
- A skill is a procedure the main agent loads on demand; a command is a thin
  shim that delegates 1:1 to the skill of the same slug.
- Subagents inherit the session's permission mode. The kit's hook applies to
  their `Bash` calls too.

### 6.2 Agent frontmatter

Every agent carries exactly four fields, enforced by
`.claude/schemas/agents.schema.md` and `scripts/lint-agents.sh`:

```yaml
---
name: "<app> Migration Reviewer"
description: "Reviews node-pg-migrate migrations for <app>; flags unpaired up/down files, unsafe expand/contract ordering, and adapter SQL that does not match the migrated schema."
color: "#7C3AED"
division: backend
---
```

- `name` — display name, unique in the roster.
- `description` — one sentence, action-first, used for discovery.
- `color` — hex color for the UI.
- `division` — one of `backend`, `security`, `testing`, `core`.

Body sections, in order: `Core Mission`, `Expertise Areas`, `Output Format`,
`Quality Standards`, `Guiding Principles`. The linter fails on a missing
section, a bad division, a directory/division mismatch or CRLF line endings.

### 6.3 Agent roster

| Agent                | Division | Use it for                                                                                                                       |
| -------------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `architect`          | core     | port boundaries, constructor injection, the `"imports"` map (`#shared/*`, `#platform/*`), selection-over-abstraction             |
| `planner`            | core     | ordered, gate-checked slices across `src/features/`                                                                              |
| `code-explorer`      | core     | trace a request through `src/bin/api.ts` → `module.ts` → `http.ts` → `usecase.ts` → `ports.ts` → `postgres.ts`, cite `file:line` |
| `code-reviewer`      | core     | the gate plus invariants: eslint architecture groups, `tsc`, vitest, error envelope, parameterized Kysely SQL                    |
| `build-resolver`     | core     | strict-mode type errors, ESM/subpath import resolution, Kysely type drift, eslint-rule strategy                                  |
| `tdd-guide`          | core     | red-green-refactor with vitest and fake ports                                                                                    |
| `api-designer`       | backend  | Fastify 5 route shape, `preHandler` order, OpenAPI entry, `module.ts` mount                                                      |
| `migration-reviewer` | backend  | paired `up`/`down` migrations, expand/contract ordering, adapter/schema drift                                                    |
| `ownership-reviewer` | backend  | ownership predicates in SQL, 404-not-403, rank checks, async interleaving and `AbortSignal`                                      |
| `depguard-reviewer`  | security | dependency direction, INV-T7 offline guard, dependency additions; guards the eslint `no-restricted-imports` groups               |
| `test-author`        | testing  | vitest unit tests and cucumber step definitions                                                                                  |

Invoke one when its job is genuinely independent. Do not spawn a reviewer with
your own conclusion in the prompt — give it the diff and let it disagree.

### 6.4 Skills and commands

| Slug                | Division | What it does                                                                                                 |
| ------------------- | -------- | ------------------------------------------------------------------------------------------------------------ |
| `/add-handler`      | backend  | scaffold a handler: Zod decode via `httpx`, `preHandler` order, `AppError`, OpenAPI entry, `module.ts` mount |
| `/add-module`       | backend  | scaffold `src/features/<slug>`: seven files, paired migration, composition-root wiring, test gate            |
| `/ownership-author` | backend  | line-anchored ownership/rank/async report before landing                                                     |
| `/depguard-guard`   | security | dependency-direction and offline-guard review; blocks weakening the eslint groups or the `"imports"` map     |
| `/review`           | core     | four-category pre-landing checklist → severity-ranked verdict                                                |
| `/spec`             | core     | turn an idea into a scoped issue, redaction-scan it, dedupe, `gh issue create`                               |
| `/plan`             | core     | write `docs/plans/<slug>.md`                                                                                 |
| `/learn`            | core     | append a lesson to `.claude/learnings.jsonl`                                                                 |
| `/ship`             | core     | cut a release PR from conventional commits; user-invoked only                                                |

Every command file contains a `## Delegate` section pointing at the skill of
the same slug; `lint-commands.sh` fails if the slug has no matching skill.
`/spec` (`gh issue create`), `/ship` (release PR) and the live evals are the
only actions with external side effects.

### 6.5 Manifest and portable install

`.claude/manifest.json` declares each module's source directory and the
runtime targets it applies to. `claude` and `claude-project` are always on;
the other 12 targets are opt-in. `hooks` is the exception: it applies to
`claude-project` only.

```sh
bash scripts/install-claude-runtime.sh --targets=codex,cursor
bash scripts/install-claude-runtime.sh --dest=/path/to/project
```

The installer is pure bash, hardcodes the install of agents/skills/commands,
and installs the hook only for `claude-project`, only if absent — it never
clobbers a user's `settings.json`. `codex` gets TOML; `opencode` and `zed` get
JSON. `install-state.schema.md` describes the state file it writes.

### 6.6 MCP servers and plugins — not shipped

The kit does not require an MCP server or a plugin. Adding one means: a
project-scoped config, a documented credential path, and a note in
`AGENTS.md` that the capability exists. Do not add an MCP server for
something a CLI (`gh`, `pnpm`, `docker`) already does.

### 6.7 Optional extras

- A `statusline` is convenience, not governance; skip it.
- A worktree launcher is useful only at the scale described in §8.3; the kit
  ships no launcher.
- A `retro` command is optional; `/learn` covers per-session capture.

---

## 7. Control plane

### 7.1 Settings layers

| Layer            | File                          | Committed?       |
| ---------------- | ----------------------------- | ---------------- |
| User             | `~/.claude/settings.json`     | no               |
| Project (shared) | `.claude/settings.json`       | yes              |
| Project (local)  | `.claude/settings.local.json` | git-ignored      |
| Enterprise       | managed policy                | outside the repo |

The repo ships exactly one hook:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash \"$CLAUDE_PROJECT_DIR/scripts/hooks/guard-bash.sh\""
          }
        ]
      }
    ]
  }
}
```

No `permissions` block ships. Permission prompts are a session-level choice,
not a repo contract; a committed allowlist that is too broad is worse than
none. Add one only when the team has agreed on a specific tool boundary.

### 7.2 Permissions

Recommended posture:

- `acceptEdits` for a session on an owned worktree; `plan` mode for design.
- Ask before `Bash` commands with external effect (`git push`, `gh pr create`,
  `docker push`, `make migrate-up` against a non-local database, anything
  touching production).
- `bypassPermissions` only in an isolated container with no credentials.

### 7.3 Hooks

`guard-bash.sh` is a tripwire, not a sandbox. It reads the hook JSON on stdin,
matches the raw command string, and fails open on any parse problem.

**Blocks** (exit `2`):

| Pattern                                                                                                        | Reason                     |
| -------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `rm -rf` targeting `/`, `/etc`, `/usr`, `/var`, `/bin`, `/lib`, `/opt`, `/boot`, `/dev`, `/home`, `~`, `$HOME` | irreversible system damage |
| `git push --force` / `-f` to `main` or `master` without `--force-with-lease`                                   | append-only history rule   |
| `chmod -R 777`                                                                                                 | world-writable recursive   |
| `dd … of=/dev/…`                                                                                               | raw device overwrite       |

**Warns** (stderr, exit `0`):

| Pattern                                                    | Reason                               |
| ---------------------------------------------------------- | ------------------------------------ |
| `rm -rf` on an absolute path outside `/tmp`                | verify the target                    |
| `git push --force-with-lease`                              | still rewrites remote history        |
| `DROP TABLE` / `TRUNCATE` outside `migrations/` or `test/` | destructive SQL in the wrong context |

Constraints the hook honors: pure bash plus POSIX `grep`/`sed` (no `jq`, no
`node`, no `awk`), rename-inert (contains no template placeholder literals),
`bash -n` clean, and invocable via `bash <path>` without an exec bit.

### 7.4 Ownership map

| Area                                                                                                    | Owner                                                             |
| ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `docs/plans/`, `specs/openapi.yaml`                                                                     | `planner`, `architect`, `api-designer`                            |
| `*.test.ts`, `test/acceptance/**`, `specs/features/**`                                                  | `test-author`                                                     |
| `src/features/<slice>/**`                                                                               | the slice's implementer                                           |
| `src/shared/**`, `src/platform/**`, `eslint.config.mjs`, `package.json`, `tsconfig*.json`, `.github/**` | shared — integrator role                                          |
| `.claude/settings.json`, `.claude/manifest.json`, `.claude/schemas/**`, `scripts/**`                    | human only; an agent writes a proposal under `.claude/proposals/` |

The last row is deliberate: an agent that can rewrite its own guardrails is
not governed. Changes to the control plane go through a human-reviewed PR.

### 7.5 What hooks cannot do

- They cannot judge intent, only text. Complex quoting can evade the pattern,
  and quoted prose can over-match.
- They cannot see subagent context, only the tool call.
- They cannot enforce a rule that requires reading a diff or a test result.
- They cannot replace CI: the hook runs on the developer's machine; CI runs on
  a clean checkout.

State this in the PR when adding a hook. A hook that claims more than it does
is security theater.

---

## 8. Coordination plane: many sessions at once

### 8.1 Parallelism options

| Option                              | When                                                          |
| ----------------------------------- | ------------------------------------------------------------- |
| One session, sequential             | small repo, coupled work, early project                       |
| Several sessions, one worktree each | independent slices; the default for team scale                |
| Subagents inside one session        | exploration and review that must not pollute the main context |
| Remote/cloud sessions               | long runs that outlive the laptop                             |

### 8.2 Work unit and naming

- One worktree per `(slice, spec)`.
- Name the session `<slice>:<spec-id>` (for example `widgets:WID-142`).
- Never two sessions on the same slice: the second edits a file the first is
  mid-change on, and the merge is a guessing game.

### 8.3 Worktree launcher — not shipped

The kit ships no launcher script. At team scale, the minimal launcher is a
shell function that creates `git worktree add ../<repo>-<slice> -b <slice>/<spec>`,
copies `.env`, and starts the session. Add one when a human is doing this by
hand more than once a day.

### 8.4 Claim protocol

There is no claim database in the kit. The protocol is social and visible:

1. A session announces the `(slice, spec)` it claims in the team channel.
2. A second session that wants the same slice messages the owner instead of
   editing.
3. A shared-module change (`src/shared`, `src/platform`, `eslint.config.mjs`,
   `specs/openapi.yaml`, package manifests) is announced to every dependent
   slice before merge.
4. The integrator merges shared-module changes; slice owners do not.

Add a `docs/claims/` file only if the team outgrows the channel.

### 8.5 Cross-session messaging

Agents can message other live sessions when the harness supports it. Use it
for: "I am changing the `WidgetRepository` port, your adapter will need the
new method", "the migration I just landed renames a column you read". Do not
use it for status updates; the PR is the status.

### 8.6 Contract-change protocol

A change to a port interface, a shared module, an error `code`, the error
envelope, the route table, `specs/openapi.yaml` or a migration is a **contract
change**:

1. Open the PR against the shared area first.
2. List every dependent slice in the PR body.
3. Get the integrator's review, not just a slice owner's.
4. Land the migration and the shared change before dependent code.
5. Run `make generate` and pass `make gen-check`; update the OpenAPI spec in
   the same PR. There is no route-registration guard — the review is the
   guard.

### 8.7 Limits and cost control

- Two failed corrections on the same issue: stop, summarize, restart with a
  sharper prompt. Long sessions with accumulated failures are worse than
  fresh ones.
- Keep subagent fan-out proportional: one explorer, one reviewer, not five of
  each.
- The live eval tier and `/ship` are the only budgeted external calls; they
  never run by default.

---

## 9. Process plane I: spec-driven development

### 9.1 Lifecycle and artifacts

```text
idea → /spec (scoped issue) → /plan (docs/plans/<slug>.md) → implementation
     → /review → make ci → make accept → /ship (release PR)
```

| Artifact   | Owner       | Content                                                        |
| ---------- | ----------- | -------------------------------------------------------------- |
| Issue      | `/spec`     | problem, scope, non-goals, acceptance criteria, redaction scan |
| Plan       | `/plan`     | problem, constraints, design, files-to-touch, risks, test plan |
| Diff       | implementer | one slice per PR where possible                                |
| Release PR | `/ship`     | version bump, changelog, one PR                                |

### 9.2 Gates

| Gate             | Command                                                                         | Blocks                             |
| ---------------- | ------------------------------------------------------------------------------- | ---------------------------------- |
| Format           | `make fmt-check` (`prettier --check`)                                           | merge                              |
| Typecheck        | `make vet` (`tsc --noEmit`, app + test configs)                                 | merge                              |
| Lint             | `make lint` (`eslint .`)                                                        | merge                              |
| Generated models | `make gen-check`                                                                | merge when the API surface changed |
| Unit tests       | `make test` (vitest)                                                            | merge                              |
| Aggregate gate   | `make ci` = the five above                                                      | merge                              |
| Acceptance       | `make accept` (cucumber over the fake-wired app)                                | merge                              |
| Dependency audit | `make vuln` (`pnpm audit --audit-level=high`)                                   | release / CI                       |
| Kit contracts    | `node --test evals/contracts.test.mjs`                                          | kit changes (CI: see §15)          |
| Template smoke   | `bash scripts/verify-template.sh`                                               | template changes                   |
| Migrations       | `make migrate-up` against a live Postgres; `verify-template.sh` runs up/down/up | release                            |

`make ci` does not include `make accept`, `make vuln`, the kit evals or the
migration smoke; run those explicitly when the change touches behavior,
dependencies, the kit or the schema. CI runs `make ci`, then `make accept`,
then `pnpm audit`.

### 9.3 Plan template

`docs/plans/<slug>.md`, six sections in order: problem, constraints, design,
files-to-touch, risks, test plan. Files-to-touch lists real paths under
`src/features/<slug>/`, `src/shared/`, `migrations/`, `specs/` and
`test/acceptance/`, each marked new or modified. Risks name the invariant they
touch (INV-T3 lockfile, INV-T7 offline guard, migration pairing, ownership in
SQL, the eslint architecture groups). The plan ends at `make ci` +
`make accept`.

### 9.4 Traceability

- Issue → plan: the plan names the issue.
- Plan → diff: each PR references the plan; each step names its gate.
- Diff → release: the conventional commit subject appears in `CHANGELOG.md`.
- Every changed line traces to the request; a drive-by change is a review
  finding, not a bonus.

### 9.5 Changing a plan mid-flight

Update the plan file in the same PR that changes direction. Do not leave a
stale plan next to a newer diff; the next session will follow the stale one.

---

## 10. Process plane II: test-driven development

### 10.1 Cycle and role separation

1. **RED** — write the failing test at the layer that owns the rule. A domain
   rule gets a vitest case next to the slice's `usecase.ts`; a route posture
   gets three cucumber cases: unauthenticated (401), wrong role (403),
   authorized (2xx). Observe it fail for the right reason.
2. **GREEN** — the minimum implementation that passes without breaking a
   sibling test.
3. **REFACTOR** — vitest is the safety net; no behavior change.
4. **OUTER LOOP** — adapter SQL has no compile-time check against the schema.
   After a `postgres.ts` change, run the migration and exercise the query
   against a real database before calling it done (`verify-template.sh` does
   this for the template; add a lane for the product).

Role split: `test-author` writes the test, `tdd-guide` drives the cycle,
`code-reviewer` checks the gate, `migration-reviewer` checks schema drift,
`ownership-reviewer` checks ownership and rank.

### 10.2 Runner

```sh
pnpm test                                             # all unit tests
pnpm vitest run src/features/widgets/usecase.test.ts  # one file
pnpm vitest run -t 'refuses a wrong code'             # one case
pnpm accept                                           # cucumber acceptance
node --test evals/contracts.test.mjs                  # kit contract tests
```

Unit tests use in-package fakes that implement the slice's `ports.ts`. They
never need `DATABASE_URL`, never open a socket, and never import a real
adapter. Acceptance builds the real Fastify app in `test/acceptance/app.ts`
with fake ports and drives it over an ephemeral port. Neither tier proves the
SQL; see `PLAYBOOK_TESTS.md` §7.

### 10.3 Exceptions

Skip a test only with a tracked issue and a comment naming the reason. A
deleted failing test is masking; a `.skip` with no issue is debt. A comment
that disables a lint rule is not a test exception; fix the code.

### 10.4 Quality complements

- `make gen-check` catches committed OpenAPI models that drifted from
  `specs/openapi.yaml`.
- `make vuln` catches advisories; it is weaker while the template has no
  lockfile (INV-T3) — commit a lockfile after rename if reproducibility
  matters.
- The kit contract evals catch a broken agent/skill/command schema.
- Coverage has no wired tool: the 80% floor in `tdd-guide` is a policy, not a
  measured gate. Add `vitest --coverage` (or `c8`) before claiming coverage
  evidence in an audit.
- Nothing runs real SQL in CI. `verify-template.sh` migrates up/down/up and
  `evals/run-live.sh` smokes a running stack, but neither is a repeatable
  integration gate.

---

## 11. Autodetection, registry and kit CI

### 11.1 What Claude Code already does

- Discovers `.claude/agents/*.md` and `.claude/skills/*/SKILL.md` by walking
  the tree. No registry file to edit.
- Loads `CLAUDE.md` / `AGENTS.md` from the working directory upward.
- Applies `.claude/settings.json` hooks for the project.
- Exposes commands as `/slug` when `.claude/commands/<slug>.md` exists.

### 11.2 Validators

| Script                                 | Checks                                                                                           |
| -------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `scripts/lint-agents.sh [root]`        | frontmatter fields, body sections, division enum, directory==division, CRLF, minimum body length |
| `scripts/lint-skills.sh [file ...]`    | same for skills; parses `skills.schema.md` at run time; strict placeholder mode                  |
| `scripts/lint-commands.sh [file ...]`  | same for commands; requires the slug to match a skill; strict placeholder mode                   |
| `scripts/leak-guard.sh`                | residual `<app>`/`<APP>`/`<scope>` placeholders and secret patterns                              |
| `scripts/verify-template.sh`           | staged rename + full gate + docker build + throwaway Postgres migrate up/down/up + kit evals     |
| `node --test evals/contracts.test.mjs` | 14 contract tests over valid/invalid fixtures                                                    |

Template mode: the skeleton deliberately contains literal placeholders.
`lint-skills.sh` and `lint-commands.sh` reject them by default, so set
`ALLOW_PLACEHOLDERS=1` when linting the template itself. `lint-agents.sh`
does not scan placeholders; post-rename, `leak-guard.sh` is what proves zero
residual placeholders.

`verify-template.sh` runs the three linters and `leak-guard.sh` against the
staged copy but treats their exit status as advisory (`|| true`). The checks
that fail the run are the app gates, the migration smoke, the OpenAPI shape
greps, `PreToolUse` wiring, `bash -n` on hooks, the kit evals, and the
`run-live.sh` no-op guard. Treat the advisory linters as must-pass anyway; a
warning nobody reads is how the kit rots.

### 11.3 Adding a new agent or skill

1. Copy an existing file; keep the exact frontmatter shape and body sections.
2. Pick the division: `backend` (slice/HTTP/schema), `security` (import law,
   secrets, supply chain, offline guard), `testing` (tests), `core`
   (everything else).
3. Write one sentence for `description`; action-first, no filler.
4. Run `bash scripts/lint-agents.sh` (or the skill/command linter).
5. Add the command shim if it needs a `/slug`.
6. Update the roster table in this playbook and in `AGENTS.md` if the list
   changed.
7. Run `node --test evals/contracts.test.mjs`.

### 11.4 Evals

- **Offline (tier 1, always on):** `evals/contracts.test.mjs` validates a
  document against hardcoded requirements and runs the three valid fixtures,
  the four invalid fixtures and the slice-import lint fixtures. Zero
  dependencies (INV-T16). It runs in `verify-template.sh`, not in `ci.yml`.
- **Live (tier 2, manual):** `EVALS=1 bash evals/run-live.sh` drives the
  `review` skill against a sample diff and asserts the output contains
  `BLOCK`/`WARN`/`APPROVE`/`Verdict`. Requires the `claude` CLI and an API
  key; exits 0 on skip.
- **Live judge (tier 3, manual):** `EVALS=1 TIER=3` adds an LLM-judge pass
  over a canned output, expecting `PASS score=<n>` or `FAIL score=<n>`.
- No workflow wires the live tiers. They are operator commands by design —
  never on push or PR, and never with a committed API key.

The validator hardcodes the schema requirements. When a schema doc changes,
update both the doc and the static arrays in `evals/contracts.test.mjs` — the
shell linters read the doc, the native evals do not.

### 11.5 Diagnosing "the agent isn't using it"

1. Is the file in the right directory and does its frontmatter parse?
   Run the linter.
2. Is the `description` phrased as the task a user would type? A description
   that reads like a title is not discovered.
3. Is the command slug identical to the skill slug?
4. Is the session's working directory the repo root (or a descendant)?
5. Is the skill too large to load cheaply? Split it.
6. Is the instruction file too long, so the important rule is buried? Cut it.

---

## 12. Switches and profiles

### 12.1 Kit switches

| Switch                | Values             | Effect                              |
| --------------------- | ------------------ | ----------------------------------- |
| `ALLOW_PLACEHOLDERS`  | `0` (default), `1` | template-mode linting before rename |
| `EVALS`               | `0` (default), `1` | enables the live eval script        |
| `TIER`                | `2` (default), `3` | live eval depth                     |
| `--targets=a,b`       | runtime slugs      | installer target set                |
| `--dest=DIR`          | path               | installer destination               |
| `--placeholders-only` | flag               | leak-guard scans placeholders only  |

### 12.2 Profiles

| Profile   | Settings                                                |
| --------- | ------------------------------------------------------- |
| Design    | `plan` mode, no edits, `/spec` + `/plan`                |
| Implement | `acceptEdits` on an owned worktree, slice-scoped prompt |
| Review    | read-only, `/review` + `code-reviewer`, no writes       |
| Release   | human-driven, `/ship`, clean tree, no force-push        |

### 12.3 Native switches to know

- `/model` — escalate only for design or a repeated failure; return to the
  workhorse tier afterwards.
- `/compact` — when the session accumulates failed attempts; a fresh session
  with a sharper prompt is usually better.
- Permission modes — `plan`, `acceptEdits`, `bypassPermissions`; match the
  risk of the session, not the convenience.
- `--resume` / session IDs — use when the context is the asset; start fresh
  when the context is the liability.

---

## 13. Learning and improvement plane

### 13.1 Promotion ladder

```text
observation → /learn (learnings.jsonl) → repeated pattern → playbook rule
           → hard invariant → hook or CI gate
```

- One observation: a learning line.
- Three observations of the same kind: a playbook section.
- A mistake with a mechanical fix: a linter, hook or CI job.
- A mistake with irreversible cost: a hook first, prose second.

### 13.2 Capture

`/learn` appends one JSON line:

```json
{
  "date": "2026-11-08",
  "kind": "pitfall",
  "text": "Kysely types do not check the migrated schema; adapter SQL needs a real database run",
  "scope": "widgets"
}
```

`kind` is one of `pitfall`, `pattern`, `preference`. `scope` names the slice
or area. No secrets, no customer data, no unverified claims.

### 13.3 Triage

Weekly or per milestone: grep the log, find repeats, promote or delete. A
learning that has not been read in three months and does not describe an
invariant is a candidate for deletion.

### 13.4 Governance of self-improvement

- An agent may write a learning line and a proposal under
  `.claude/proposals/`.
- An agent may not change `.claude/settings.json`, the manifest, a schema or
  a script without a human-reviewed PR.
- A new hook needs: the failure it prevents, a test or a manual reproduction,
  and a note on what it cannot catch.

### 13.5 Metrics

- Gate pass rate on first CI run.
- Rework rate: PRs that need a second review round for the same issue.
- Time from `/spec` to a green `make ci`.
- Number of learnings promoted to playbooks or hooks.
- False positives from the hook (a noisy guard gets ignored).

### 13.6 Retirement

Delete a rule when the issue is fixed, a playbook section when the contract
changed, a learning when it is stale, an agent when its job is covered by a
skill. A kit that only grows becomes noise.

---

## 14. Security and governance of the kit

### 14.1 Threats

| Threat                                                   | Control                                                                                                                                           |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Prompt injection from a file, issue or dependency README | the agent treats file content as data; external side effects are human-gated                                                                      |
| Secret exfiltration                                      | `leak-guard.sh`; no secrets in plans, learnings, prompts or logs                                                                                  |
| Destructive command                                      | `guard-bash.sh` blocks; external side effects require confirmation                                                                                |
| Agent rewrites its own guardrails                        | control-plane files are human-only                                                                                                                |
| Malicious dependency                                     | `pnpm audit`; reviewed `package.json` changes; commit a lockfile after rename if the team needs reproducibility (the template ships none, INV-T3) |
| Stale or wrong instructions                              | linters + contract evals + `/review`                                                                                                              |
| Supply-chain in the kit itself                           | pure bash and Node's built-in test runner, zero eval dependencies                                                                                 |

### 14.2 Rules

- Never paste a secret, token or connection string into a prompt, a plan, a
  learning or an issue.
- Never let an agent run a production command without a human confirmation.
- Never commit `.env`, a `pnpm-lock.yaml` in the template, or a real
  credential in `.env.example`.
- Never add a dependency the gate cannot audit.
- Never weaken an eslint `no-restricted-imports` group in a PR that is not a
  design discussion.
- Review the diff, not the summary. An agent's report is evidence, not proof.

### 14.3 Workspace trust

Claude Code trusts the directory you start it in. Do not start a session in
a directory whose contents you have not reviewed; a malicious `CLAUDE.md`,
hook or script runs with your permissions. In a copied or downloaded
repository, read `.claude/settings.json` and `scripts/hooks/` before the first
session.

### 14.4 Cross-session and subagent boundaries

- A subagent inherits the session's permissions; it does not gain new rights.
- A subagent's report is a summary, not a verified fact; the parent verifies
  a destructive or external claim before acting.
- Cross-session messages are coordination, not authorization.

### 14.5 Organization layer

At organization scale, the managed policy layer can pin permission rules and
deny hooks across repositories. It is outside this repo; document the
organization's policy path in `AGENTS.md` when it exists.

---

## 15. CI/CD and compliance integration

### 15.1 CI checks

`.github/workflows/ci.yml` has one job, `gates`, with these steps:

| Step              | Command                                                 |
| ----------------- | ------------------------------------------------------- |
| Gate              | `make ci` (fmt-check + vet + lint + gen-check + vitest) |
| Unit + acceptance | `make test`, then `make accept`                         |
| Audit             | `pnpm audit --audit-level=high`                         |
| Workflow lint     | `raven-actions/actionlint`                              |

The kit's own validators and contract evals run in `verify-template.sh`, not
in `ci.yml`. Add a kit job when the kit changes often enough to justify it;
until then, run `node --test evals/contracts.test.mjs` by hand on every
`.claude/**` change.

Node 24 and pnpm 10 are pinned by `packageManager`; CI uses
`pnpm/action-setup` + `actions/setup-node` with pnpm caching. The template
ships no lockfile, so the CI install resolves latest-for-range until one is
committed after rename.

### 15.2 Using Claude Code in CI

Use it for review or triage, never for an unreviewed write to `main`:

- `claude -p` for a review pass on a PR diff, with a read-only token.
- An API key in a secret, never in the workflow file.
- A fork PR gets no secret and must skip cleanly.
- The live eval workflow is the model: `workflow_dispatch`, secret-gated,
  fail-closed on a missing key.

### 15.3 Audit evidence mapping

| Control area          | Evidence                                                                  |
| --------------------- | ------------------------------------------------------------------------- |
| Change management     | PR review, plan, conventional commit, `CHANGELOG.md`                      |
| Code quality          | CI `make ci` output (prettier, tsc, eslint, vitest)                       |
| Access control        | ownership map, control-plane human-only rule                              |
| Secure SDLC           | `PLAYBOOK_SECURITY.md`, `pnpm audit` output                               |
| Testing               | vitest + cucumber results, contract evals, acceptance summary             |
| Dependency management | `package.json` review, `pnpm audit`, lockfile decision                    |
| Monitoring            | request id, Pino logs, `/healthz` + `/readyz`, `PLAYBOOK_SECURITY.md` §12 |
| Kit integrity         | linter output, leak-guard output, `verify-template.sh` log                |

Test names carry `@SOC2-CC8.1`-style tags where the evidence is needed; see
`PLAYBOOK_TESTS.md` §11.

---

## 16. Operating cadences

| Cadence       | Action                                                                              |
| ------------- | ----------------------------------------------------------------------------------- |
| Every change  | `/spec` if nontrivial → `/plan` → red/green → `/review` → `make ci` + `make accept` |
| Every PR      | review the diff, not the summary; confirm the gate output                           |
| Weekly        | triage `learnings.jsonl`; delete stale rules                                        |
| Per milestone | promote repeated learnings; run `make vuln`; review open gaps                       |
| Per release   | `/ship`; verify migrations up/down on a fresh database; record evidence             |
| Quarterly     | prune the agent roster and the playbook; re-read this file                          |

---

## 17. Adoption roadmap and maturity levels

| Level          | State                                                 | What exists                           |
| -------------- | ----------------------------------------------------- | ------------------------------------- |
| 0 — ad hoc     | prompts in a chat                                     | nothing committed                     |
| 1 — instructed | `AGENTS.md` + `CLAUDE.md` symlink                     | rules every session sees              |
| 2 — capable    | agents, skills, commands, schemas, linters            | repeatable procedures, validated kit  |
| 3 — controlled | hooks, ownership map, contract evals, CI              | mechanical guardrails, tested harness |
| 4 — scaled     | worktrees, claims, cross-session protocol, live evals | many sessions, measured quality       |
| 5 — governed   | audit evidence, org policy, retirement cadence        | auditable program                     |

This kit ships levels 1–3 in full and the conventions for level 4. Level 5
depends on the organization, not on the repository.

---

## 18. Troubleshooting

| Symptom                                    | Likely cause                                                       | Fix                                                                                        |
| ------------------------------------------ | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| Agent not discovered                       | wrong directory, bad frontmatter                                   | run `lint-agents.sh`; check the four fields                                                |
| Skill not invoked                          | description reads like a title, or slug mismatch                   | rewrite the description as the user's task; check the command shim                         |
| Hook never fires                           | `settings.json` not loaded, wrong path, non-Bash tool              | confirm the working directory, the `PreToolUse` matcher and `CLAUDE_PROJECT_DIR`           |
| Linter fails on a clean template           | literal placeholders by design in skills/commands                  | `ALLOW_PLACEHOLDERS=1`; strict after rename                                                |
| `verify-template.sh` fails on placeholders | a placeholder survived rename                                      | run `rename.sh` again; run `leak-guard.sh --placeholders-only`                             |
| Build fails with `<` in a package name     | template used before rename                                        | instantiate first; never build the raw skeleton                                            |
| eslint architecture rule fires             | a slice reached across a boundary                                  | move the shared code to `#shared`/`src/platform`, or expose a port; never disable the rule |
| `tsc` cannot resolve `#shared/*`           | wrong relative depth or a missing `package.json` `"imports"` entry | use the subpath import; keep own-slice imports relative                                    |
| Kysely type error after a migration        | adapter types drifted from schema                                  | update `queries.ts`/`postgres.ts` types; re-run the migration smoke                        |
| `make gen-check` fails                     | `src/gen/api` stale                                                | `make generate` and commit                                                                 |
| `pnpm audit` fails                         | a high advisory in the range                                       | pin or replace the dependency; a lockfile decision may be needed                           |
| Agent ignores a rule                       | file too long, rule buried                                         | move it to a playbook or a hook                                                            |
| Live eval fails immediately                | no `claude` CLI or API key                                         | expected skip; set both and `EVALS=1`                                                      |

---

## 19. Kit acceptance checklist

### Instruction plane

- [ ] `AGENTS.md` exists and is under ~200 lines.
- [ ] `CLAUDE.md` is a symlink, not a copy.
- [ ] Every rule traces to a real mistake or an invariant.

### Memory plane

- [ ] `MEMORY.md` states the premise, actors, slice→table map and gates.
- [ ] No invented table, route or command.
- [ ] `.claude/learnings.jsonl` is append-only and contains no secrets.

### Capability plane

- [ ] 11 agents, each with four frontmatter fields and five body sections.
- [ ] 9 skills, each with the required sections.
- [ ] 9 commands, each a `## Delegate` shim to the matching skill.
- [ ] Roster tables in this playbook and `AGENTS.md` agree.

### Control plane

- [ ] `.claude/settings.json` wires the `PreToolUse` Bash hook.
- [ ] `guard-bash.sh` is `bash -n` clean and rename-inert.
- [ ] Control-plane files are human-only.
- [ ] Ownership map names every shared area.

### Coordination plane

- [ ] Worktree-per-slice convention documented.
- [ ] Contract-change protocol documented.
- [ ] No two sessions own the same slice.

### Process plane

- [ ] `/spec` → `/plan` → red/green → `/review` → `make ci` documented.
- [ ] Plan template has all six sections.
- [ ] `make ci`, `make accept` and `make vuln` names are correct.

### Learning and governance

- [ ] `/learn` format documented.
- [ ] Promotion ladder and retirement rule documented.
- [ ] Kit changes require a human-reviewed PR.
- [ ] Audit evidence mapping points at real commands.

### Verification

- [ ] `ALLOW_PLACEHOLDERS=1 bash scripts/lint-skills.sh` passes.
- [ ] `ALLOW_PLACEHOLDERS=1 bash scripts/lint-commands.sh` passes.
- [ ] `bash scripts/lint-agents.sh` passes.
- [ ] `node --test evals/contracts.test.mjs` passes (14 tests).
- [ ] `bash scripts/leak-guard.sh --placeholders-only` passes.
- [ ] `bash scripts/verify-template.sh` passes after rename.

---

## Appendix A — Reference templates

### A.1 `AGENTS.md` skeleton

```markdown
# <app> — agent instructions

## Commands

- `make ci` — prettier check + tsc + eslint + gen-check + vitest
- `make accept` — cucumber over the fake-wired app
- `make migrate-up`, `make gen-check`, `make vuln`

## Layout

- `src/{bin,features,platform,shared,gen}`, `migrations/`, `specs/`,
  `test/acceptance/`, `evals/`, `.claude/`, `scripts/`

## Hard rules

- English only.
- `make ci` green before done.
- The eslint `no-restricted-imports` groups are the architecture law.
- No unexplained suppressions; never fabricate paths or results.
- Touch only what the request needs.
- INV-T3: no lockfile in the template. INV-T7: offline guard.

## Workflow

`/spec` → `/plan` → red → green → `/review` → `make ci`.

## Who edits what

- Control plane: human only.
- Shared modules: integrator.
- Slice: its implementer.

## Multi-session

One worktree per (slice, spec); message the owner, do not edit a claimed slice.

## Never

- Force-push a shared branch.
- Commit `.env` or a real credential.
- Delete a failing test to make the gate green.
```

### A.2 Agent template

```markdown
---
name: "<app> <Role>"
description: "<one sentence: what it does, for which files>"
color: "#RRGGBB"
division: <backend|security|testing|core>
---

# <app> <Role>

## Core Mission

<the one job, in one paragraph>

## Expertise Areas

- <area>: <what it knows>

## Output Format

<the exact shape of the report, with a short example>

## Quality Standards

- <a checkable standard>

## Guiding Principles

1. <a decision rule>
```

### A.3 Skill template

```markdown
---
name: "<slug>"
description: "<one sentence>"
division: <backend|security|testing|core>
---

# <app> <slug> skill

## When to Activate

<the request that should trigger it; what should not>

## Core Principles

- <the fixed procedure>

## Rules

- <a hard rule and why>

## Research References

| Claim | Source | Year | Link |
| ----- | ------ | ---- | ---- |

## Output Format

<the exact shape>
```

### A.4 Rule template — not shipped

If a rules directory is added later:

```markdown
# <area> rules

- <rule> — <reason>
```

Keep each file under 50 lines and scoped to a path.

### A.5 Hook contract (language-agnostic pseudocode)

```text
read JSON from stdin
if tool_name != "Bash": exit 0
extract command (best effort)
if command matches a BLOCK pattern: reason to stderr; exit 2
if command matches a WARN pattern: note to stderr; exit 0
exit 0
```

Fail open on any parse error. A hook that wedges normal tool use is worse
than no hook.

### A.6 `settings.json` skeleton

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash \"$CLAUDE_PROJECT_DIR/scripts/hooks/guard-bash.sh\""
          }
        ]
      }
    ]
  }
}
```

### A.7 Lesson entry format

```json
{
  "date": "YYYY-MM-DD",
  "kind": "pitfall|pattern|preference",
  "text": "...",
  "scope": "<slice|area>"
}
```

### A.8 Proposal format

```markdown
# Proposal: <title>

## Failure it prevents

## Proposed control (hook / lint / eval / rule)

## What it cannot catch

## Verification

## Owner
```

### A.9 Eval dataset entry

```text
input:   a fixture document (agent/skill/command) or a sample diff
expect:  pass for valid, a known error substring for invalid
runner:  node --test evals/contracts.test.mjs (offline, zero deps)
```

---

## Appendix B — How this kit maps to the template

| Template concern       | Kit artifact                                                 |
| ---------------------- | ------------------------------------------------------------ |
| Slice layout (8 files) | `AGENTS.md` layout section + `/add-module`                   |
| Dependency direction   | `AGENTS.md` table + `architect`, `depguard-reviewer`         |
| Handler shape          | `/add-handler` + `api-designer`                              |
| Ports and adapters     | `PLAYBOOK_BACKEND.md` §3 + `architect`, `ownership-reviewer` |
| Migrations             | `PLAYBOOK_BACKEND.md` §6 + `migration-reviewer`              |
| Routes and envelope    | `PLAYBOOK_URLS.md` + `api-designer`                          |
| Tests                  | `PLAYBOOK_TESTS.md` + `tdd-guide`, `test-author`             |
| Security               | `PLAYBOOK_SECURITY.md` + `depguard-reviewer`                 |
| Audit                  | `PLAYBOOK_AUDIT.md` + `/ship` evidence                       |
| Gate                   | `make ci` + CI `ci.yml`                                      |
| Kit integrity          | linters + `evals/` + `verify-template.sh`                    |

The rule that closes the loop: a change to the kit is a change to the
template. It lands through the same gate, the same review and the same
`CHANGELOG.md` as the TypeScript under `src/`.
