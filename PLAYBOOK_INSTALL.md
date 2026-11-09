# FACTA playbook — installation and bootstrap

This is the runbook for a new machine (or a re-provisioned one) and for a fresh
checkout of the working set: clone the playbook, install the user-scope kit,
clone the repositories, configure each one, verify. Every step is a shell
command, so the whole bootstrap costs zero model tokens; the model is only
asked to make judgment calls. Companions:
[PLAYBOOK_CLAUDE_CODE.md](PLAYBOOK_CLAUDE_CODE.md) (what the kit must
contain) and [CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md](CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md)
(model tiers, `~/.claude/settings.json`, verification, rollback). This file is
the install path only; it does not restate the playbooks.

---

## 0. What an install costs

| Step | Executed by | Model tokens |
| --- | --- | --- |
| Clone the playbook and the working set | shell (`clone-repos.sh`) | 0 |
| Install the user-scope kit | shell (`install-user-kit.sh`) | 0 |
| Configure each repository | shell (`configure-repo.sh`) | 0 |
| Validate and probe | shell (`kit_check.py`) + one cheap-tier probe | 0 + ≤1 cheap probe |
| Judgment calls (which kit, which group, when to escalate) | model | charged to the tier that makes them |

Target: a full bootstrap with **zero model calls**; at most one cheap-tier probe (`haiku` → `minimax-m3.1-flash:cloud`) to confirm the deny rules and the resolved tier. Escalate to `opus` only for a design decision, never for setup.

### Rules that keep it at zero

1. Scripts do the work; the model reviews the output. Never paste a playbook
   into a session — point at the section.
2. Shallow, single-branch clones: `--depth 1 --single-branch`.
3. No `@-imports` of playbooks anywhere: `CLAUDE.md` links by path, never
   `@file`.
4. The always-loaded rule bytes (rules without a `paths:` block) stay under
   20,480 B, the ceiling `kit_check.py` enforces; `CLAUDE.md` stays under 200
   lines. Scope per-stack rules with `paths:` so they load only where needed.
5. Never copy plugin components into user or project scope: plugins are
   namespaced (`/ecc:*`, `ecc:*`, `/ponytail:*`). A user copy of a plugin
   agent shadows it by name.
6. Use the cheap tier for probes and mechanical work; escalate only for a
   design decision.
7. `claude -p ... --bare` (playbook §12.3) skips hooks, skills, plugins, MCP,
   auto memory and `CLAUDE.md` — the minimal-token scripted path.

---

## 1. Prerequisites

| Tool | Minimum | Check |
| --- | --- | --- |
| git | any supported | `git --version` |
| gh | any, authenticated | `gh auth status` |
| claude | v2.1.251 or later (2.1.309+ recommended) | `claude --version` |
| ollama | ≥ 0.33 with `ollama launch claude` | `ollama --version` |
| python3 | 3.9+ (stdlib only) | `python3 --version` |
| rtk | optional (token-saving shell proxy) | `rtk --version` |

`gh auth status` must report a logged-in account with repo scope before
cloning anything private.

---

## 2. Machine bootstrap (user scope, `~/.claude`)

1. Shallow clone the playbook:

   ```bash
   git clone --depth 1 --single-branch https://github.com/Facta-Dev/facta-playbook.git ~/facta-playbook
   ```

2. Install the user-scope kit:

   ```bash
   bash ~/facta-playbook/scripts/claude-kit/install-user-kit.sh --dry-run   # preview
   bash ~/facta-playbook/scripts/claude-kit/install-user-kit.sh             # install
   bash ~/facta-playbook/scripts/claude-kit/install-user-kit.sh --force      # overwrite installer-owned files
   bash ~/facta-playbook/scripts/claude-kit/install-user-kit.sh --no-settings # leave settings.json untouched
   ```

   It installs `kit/CLAUDE.md`, `kit/agents`, `kit/skills` and `kit/templates/sdd` into `~/.claude/`, and **never overwrites user-owned files**: an existing destination is skipped, not replaced. Backups land in `~/.claude/backups/install-<UTC timestamp>/`.

   It also merges `kit/settings.fragment.json` into `~/.claude/settings.json`: the model-tier `env` (including the Ollama endpoint), the `"model": "sonnet"` default and the `permissions.deny` list. Existing keys win; the installer warns when `model` is not `sonnet`, because a bare `claude` then runs the escalation model. `--no-settings` skips the merge entirely.

