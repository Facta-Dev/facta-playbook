# `<app>` — Chrome extension skeleton

> Companion repo:
> [beautifulextension](https://github.com/Facta-Dev/beautifulextension) — the
> Agent Skills and linter config for this stack.

A Manifest V3 Chrome extension template: TypeScript + React 19 side panel, a
classic service worker, zod contracts shared by both ends of the message
protocol, a typed storage layer with schema versions, and **zero host
permissions**. Every `<app>`, `<APP>` and `<scope>` token is a **literal
placeholder** — the package name is invalid until you rename, so the project
cannot build (and no lockfile is committed). See
[How to apply](#how-to-apply).

## Layout

```
<app>/
├── entrypoints/           # THIN: composition roots only
│   ├── background.ts      # service worker: builds the container, registers the RPC server
│   ├── content/example.content.ts
│   ├── options/{index.html,main.ts}
│   └── sidepanel/{index.html,main.tsx}
├── src/
│   ├── shared-kernel/     # ids · result · errors — pure, imported by everyone
│   ├── contracts/         # zod: protocol · notes · storage · policy · errors
│   ├── platform/          # THE ONLY place that touches chrome.*
│   │   ├── runtime.ts     #   the message transport
│   │   ├── storage.ts     #   local / session / managed areas
│   │   └── sidepanel.ts   #   the panel behaviour
│   ├── messaging/         # typed RPC client + server, sender validation
│   ├── storage/           # typed stores, schema versions, migrations
│   ├── policy/            # managed policy reader + resolver
│   ├── telemetry/         # logger + redaction
│   ├── security/          # sender allowlist, URL matchers
│   ├── features/notes/    # the example slice — replace with your domain
│   └── manifest.ts        # MV3 manifest, written to dist/manifest.json by the build
├── packages/sdk/          # typed API client (no React, no chrome)
├── public/_locales/en/messages.json
├── tests/                 # notes/frontend/{unit,contract} · _platform · _shared/fakes
├── evals/                 # native contract tests (node --test) + live smoke
├── managed-schema.json    # the storage.managed schema (admin policy)
├── .claude/               # agents · skills · commands · schemas · settings · manifest
├── scripts/               # rename · verify-template · lint-* · leak-guard · budget · runtime
├── PLAYBOOK_*.md         # seven local contracts — read on demand
├── AGENTS.md             # agent instructions; CLAUDE.md is a symlink to it
├── MEMORY.md             # durable premise: slices, permissions, protocol, storage
├── Makefile              # ci · build · test · lint · format · verify
└── package.json · vite.config.ts · vitest.config.ts · tsconfig*.json
```

## The permission model

`src/manifest.ts` declares exactly three permissions — `storage`,
`sidePanel`, `alarms` — and **no host permissions at all**
(`host_permissions: []`). The content script matches `https://example.com/*`
only. A new permission is a security review, not a one-line change: it needs
an entry in `docs/permissions-register.md`, a justification and a test. The
`debugger` permission is never declared, not even as an optional one. The
`evals/contracts.test.mjs` suite asserts this posture, so a regression fails
the gate instead of shipping it.

## The messaging model

One envelope, `{ v, id, type, correlationId, payload }`, defined once in
`src/contracts/protocol.ts` and validated by the same zod schemas on both
ends. Unknown versions and unknown types are rejected with a typed error;
errors are values (`Result<T, RpcError>`), never exceptions thrown across an
execution context. Every handler validates the sender **before** parsing
anything: the sender id must equal the extension id, the origin must be an
extension page, the frame must be the top frame, and the message type must
allow the sender's class (`ui` or `content`). The per-type allowlist lives
in the `PROTOCOL` table.

## The gate

```sh
make ci                  # pnpm typecheck + lint (eslint + dependency-cruiser) + test + build
pnpm run evals           # node --test evals/contracts.test.mjs (native, zero deps)
```

`pnpm install` needs a network and writes the lockfile the template does not
ship — run it on the renamed copy, never in the template root.

## Working with agents

`AGENTS.md` (symlinked as `CLAUDE.md`) is the operating instruction file: hard
rules, the never-list, the layers and their rules, the gates, the spec → test
→ code workflow, the playbook index. `MEMORY.md` holds the premise — slices,
permission posture, the message envelope, the storage version, the test
tiers, the stack.

Seven local contracts ship next to this file and load **on demand** through
path-scoped rules, never as `@`-imports: `PLAYBOOK_CHROME_EXTENSION.md`
(architecture, manifest, messaging, storage, security), `PLAYBOOK_TESTS.md`
(test tiers, fixtures, gates, audit evidence), `PLAYBOOK_SECURITY.md` (threat
model, sender validation, CSP, secrets), `PLAYBOOK_FRONTEND.md` (UI layers,
ViewModels, accessibility), `PLAYBOOK_URLS.md` (route and API naming, the
error envelope), `PLAYBOOK_AUDIT.md` (SOC 2 / ISO 27001 / ISO 9001 programs)
and `PLAYBOOK_CLAUDE_CODE.md` (the kit, SDD + TDD loop, governance).

The `.claude/` layer ships 11 agents across four divisions (`core`, `client`,
`security`, `testing`), 9 skills with 1:1 slash commands (`/add-slice`,
`/add-message`, `/permission-change`, `/storage-migration`, `/review`,
`/spec`, `/plan`, `/learn`, `/ship`), schema-driven linters in `scripts/`, a
`PreToolUse` Bash guard, and a `manifest.json` target matrix for installing
the kit into other agent runtimes.

## How to apply

1. Copy this directory to your repo root.
2. `bash scripts/rename.sh <app> <scope>` — the script applies the
   `<scope> → <APP> → <app>` substitutions in safe order.
3. `pnpm install` — creates the lockfile; keep it out of the template.
4. `pnpm format` — one run: the substituted tokens re-align the Markdown tables.
5. `make ci` — green.
6. Replace the `notes` example slice with your domain.

### Installing the `.claude` runtime

The kit installs the same agents, skills and commands into other agent
runtimes (Cursor, Codex, Gemini, Zed, …) from the target matrix in
`.claude/manifest.json`:

```sh
bash scripts/install-claude-runtime.sh                        # claude + claude-project
bash scripts/install-claude-runtime.sh --targets=cursor,codex # opt-in runtimes
```

It is idempotent and never clobbers a file you already own — a pre-existing
destination is skipped with a warning, and every operation is recorded in
`.claude/install-state.json`. `bash scripts/verify-template.sh` stages a
throwaway copy under `/tmp`, renames it and runs every gate on the copy.
