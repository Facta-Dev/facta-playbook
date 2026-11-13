# FACTA Playbook — how we work

This repo is the operating contract: **which plugins to run, which playbook
governs each kind of implementation, and which repositories make up the
working set.** Product-facing conventions (identity, launches, copy) stay in
their own repos; this one is about how the work gets done.

The playbooks were written for **Heimdall** and are language-agnostic —
Heimdall is the reference implementation; every other project adopts them by
adapting the stack profiles, not the rules.

---

## 0. Start here — the install decision tree

One command reads the machine and walks the tree (shell only, zero model
tokens; `--dry-run` previews the plan without touching anything):

```bash
bash ~/ai-coding-playbook/scripts/claude-kit/bootstrap.sh --dry-run   # preview
bash ~/ai-coding-playbook/scripts/claude-kit/bootstrap.sh             # install
```

```text
              PRECONDITIONS: git · python3 ≥ 3.9 · claude ≥ 2.1.251 · gh auth
                       │
                  OS?
             ├─ Linux / macOS / WSL2 ──────► continue
             └─ native Windows ─► use WSL2 first, re-run inside it
                       │
              PROVIDER?
             ├─ Ollama answers at 127.0.0.1:11434 ──► OLLAMA path
             │    settings env = tier aliases (opus/sonnet/haiku → Ollama
             │    cloud models) + endpoint + literal token `ollama` + cheap
             │    subagent routing.  Verify: §9 probes of the routing runbook.
             └─ otherwise (Anthropic login / gateway) ─► ANTHROPIC path
                  settings env = none (no endpoint pinning); `model: sonnet`
                  still applies; native --advisor available (runbook §7)
                       │
              SCOPE — the same shell steps on both paths
             ├─ user kit      install-user-kit.sh [--provider=ollama|anthropic|auto]
             ├─ kit check     kit_check.py            (0 errors required)
             ├─ working set   clone-repos.sh --group=selected --dir ~/work
             ├─ skill packs   install-skill-packs.sh --dir ~/work
             ├─ cost tracker  ~/work/claude-code-cli-cost-tracker/install.sh
             └─ each repo     configure-repo.sh <repo> --kit=<golang|nodejs|python|rust|extension|desktop|frontend-mobile|mobile-android|mobile-ios>

VERIFY: kit_check.py · deny probe (PLAYBOOK_INSTALL §5) · routing probes (§9)
        · /context · /doctor
```