3. Validate the user scope:

   ```bash
   python3 ~/facta-playbook/scripts/claude-kit/kit_check.py   # 0 clean, 1 warnings, 2 errors
   ```

   The text report ends with `agents= skills= rules= always_loaded= claude_md=
   collisions= broken_links=`. The `--json` summary exposes the same values
   under `agents`, `skills`, `rules`, `always_loaded_bytes`, `claude_md_lines`,
   `collision_warnings`. Always-loaded bytes must stay under 20,480 B.

4. Lock down the control plane:

   ```bash
   chmod 600 ~/.claude/settings.json
   ```

5. Optional dedupe (remove user copies of plugin components):

   ```bash
   python3 ~/facta-playbook/scripts/claude-kit/dedupe_plugins.py            # dry run
   python3 ~/facta-playbook/scripts/claude-kit/dedupe_plugins.py --apply
   ```

   The dry run lists what would move into `~/.claude/backups/dedupe-<UTC timestamp>/`.
   Nothing is deleted; `--restore` puts it back.

---

## 3. Clone the working set

```bash
bash ~/facta-playbook/scripts/claude-kit/clone-repos.sh --group=all \
  --dir ~/work --depth 1 --dry-run
```

| Flag | Meaning |
| --- | --- |
| `--group=` | `core`, `products`, `tools`, `clients`, `reference`, `all` (default `all`) |
| `--dir=` | destination root (default `~/work`) |
| `--depth=` | clone depth (default `1`) |
| `--dry-run` | print the plan and the exact commands, clone nothing |

Both `--flag=value` and `--flag value` are accepted.

The inventory is `install/repos.txt` — one `group|org/repo|why` per line,
derived from README §4. It is the single source of truth: add a repo there and
in the README table, never in the script.

| Group | Repos (README §4) |
| --- | --- |
| `core` | `facta-playbook` (this repo) |
| `products` | §4.1 + §4.2: `talos`, `deino`, `staxis`, `v0ss`, `enyo-platform`, `heimdall-platform`, `trustanova`, `ReachPilot`, `ReachPilotPro`, `package_builder`, `ei2`, the `zypho-*` suite, `nest`, `ctx0`, `facta_archetype`, `facta_frontend`, `blog-templates`, `3d-landings-pro`, `claude-skill`, `ai-job-search`, `agents-office`, `synthia_frontend` |
| `tools` | `aws-ec2-setup`, `claude-code-cli-cost-tracker`, `terrain`, `OmniRoute`, `repomix` |
| `clients` | `everythink_platform`, `everythiink_arxiv`, `everythink_android`, `everythink_ios`, `everythink_backend`, `transportation-bank`, `technorides_eu`, `technorides_web`, `technorides_web_premium`, `trustanova-platform` |
| `reference` | skill and factory packs (§4.4) plus the upstream read-only clones (§4.6) |
| `all` | every group above |

`Facta-Dev/` owns the products and tooling; `contextzero/` owns `nest` and
`ctx0`; `everythink-ai`, `Technorides` and `matiasbaglieri` own the client
rebuilds. Shallow clones are for consumption. Use full history (`--depth 0`)
only in the repositories you actually contribute to.

---

## 4. Configure a repository

```bash
bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh ~/work/talos \
  --kit=golang --dry-run
bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh ~/work/talos --kit=golang
```

`--kit=` is one of `golang`, `nodejs`, `python`, `rust`.

The script is idempotent and does exactly this:

- appends the `<!-- model-routing:ollama -->` block to the repo's `CLAUDE.md` **only when the marker is absent** (text: `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md §6`);
- writes `.claude/README.md` pointing at the playbooks instead of restating them;
- merges **only** `permissions.deny` into the repo's `.claude/settings.json` (never `allow`, never hooks, never `env`);
- appends `.gitignore` entries: `CLAUDE.local.md`, `.claude/settings.local.json`, `.claude/install-state.json`, `.claude/backups/`, `.claude/logs/`;
- with `--kit`, runs that kit's own `scripts/install-claude-runtime.sh`.

