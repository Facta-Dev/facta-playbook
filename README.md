# FACTA Playbook — how we work

This repo is the operating contract: **which plugins to run, which playbook
governs each kind of implementation, and which repositories make up the
working set.** Product-facing conventions (identity, launches, copy) stay in
their own repos; this one is about how the work gets done.

The playbooks were written for **Heimdall** and are language-agnostic —
Heimdall is the reference implementation; every other project adopts them by
adapting the stack profiles, not the rules.

---

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

Project-level extras live in the project's `.claude/settings.json` — the
frontend/site repo (`facta_frontend`) is the current example: `ei2`, `pb`,
the ten `voltagent-*` agent packs, `claude-seo@agricidaniel-claude-seo` and
`github@claude-plugins-official`.

Rule: install at project level only what the baseline does not cover, and
never duplicate a baseline plugin in a project file.

## 3. Standard playbooks

| File | Governs | Use when |
| --- | --- | --- |
| `PLAYBOOK_CLAUDE_CODE.md` | The Claude Code implementation kit: files, agents, skills, rules, hooks, permissions, memory, collaboration protocol, parallel sessions, SDD + TDD loop | Starting a repo, or auditing whether a repo is agent-ready |
| `PLAYBOOK_BACKEND.md` | Backend architecture: hexagonal + vertical slices, language-agnostic folder and dependency rules | Any backend service, any language |
| `PLAYBOOK_FRONTEND.md` | Frontend: React SPA, React Native, Chrome MV3, MVVM + shared SDK | Any client |
| `PLAYBOOK_URLS.md` | Route/URL contract for API and SPA routes; the user-type segment never authorizes | Designing endpoints or permissions |
| `PLAYBOOK_TESTS.md` | Testing contract, slice-aligned, doubling as audit evidence | Every implementation |
| `PLAYBOOK_SECURITY.md` | Security contract with per-language profiles (Node, Python, Go, Rust) | Every backend, before release |
| `PLAYBOOK_AUDIT.md` | SOC 2 / ISO 27001 / ISO 9001 preparation, consuming the other playbooks as controls | Certification, enterprise or regulated deals |
| `PLAYBOOK_REA.md` | REA setup: register the reverse-engineering MCP + skill with every agent on the machine | Analyzing binaries, bundles or captured traffic |
| `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md` | Model tiers (`opus`/`sonnet`/`haiku`), `advisor` agent, settings, verification, gotchas, rollback | Setting up a new machine or a new repo |

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
| `Facta-Dev/trustanova`, `Facta-Dev/trustanova-platform` | Python | TrustaNova — AI-assisted online dispute resolution (ODR) |
| `Facta-Dev/ReachPilot`, `Facta-Dev/ReachPilotPro` | Python / Rust | ReachPilot — multi-platform social engagement engine |
| `Facta-Dev/zypho-editor`, `zypho-platform`, `zypho_backend`, `zypho_frontend`, `zypho-chrome-extension` | TS / Python / HTML | The zypho editor suite |
| `Facta-Dev/package_builder` | HTML | The factory behind every product: 12 instantiable stacks behind a fail-closed spec gate (`pb.facta.dev`) |
| `Facta-Dev/ei2` | Python | ei2 — prompt/content package library (assets-only product) |
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

### 4.3 Client rebuilds

| Repo | Org | What it is |
| --- | --- | --- |
| `everythink_platform` | everythink-ai | EveryThink platform (the 90-day rebuild, in progress) |
| `everythink_android`, `everythink_ios`, `everythink_backend` | matiasbaglieri | EveryThink mobile + backend |
| `everythiink_arxiv` | everythink-ai | EveryThink arXiv corpus |
| `transportation-bank` | Technorides | TechnoRides — transportation bank backend + reference book |
| `technorides_eu`, `technorides_web`, `technorides_web_premium` | Technorides | TechnoRides platform surfaces |

### 4.4 Skills and factory packs

| Repo | What it is |
| --- | --- |
| `Facta-Dev/skills` | Engineering skill pack ("skills for real engineers") |
| `Facta-Dev/skills-images-video-audio-3d` | Image, video, audio and 3D generation skills (84 skills) |
| `Facta-Dev/garden-skills` | Web design, knowledge retrieval, video presentation skill pack |
| `Facta-Dev/beautifulgo`, `beautifulnodejs`, `beautifulpython`, `beautifulrust` | Idiomatic-language skill packs |
| `Facta-Dev/archify` | Architecture / workflow / sequence diagram skill |
| `Facta-Dev/ponytail` | Fork of the lazy-senior-dev skill behind the `ponytail` plugin |
| `Facta-Dev/superpowers`, `Facta-Dev/frontend-slides`, `Facta-Dev/marketingskills`, `Facta-Dev/ui-ux-pro-max-skill`, `Facta-Dev/awesome-claude-*` | Vendored skill/framework collections |

### 4.5 Internal tooling

| Repo | What it is |
| --- | --- |
| `Facta-Dev/facta-playbook` | This repo: working model, plugins, playbooks |
| `Facta-Dev/aws-ec2-setup` | Declarative Ubuntu infrastructure stack (PostgreSQL 17 + pgvector) |
| `Facta-Dev/claude-code-cli-cost-tracker` | Cost tracking for the `claude` CLI |
| `Facta-Dev/toptal-interview` | `ti` backend — FastAPI/asyncpg/alembic, an interview exercise built with the factory |
| `matiasbaglieri/terrain` | Agent tooling: RTK-style read/grep, skills and codegraph wiring |
| `matiasbaglieri/OmniRoute` | Model/gateway routing dashboard |

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
