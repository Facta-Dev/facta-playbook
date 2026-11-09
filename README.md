# FACTA Playbook — how we work

This repo is the operating contract: **which plugins to run, which playbook
governs each kind of implementation, and which repositories make up the
working set.** Product-facing conventions (identity, launches, copy) stay in
their own repos; this one is about how the work gets done.

The playbooks were written for **Heimdall** and are language-agnostic —
Heimdall is the reference implementation; every other project adopts them by
adapting the stack profiles, not the rules.

---

## 0. Start here

A new machine is three shell steps. The install spends **no model tokens** — it
is shell only; the model is consulted later, for judgment calls only.

1. **Clone** the playbook:
   `git clone --depth 1 https://github.com/Facta-Dev/facta-playbook.git ~/facta-playbook`
2. **Bootstrap** the user-scope kit (`~/.claude`):
   `bash ~/facta-playbook/scripts/claude-kit/install-user-kit.sh` (`--dry-run` to preview)
3. **Configure** the repos you need:
   `bash ~/facta-playbook/scripts/claude-kit/clone-repos.sh --dir ~/work`
   clones the install set (`selected`: the four language skill packs, the cost
   tracker and the skill packs), then
   `bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh ~/work/<repo> --kit=<lang>`
   wires each one. `--group=selected|core|products|tools|clients|reference|all`
   narrows or widens the set (`all` is the whole map in §4). Skill packs and the
   cost tracker ship their own installers —
   see [PLAYBOOK_INSTALL.md §4.1](PLAYBOOK_INSTALL.md#41-repositories-that-install-something).
4. **Install tooling**: `bash ~/facta-playbook/scripts/claude-kit/install-skill-packs.sh --dir ~/work`
   registers the skill-pack marketplaces and installs their plugins; the cost
   tracker installs with `bash ~/work/claude-code-cli-cost-tracker/install.sh`.

Flags, groups, verification and rollback: [PLAYBOOK_INSTALL.md](PLAYBOOK_INSTALL.md).

## 1. Working model

- **Spec-driven + test-driven.** Nothing ships without an approved spec; the
  gate is fail-closed (`package_builder` at `pb.facta.dev` instantiates the
  scaffold, the spec gate decides whether it may merge).
- **Architecture is a contract, not a preference.** Backend = hexagonal
  (ports & adapters) organized by vertical slices; frontend = MVVM with a
  shared SDK across web, mobile and extension; routes follow the URL contract
  where the *user-type segment never authorizes*.
- **Tests are audit evidence.** The suite is designed so the same artifacts
  serve SOC 2 / ISO 27001 control evidence — not a separate compliance
  exercise bolted on later.
- **Several `claude` sessions run in parallel**, so every repo carries the
  Claude Code implementation kit: rules, agents, skills, hooks, permissions,
  memory and the collaboration protocol (`PLAYBOOK_CLAUDE_CODE.md`).
- **Conventional commits, English code and comments**, traceability over a
  clean tree: everything the work produced is committed (assets, prompts,
  logs, costs).
- **Cheap model writes, cheap model digs, strong model decides.** The tiering
  and the `advisor` agent are specified in
  `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md`; every active repo carries the
  matching `model-routing:ollama` block in its `CLAUDE.md`.

## 2. Plugins

Baseline, enabled globally in `~/.claude/settings.json` — every machine:

| Plugin | Marketplace | What it gives us |
| --- | --- | --- |
| `ecc@ecc` | `affaan-m/ECC` | Agent/skill/workflow pack: review, planning, build and audit workflows |
| `ponytail@ponytail` | `DietrichGebert/ponytail` | Lazy-senior-dev discipline: smallest diff, stdlib first, no speculative abstraction |
| `rust-analyzer-lsp@claude-plugins-official` | official | Rust language server (`v0ss`, `ReachPilotPro`) |
| `claude-model-router-hook@claude-model-router-hook` | — | Routes each request to the tier it deserves |
| `nanobanana@creding-plugins` | — | Image generation (`nanobanana:generate-image`) |

Plugin components are namespaced (`/ecc:*` commands, `ecc:*` agents). Never copy
a plugin component into `~/.claude`: it shadows the plugin agent by name and
bills its description every turn.

Stack and craft skill packs install on top of the baseline through
`scripts/claude-kit/install-skill-packs.sh`: `beautiful-go`,
`beautiful-nodejs`, `beautiful-python`, `beautiful-rust`, `mattpocock-skills`,
`motionsites-web-builder`, the six `garden-skills` plugins, `ui-ux-pro-max`
and `superpowers`. `ponytail` stays on the baseline upstream marketplace.

Project-level extras live in the project's `.claude/settings.json` — the
frontend/site repo (`facta_frontend`) is the current example: `ei2`, `pb`,
the ten `voltagent-*` agent packs, `claude-seo@agricidaniel-claude-seo` and
`github@claude-plugins-official`.

Rule: install at project level only what the baseline does not cover, and
never duplicate a baseline plugin in a project file.

## 3. Standard playbooks

| File | Governs | Use when |
| --- | --- | --- |
| `PLAYBOOK_INSTALL.md` | Installation and bootstrap: clone order, idempotent kit configuration, user-scope validation, minimal-token install path | Preparing a new machine or re-provisioning one |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code implementation kit: files, agents, skills, rules, hooks, permissions, memory, collaboration protocol, parallel sessions, SDD + TDD loop | Starting a repo, or auditing whether a repo is agent-ready |
| `PLAYBOOK_BACKEND.md` | Backend architecture: hexagonal + vertical slices, language-agnostic folder and dependency rules | Any backend service, any language |
| `PLAYBOOK_FRONTEND.md` | Frontend: React SPA, React Native, Chrome MV3, MVVM + shared SDK | Any client |
| `PLAYBOOK_URLS.md` | Route/URL contract for API and SPA routes; the user-type segment never authorizes | Designing endpoints or permissions |
| `PLAYBOOK_TESTS.md` | Testing contract, slice-aligned, doubling as audit evidence | Every implementation |
| `PLAYBOOK_SECURITY.md` | Security contract with per-language profiles (Node, Python, Go, Rust) | Every backend, before release |
| `PLAYBOOK_AUDIT.md` | SOC 2 / ISO 27001 / ISO 9001 preparation, consuming the other playbooks as controls | Certification, enterprise or regulated deals |
| `PLAYBOOK_REA.md` | REA setup: register the reverse-engineering MCP + skill with every agent on the machine | Analyzing binaries, bundles or captured traffic |
| `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md` | Model tiers (`opus`/`sonnet`/`haiku`), `advisor` agent, settings, verification, gotchas, rollback | Setting up a new machine or a new repo |

### 3.1 User-scope kit and install path

`kit/` is the canonical user-scope (`~/.claude`) layer, versioned here so a new
machine is one command away; `scripts/claude-kit/` is the install layer.

| Path | What it is |
| --- | --- |
| `kit/CLAUDE.md` | User-scope instructions (English artifacts, model tiers, kit governance) |
| `kit/agents/` | `advisor` (read-only decision advisor) and `kit-curator` (kit maintenance) |
| `kit/skills/` | The 8 SDD + audit skills: `spec-new`, `spec-clarify`, `spec-plan`, `spec-tasks`, `tdd-cycle`, `spec-verify`, `retro`, `kit-audit` |
| `kit/templates/sdd/` | Spec, plan, tasks and lessons templates |
| `kit/settings.fragment.json` | Model-routing env (local Ollama endpoint plus the fixed literal token `ollama`), default model and the deny list — no real credentials |
| `scripts/claude-kit/` | `install-user-kit.sh` (kit into `~/.claude`), `clone-repos.sh` (working set by group), `install-skill-packs.sh` (register + install the skill-pack plugins), `configure-repo.sh` (wire one repo, `--check` to verify), `kit_check.py` (read-only validator), `dedupe_plugins.py` (move user copies of plugin components aside) — flags and the no-delete contract: [scripts/claude-kit/README.md](scripts/claude-kit/README.md) |
| `install/repos.txt` | The working-set inventory — one `group\|org/repo\|why` per line, mirrored from §4. Single source of truth for `clone-repos.sh` |

The installers never delete and never overwrite without `--force`; see
`PLAYBOOK_INSTALL.md` for the full bootstrap (clone, install, configure,
verify) and `kit/README.md` for the ownership rules.

### 3.2 Per-language kits

Each language directory ships the backend playbooks specialized to that
language's implementation, plus the matching `.claude/` kit:

| Directory | Content | Status |
| --- | --- | --- |
| `golang/` | Runnable Go template (slices, auth core, worker, realtime) + `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, kit linters | Ready |
| `rust/` | Runnable Rust workspace (11 crates, axum 0.8, sqlx 0.8, sessions, outbox, realtime) + `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents (incl. `unsafe-reviewer`), 9 skills/commands, contract evals | Ready |
| `nodejs/` | Runnable Node 24 + TypeScript template (Fastify 5, Kysely, full auth core, outbox worker, WebSocket + SSE) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, 14 kit contract evals | Ready |
| `python/` | Runnable Python 3.12 + uv template (FastAPI, pydantic v2, SQLAlchemy 2 async, Alembic, full auth core, outbox worker, WebSocket + SSE) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, 14 kit contract evals | Ready |

The Go directory is the reference implementation for the rest.

## 4. Repository map

Orgs: **Facta-Dev** (products, tooling), **contextzero** (NEST / CTX0),
**everythink-ai** + **Technorides** (client rebuilds), **matiasbaglieri**
(owner's personal tooling). Visibility varies; several public repos are
forked skill packs vendored for use, not authored here.

### 4.1 Products

| Repo | Stack | What it is |
| --- | --- | --- |
| `Facta-Dev/talos` | Python | TALOS — organizational AI infrastructure: Cortex (multi-agent chat), Billing, Conductor (orchestration), System Config, Pulse (insights) |
| `Facta-Dev/deino` | TypeScript | Deino — oracle intelligence / auditable forecasting infrastructure |
| `Facta-Dev/staxis` | TypeScript | Staxis — crypto trading infrastructure |
| `Facta-Dev/v0ss` | Rust | v0ss — real-time subtitles plus answers from your own knowledge base |
| `Facta-Dev/enyo-platform` | Go | ENYO — self-hosted security testing and OSINT platform |
| `Facta-Dev/heimdall-platform` | TypeScript | Heimdall — "the guardian of the pipeline": reply triage. Reference implementation for every playbook here |
| `Facta-Dev/trustanova` | Python | TrustaNova — AI-assisted online dispute resolution (ODR) |
| `Facta-Dev/ReachPilot`, `Facta-Dev/ReachPilotPro` | Python / Rust | ReachPilot — multi-platform social engagement engine |
| `Facta-Dev/package_builder` | HTML | Package Builder (`pb`) — the FACTA template factory: project skeletons (React, Python, Rust, RN, CLI, Chrome ext), the `pb` Claude Code plugin, 16 invariants as PR blockers (`pb.facta.dev`) |
| `Facta-Dev/ei2` | Python | **Eidos** — 1,756 book-derived archetype packages for Claude Code across 12 segments; assets-only offer |
| `Facta-Dev/zypho-editor`, `zypho-platform`, `zypho_backend`, `zypho_frontend`, `zypho-chrome-extension` | TS / Python / HTML | The zypho editor suite |
| `contextzero/nest` | — | NEST — local-first control plane for AI coding agents (free, self-hosted) |
| `contextzero/ctx0` | — | CTX0 — the managed-cloud version of NEST |

### 4.2 Content, site and side products

| Repo | Stack | What it is |
| --- | --- | --- |
| `Facta-Dev/facta_archetype` | HTML + Python | Content/social engine: landings, slide decks, GIFs, MP4s, per-product identities |
| `Facta-Dev/facta_frontend` | JavaScript | facta.dev site and the private investor deck |
| `Facta-Dev/blog-templates` | HTML | Crawled blog post templates by provider (`article.md`, `article.html`, `meta.json`) |
| `Facta-Dev/3d-landings-pro` | Python | Scroll-scrubbed "fly through the world" landing templates |
| `Facta-Dev/claude-skill` | Python | Skill that makes Claude build cinematic landing pages |
| `Facta-Dev/ai-job-search` | JavaScript | AI job-application framework that runs locally |
| `Facta-Dev/agents-office` | JavaScript | Agents Office v3 — 3D isometric office where agents do real work |
| `Facta-Dev/synthia_frontend` | JavaScript | SYNTHIA platform — enterprise AI orchestration dashboard (React) |

### 4.3 Client rebuilds

| Repo | Org | What it is |
| --- | --- | --- |
| `everythink_platform` | everythink-ai | EveryThink platform (the 90-day rebuild, in progress) |
| `everythink_android`, `everythink_ios`, `everythink_backend` | matiasbaglieri | EveryThink mobile + backend |
| `everythiink_arxiv` | everythink-ai | EveryThink arXiv corpus |
| `transportation-bank` | Technorides | TechnoRides — transportation bank backend + reference book |
| `technorides_eu`, `technorides_web`, `technorides_web_premium` | Technorides | TechnoRides platform surfaces |
| `trustanova-platform` | Facta-Dev | TrustaNova platform rebuild — ODR monorepo; the legacy surface being replaced sits in `to_migrate/` |

### 4.4 Skills and factory packs

| Repo | What it is |
| --- | --- |
| `Facta-Dev/skills` | Engineering skill pack ("skills for real engineers") |
| `Facta-Dev/skills-images-video-audio-3d` | Image, video, audio and 3D generation skills (84 skills) |
| `Facta-Dev/garden-skills` | Web design, knowledge retrieval, video presentation skill pack |
| `Facta-Dev/beautifulgo`, `beautifulnodejs`, `beautifulpython`, `beautifulrust` | Idiomatic-language skill packs |
| `Facta-Dev/archify` | Architecture / workflow / sequence diagram skill |
| `Facta-Dev/ponytail` | Fork of the lazy-senior-dev skill behind the `ponytail` plugin |
| `Facta-Dev/superpowers`, `Facta-Dev/frontend-slides`, `Facta-Dev/marketingskills`, `Facta-Dev/ui-ux-pro-max-skill`, `Facta-Dev/awesome-claude-skills`, `Facta-Dev/awesome-claude-*` | Vendored skill/framework collections |

### 4.5 Internal tooling

| Repo | What it is |
| --- | --- |
| `Facta-Dev/facta-playbook` | This repo: working model, plugins, playbooks |
| `Facta-Dev/aws-ec2-setup` | Declarative Ubuntu infrastructure stack (PostgreSQL 17 + pgvector) |
| `Facta-Dev/claude-code-cli-cost-tracker` | Cost tracking for the `claude` CLI |
| `matiasbaglieri/terrain` | Agent tooling: RTK-style read/grep, skills and codegraph wiring |
| `Facta-Dev/OmniRoute` | Model/gateway routing dashboard (private working copy of `diegosouzapw/OmniRoute`) |
| `contextzero/repomix` | Private working copy of `yamadashy/repomix` with local changes on `main` |

### 4.6 Reference clones (upstream, read-only)

Cloned for study or as vendored dependencies, not owned or contributed to:
`storytold/craft-anything`, `rmcrackan/Libation`, `OlaProeis/Ferrite`,
`jo-inc/camofox-browser`, `aldinokemal/go-whatsapp-web-multidevice`,
`EasyDevv/project-to-markdown`, `JoasASantos/REA-Prompts`,
`samber/cc-skills-golang`, and the upstream forks in the Facta-Dev org
(`ChatDev`, `Fooocus`, `FreeCAD`, `HunyuanVideo`, `MinerU`, `Wav2Lip`,
`facefusion`, …).

---

## 5. Adding a project to the map

A repo belongs here when it is actively worked on. To add one:

1. Add a row to the matching table — repo, stack, one line of role.
2. Append the `model-routing:ollama` block to its `CLAUDE.md` (idempotent
   marker; see `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md §6`).
3. Enable only the project-level plugins the baseline does not cover.
4. Point the repo's `CLAUDE.md` at the playbooks in §3 instead of restating
   them.
5. Configure it with `scripts/claude-kit/configure-repo.sh <repo> --kit=<lang>`
   and confirm with `--check`; then run `kit_check.py` and confirm the repo
   carries no user-scope duplicate of a plugin command or agent.