`--check` verifies the repo without writing anything; `--dry-run` prints the diff it would apply.

Per-script flags and the no-delete contract: [scripts/claude-kit/README.md](scripts/claude-kit/README.md).

### 4.1 Repositories that install something

Cloning is not installing. A few working-set repos ship tooling that must be
installed once per machine, after `clone-repos.sh` and before you rely on it.
Everything here is idempotent.

| Repo | What to run | What it does |
| --- | --- | --- |
| `claude-code-cli-cost-tracker` | `bash ~/work/claude-code-cli-cost-tracker/install.sh` | Runs its test suite, copies `cccost/`, `statusline.py`, `cccost.py`, `prices.json`, `commands/cccost.md` and `hooks/session_status.py` into `$CLAUDE_CONFIG_DIR` (default `~/.claude`), then prints the `statusLine` and per-event hook JSON to merge into `settings.json`. It never edits your settings. On Linux it also installs `ccpanel` to `~/.local/bin` and registers the Terminator layout |
| `Facta-Dev/skills` | `/plugin marketplace add Facta-Dev/skills` → `/plugin install mattpocock-skills@mattpocock` | Engineering skill pack: grilling, spec/ticket flows, TDD, review |
| `Facta-Dev/beautifulgo` | `/plugin marketplace add Facta-Dev/beautifulgo` → `/plugin install beautiful-go@beautiful-go` | Idiomatic-Go skills |
| `Facta-Dev/beautifulnodejs` | `/plugin marketplace add Facta-Dev/beautifulnodejs` → `/plugin install beautiful-nodejs@beautiful-nodejs` | Idiomatic Node/TS skills |
| `Facta-Dev/beautifulpython` | `/plugin marketplace add Facta-Dev/beautifulpython` → `/plugin install beautiful-python@beautiful-python` | Idiomatic-Python skills |
| `Facta-Dev/beautifulrust` | `/plugin marketplace add Facta-Dev/beautifulrust` → `/plugin install beautiful-rust@beautiful-rust` | Idiomatic-Rust skills |
| `Facta-Dev/garden-skills` | `/plugin marketplace add Facta-Dev/garden-skills` → `/plugin install <plugin>@garden-skills` | Six plugins: `presentation-skills`, `web-design-skills`, `knowledge-base-skills`, `image-generation-skills`, `motion-showreel-skills`, `beautiful-article-skills` |
| `Facta-Dev/superpowers` | `/plugin marketplace add Facta-Dev/superpowers` → `/plugin install superpowers@superpowers-dev` | Vendored skill collection |
| `Facta-Dev/archify`, other packs | same two-step pattern when the clone has `.claude-plugin/marketplace.json` | Plugin registration; no copy into `~/.claude/skills` |

The marketplace name comes from the repo's `.claude-plugin/marketplace.json`,
not from the repository name (`Facta-Dev/beautifulgo` → `beautiful-go`);
`/plugin marketplace add` prints the name it registered.

Rules:

- **Plugins are registered, never copied.** `~/.claude/skills/<pack>/` would
  shadow a namespaced plugin skill and bill its description every turn; the
  marketplace entry plus `enabledPlugins` is the only install path.
- The non-interactive equivalent of the two slash commands is the pair
  `extraKnownMarketplaces.<name>.source = {"source": "github", "repo": "<org>/<repo>"}`
  and `enabledPlugins["<plugin>@<marketplace>"] = true` in `~/.claude/settings.json`
  (the shape already used for `ecc@ecc` and `ponytail@ponytail`). Keep the
  edits human: `settings.json` is the control plane.
- A repo without an `install.sh` and without `.claude-plugin/` is consumed as
  source: clone, `configure-repo.sh`, done. That is the default for products.
- After installing tooling, re-run `kit_check.py` (it reports user skills and
  command collisions) and `/cccost` to confirm the tracker is wired.