The provider branch changes only which settings env lands in
`~/.claude/settings.json`; everything downstream is identical shell work.
`bootstrap.sh` runs the four chain steps and prints the two judgment-call
steps it does not automate (per-repo wiring and the tracker's own installer).

Flags, groups, verification and rollback: [PLAYBOOK_INSTALL.md](PLAYBOOK_INSTALL.md).
`install-skill-packs.sh` registers the marketplaces from GitHub by default
(`--local` resolves them from the clones instead).

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
| `PLAYBOOK_JAVA_J2EE.md` | Java enterprise (J2EE / Jakarta EE 11): Maven modules, CDI, JPA/JTA, JMS/outbox, JAX-RS, app-server deployment | Any Java enterprise backend |
| `PLAYBOOK_FRONTEND.md` | Frontend on React: SPA, MVVM + shared SDK (the client playbooks specialize it) | Any web client |
| `PLAYBOOK_CHROME_EXTENSION.md` | Chrome MV3 extension: service worker, messaging, permissions, store review | Any extension |
| `PLAYBOOK_DESKTOP_QT.md` | Desktop client on Qt/QML: layering, threading, packaging and updates | Desktop clients |
| `PLAYBOOK_CLI.md` | Cross-platform terminal CLI/TUI: headless core + many frontends, stdout/stderr and exit-code contracts, PTY tests, signing and updates | Any CLI/TUI product |
| `PLAYBOOK_FRONTEND_MOBILE.md` | Mobile frontend on React Native + Expo: variants, device classes, quality governor, scene engines, push | Any RN/Expo app |
| `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` | Native Android on Kotlin + Compose: Gradle module graph, UDF, Navigation 3, FCM, ARCore | Any native Android app |
| `PLAYBOOK_MOBILE_IOS_SWIFT.md` | Native iOS on Swift 6 + SwiftUI written as code: Swift packages, Observation/MVVM, APNs, StoreKit, scene engines, App Store | Any native iOS app |
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
| `kit/settings.fragment.json` | Base settings fragment merged into `~/.claude/settings.json` on every machine: default model (`sonnet` — the cost lever) and the deny list. Provider-agnostic |
| `kit/settings.ollama.json` | Ollama-only fragment, merged on top when Ollama is the provider: tier aliases (`opus`/`sonnet`/`haiku` → Ollama cloud models), local endpoint and the fixed literal token `ollama`, cheap subagent routing |
| `scripts/claude-kit/` | `bootstrap.sh` (decision tree: OS + Ollama/Claude detection, chains the installers), `install-user-kit.sh` (kit into `~/.claude`, `--provider` picks the settings env), `clone-repos.sh` (working set by group), `install-skill-packs.sh` (register + install the skill-pack plugins), `configure-repo.sh` (wire one repo, `--check` to verify), `kit_check.py` (read-only validator), `dedupe_plugins.py` (move user copies of plugin components aside) — flags and the no-delete contract: [scripts/claude-kit/README.md](scripts/claude-kit/README.md) |
| `install/repos.txt` | The working-set inventory — one `group\|org/repo\|why` per line, mirrored from §4. Single source of truth for `clone-repos.sh` |

The installers never delete and never overwrite without `--force`; see
`PLAYBOOK_INSTALL.md` for the full bootstrap (clone, install, configure,
verify) and `kit/README.md` for the ownership rules.

### 3.2 Per-stack kits

Each kit directory ships a runnable template for its stack, the playbooks
specialized to it, and the matching `.claude/` plane:

| Directory | Content | Status |
| --- | --- | --- |
| `golang/` | Runnable Go template (slices, auth core, worker, realtime) + `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, kit linters | Ready |
| `rust/` | Runnable Rust workspace (11 crates, axum 0.8, sqlx 0.8, sessions, outbox, realtime) + `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents (incl. `unsafe-reviewer`), 9 skills/commands, contract evals | Ready |
| `nodejs/` | Runnable Node 24 + TypeScript template (Fastify 5, Kysely, full auth core, outbox worker, WebSocket + SSE) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, 14 kit contract evals | Ready |
| `python/` | Runnable Python 3.12 + uv template (FastAPI, pydantic v2, SQLAlchemy 2 async, Alembic, full auth core, outbox worker, WebSocket + SSE) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 6 backend playbooks, 11 agents, 9 skills/commands, 14 kit contract evals | Ready |
| `extension/` | Runnable Chrome MV3 template (Vite 7, React 19 side panel, classic service worker, zod message protocol with sender validation, versioned storage, managed policy, zero host permissions) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 7 client playbooks, 11 agents, 9 skills/commands, node contract evals | Ready |
| `desktop/` | Runnable Qt 6 Widgets template (C++20, CMake presets, one static library per layer with link-time rules, MVVM notes slice, SQLite, managed policy, headless Qt Test) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 7 client playbooks, 11 agents, 9 skills/commands, C++ contract evals | Ready |
| `frontend-mobile/` | Runnable React Native + Expo template (SDK 58, React 19.3, RN 0.88, plain App entry, device tiers + quality governor, typed route table, MVVM notes slice, vitest architecture tests, Metro export gate) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 7 client playbooks, 11 agents, 9 skills/commands, node contract evals | Ready |
| `mobile-android/` | Runnable native Android template (Gradle 9.6, AGP 9.5, Kotlin 2.4, Compose, one Gradle module per layer, Navigation 3 typed NavKey, UDF notes slice, JVM tests, Gradle kit evals) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 7 client playbooks, 11 agents, 9 skills/commands, JVM contract evals | Ready |
| `mobile-ios/` | Runnable native iOS template (Swift 6, SwiftUI, iOS 18, XcodeGen `project.yml`, one SwiftPM package per layer, typed `NotesRoute` enum, UDF notes slice, Swift Testing, node contract evals, native gates that SKIP off macOS) + `docs/` 00–14, `AGENTS.md`, `MEMORY.md`, 7 client playbooks, 11 agents, 9 skills/commands, node contract evals | Ready (native gates pending macOS/CI) |

The Go directory is the reference implementation for the backend kits; the
extension kit is the reference for the client kits.

Every kit is a **runnable template**, not a snapshot: a renamed copy is a
working project. The contract a kit must satisfy — required surface, two eval
tiers, frozen domain-adapted fixtures, local playbook copies — is
`PLAYBOOK_CLAUDE_CODE.md` §21; the shared gate surface (`make ci`, `make evals`,
`make verify`) and what each gate proves is `PLAYBOOK_TESTS.md` §12.

### 3.3 Client families and the instruction budget

Each client family gets its own playbook and its own path-scoped rule
(`frontend.md`, `desktop.md`, `mobile.md`, `android.md`, `extension.md`), and **none is
`@`-imported**. The harness keeps the first 40,000 chars of an instruction
file and caps the always-on set (CLAUDE.md/AGENTS.md + imports + path-less
rules) at 120,000 chars; importing a playbook set is how a repo silently loses
half its contract while warning every session.

`scripts/instruction-budget.sh` ships in every kit and runs from
`verify-template.sh`; it fails when a file would be truncated or the set is
over budget. To adopt a playbook, add the rule that names it — not an import.
When a playbook passes ~40k chars it stops working as one instruction file:
split it by concern.

### 3.4 The Beautiful series

Every stack playbook has a runnable, machine-checkable companion repository.
Each one ships three standard Agent Skills — write (`<stack>-best-practices`),
review (`<stack>-review`) and port (`<stack>-port`) — plus a drop-in linter
config whose rules are proven to fire, a skill validator, and the same gate
shape (`check-skill`, `check-examples`, `check-lints` or the native
equivalent). The playbooks are the contract; these repositories are the
enforcement and the worked examples for frontier models.

| Playbook | Companion repo(s) | Stack |
| --- | --- | --- |
| `PLAYBOOK_BACKEND.md` | [beautifulgo](https://github.com/Facta-Dev/beautifulgo), [beautifulnodejs](https://github.com/Facta-Dev/beautifulnodejs), [beautifulpython](https://github.com/Facta-Dev/beautifulpython), [beautifulrust](https://github.com/Facta-Dev/beautifulrust) | Go, Node.js/TypeScript, Python, Rust |
| `PLAYBOOK_FRONTEND.md` | [beautifulreact](https://github.com/Facta-Dev/beautifulreact) | React 19 + TypeScript |
| `PLAYBOOK_CHROME_EXTENSION.md` | [beautifulextension](https://github.com/Facta-Dev/beautifulextension) | Chrome MV3 + TypeScript |
| `PLAYBOOK_DESKTOP_QT.md` | [beautifulqt](https://github.com/Facta-Dev/beautifulqt) | Qt 6 + C++20 |
| `PLAYBOOK_FRONTEND_MOBILE.md` | [beautifulreactnative](https://github.com/Facta-Dev/beautifulreactnative) | React Native + Expo |
| `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` | [beautifulandroid](https://github.com/Facta-Dev/beautifulandroid) | Kotlin + Compose |
| `PLAYBOOK_MOBILE_IOS_SWIFT.md` | [beautifulios](https://github.com/Facta-Dev/beautifulios) | Swift 6 + SwiftUI |
| `PLAYBOOK_CLI.md` | [beautifulcli](https://github.com/Facta-Dev/beautifulcli) | TypeScript CLI/TUI |

The shared contracts — `PLAYBOOK_URLS.md`, `PLAYBOOK_TESTS.md`,
`PLAYBOOK_SECURITY.md`, `PLAYBOOK_AUDIT.md` and `PLAYBOOK_CLAUDE_CODE.md` —
apply across the series: every companion repo carries the same three-skill
shape, the same "unbreakable contract" wording, and a CI workflow, and every
rule it enforces cites the playbook it comes from. All repositories are public
under <https://github.com/Facta-Dev> and link to each other as siblings.

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
| `Facta-Dev/ai-coding-playbook` | This repo: working model, plugins, playbooks |
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
   them — through a path-scoped rule, never an `@`-import (§3.3).
5. Configure it with `scripts/claude-kit/configure-repo.sh <repo> --kit=<lang>`
   and confirm with `--check`; then run `kit_check.py` and confirm the repo
   carries no user-scope duplicate of a plugin command or agent.