---

## 5. Verify the install

All zero-token, except one cheap-tier probe.

```bash
# 1. user scope: 0 clean / 1 warnings / 2 errors
python3 ~/facta-playbook/scripts/claude-kit/kit_check.py; echo "exit=$?"

# 2. per repo: nothing to write, nothing missing
bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh ~/work/talos --check

# 3. always-loaded bytes, inside a session
#    /context        -> reports the always-loaded footprint
# 4. session health, inside a session
#    /doctor
```

Deny probes (one session, cheap tier — the only model call in the install):

```bash
claude -p --model haiku "Attempt to read the file .env in this repo and then run 'git push --force'. Report only whether each was blocked by a permission denial." --output-format json
```

Expected: the `Read` of `.env` and the `git push --force` are denied by the `permissions.deny` list.

Accepted gap: the deny list covers `Read`, so `cat .env` through Bash is **not** denied — deny `Bash(cat .env*)` explicitly if that matters. `defaultMode: bypassPermissions` enforces **deny** but not **allow**: an `allow` list is inert on that machine.

---

## 6. What actually costs tokens

| Problem | Cost | Do instead |
| --- | --- | --- |
| Reading a whole playbook into a session | thousands of tokens, every turn | Point at the section; `Read` the range you need |
| `/init` on a large repository | a full-tree summary it re-reads next session | Run the kit's own scripts; let `configure-repo.sh` write the skeleton |
| Duplicating plugin agents/commands into `~/.claude` | duplicate descriptions billed every turn, plus name shadowing | Keep plugins namespaced; run `dedupe_plugins.py` |
| Broad recursive searches | a search agent re-reads files already summarized | Search a known path, or narrow with `paths:`-scoped rules |
| Fanning out subagents for mechanical work | one call per task | Do it in the main session; subagents only for judgment |
| Re-reading the same plan in the advisor | repeated opus calls | One `advisor` call with concrete context, before locking the plan |

---

## 7. Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| Clone fails with 403/404 | not authenticated or missing repo scope | `gh auth status`; `gh auth login`; add the repo to the org |
| Clone fails with `repository not found` | wrong org prefix | Products and tooling are `Facta-Dev/`, clients are `everythink-ai/` / `Technorides/` / `matiasbaglieri/` |
| `install-user-kit.sh` refuses to overwrite | destination is user-owned | That is the contract; use `--force` only for installer-owned files. A `settings.json` merge conflict is resolved from the backup at `~/.claude/backups/install-<UTC timestamp>/settings.json`; only `permissions.deny` is merged |
| `kit_check.py` exits 2 | findings with level `error` | Read the `area` field: `agents`, `skills`, `rules`, `claude-md`, `settings`, `commands`, `links` |
| Deny probe passes through | project-relative pattern at user scope | User-scope denies must anchor `//` or `~/` (`Read(//**/.env)`), never `./.env` |
| Deny probe passes through | expecting `allow` to block | Under `bypassPermissions`, `deny` is enforced and `allow` is inert |
| Skill not discovered | wrong layout | A skill is `skills/<name>/SKILL.md`; a flat `.md` is ignored |
| Rule not loading | no `paths:` frontmatter | Unscoped rules load always (budget: 20 KB total); scoped rules need `paths:` and only `.md` is read |
| `collision_warnings` > 0 | a user file duplicates a plugin command | `dedupe_plugins.py --apply` |

---

## 8. Rollback

| Action | Command |
| --- | --- |
| Restore the user scope | `~/.claude/backups/install-<UTC timestamp>/` holds the pre-install `settings.json` and kit files — copy back what you need |
| Undo a dedupe | `python3 ~/facta-playbook/scripts/claude-kit/dedupe_plugins.py --restore` |
| Undo a repo configuration | `bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh <path> --dry-run` to review before writing; the routing block is marker-guarded, so remove the `<!-- model-routing:ollama -->` section by hand |
| Drop a clone | `rm -rf ~/work/<repo>` — shallow clones hold no local work |

`install-user-kit.sh` never deletes: it skips user-owned destinations and
keeps backups, so a rollback is a copy, not an undo.
