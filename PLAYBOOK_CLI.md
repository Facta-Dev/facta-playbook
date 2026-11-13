# PLAYBOOK_CLI.md — Cross-platform terminal CLI/TUI playbook (Linux · macOS · Windows)

Architecture and engineering contract for building, shipping and operating a
**command-line product in the style of Claude Code**: an interactive terminal UI
(TUI) with streaming output, slash commands and permission prompts, **and** a
fully scriptable headless mode (print/JSON/NDJSON), resource subcommands,
extensibility (skills, hooks, plugins, MCP) and managed enterprise
configuration, all running on **Linux, macOS and Windows**.

It follows the same architecture as the rest of the product family:
**hexagonal + vertical slices**, a generated **SDK** that mirrors the URL
contract, MVVM-style presentation, server-side authorization, spec/test-driven
workflow. It extends and must stay consistent with `PLAYBOOK_URLS.md` (routes,
permissions), `PLAYBOOK_BACKEND.md` (slices, realtime, idempotency),
`PLAYBOOK_FRONTEND.md` (MVVM + SDK, error taxonomy), `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_TESTS.md`, `PLAYBOOK_AUDIT.md`, `PLAYBOOK_CLAUDE_CODE.md` (instruction,
skills, hooks, multi-session patterns), `PLAYBOOK_DESKTOP_QT.md` (realtime
state machine, policy, updater) and the mobile playbooks (device-code style
sign-in is new here; push is not applicable).

The runnable, machine-checkable companion is
[beautifulcli](https://github.com/Facta-Dev/beautifulcli): three Agent Skills
(write, review, port) and a drop-in eslint config that enforces this contract.

The rules that carry the whole file:

1. **One headless core, many frontends.** The engine (use cases, ports, event
   stream) knows nothing about terminals. The TUI, print mode, resource
   subcommands, the RPC server and IDE bridges are clients of the same core.
2. **Everything is scriptable.** Every feature works without a TTY. `stdout` is
   data, `stderr` is diagnostics and UI chrome; exit codes and JSON schemas are
   versioned contracts.
3. **Be TTY-aware.** Interactive features only when stdin/stdout are terminals;
   honor `NO_COLOR`, `CI`, `TERM=dumb`; degrade, never crash.
4. **Terminal state is sacred.** Raw mode, alternate screen, cursor, mouse,
   bracketed paste and keyboard protocols are always restored on exit, crash or
   signal.
5. **Untrusted text never reaches the terminal raw.** Model output, server
   content, file content and filenames are sanitized for escape sequences before
   rendering.
6. **Safe by default.** Workspace trust, explicit permission model, no secrets
   in logs or config, signed releases, verified updates.
7. **Cross-platform by construction.** OS differences live in `platform/`
   adapters; three-OS CI is mandatory; paths, shells, signals and encodings are
   tested, not assumed.
8. **Startup time is a feature.** Fast path for `--version`/`--help`, lazy loading,
   compiled single binary, budgets enforced in CI.
9. **Authorization is on the server.** The CLI never decides access; `{userType}`
   selects an experience, not a permission.
10. **Admins can govern it.** Managed policy, update channels, kill switch,
    diagnostics, audit trail.
11. **Releases are signed, reproducible and reversible.**

> Verification note: status as of November 2026. Reference implementations in
> this space include **Claude Code** (TypeScript, React with a forked Ink
> renderer; native per-platform binaries since v2.1.113; installable through a
> native installer, Homebrew, WinGet or npm), **Gemini CLI** (TypeScript, React
> 19 + Ink), **Codex CLI** (rewritten in Rust with ratatui in 2025) and **Crush**
> (Go, Bubble Tea v2; first released in 2025, rewritten on v2 in February 2026).
> Items marked **(verify)** depend on versions, terminal support or vendor
> behavior and must be re-checked before adoption. Budgets are starting points
> to calibrate on your own hardware.

---

## 1. Decisions up front (record each as an ADR)

| # | Decision | Recommendation | Rationale / alternatives |
|---|---|---|---|
| D1 | Language/runtime | **TypeScript (strict) on Node/Bun with Ink (React)** as the default | Reuses the generated TypeScript SDK, shared ViewModels and types with web/extension/mobile; React mental model; proven by Claude Code and Gemini CLI. **Rust + ratatui** or **Go + Bubble Tea v2** are valid when single-binary size/startup/memory dominate or the team lives in those languages; the architecture below is language-neutral (ports, events, contracts) |
| D2 | TUI renderer | **Ink** (React 19) with a thin `ui-kit`; incremental rendering, `Static` transcript region, synchronized output where supported | Fallback to plain line output when not a TTY; OpenTUI is an option if you need mouse/images/speed **(verify maturity)** |
| D3 | Packaging | **Single executable per OS/arch**: Bun `--compile` (fast startup, cross-compile; verify signing behavior on your Bun version), Node SEA (stability 1.1 "active development"; CommonJS by default via `mainFormat`, ESM supported but not combinable with `useSnapshot`; cross-target builds require `useCodeCache/useSnapshot` off), or Deno compile; **plus** an npm package that fetches the right native binary through optional platform dependencies | Users without Node; fast cold start; npm for developers and CI |
| D4 | Argument parsing | A typed command framework (e.g., Stricli/Commander/yargs, or clap/cobra in Rust/Go) with **generated help and shell completions**; commands registered from the **route table + OpenAPI** for resource subcommands | No drift with the backend |
| D5 | SDK | The same generated SDK as other clients (`sdk.app.funnels.leads.get(id)`); transport = `fetch`/undici with interceptors | `PLAYBOOK_FRONTEND.md` §3 |
| D6 | Output contracts | `text` (human), `json` (single envelope), `stream-json` (NDJSON events); schemas versioned in `contracts/` | Same pattern as Claude Code's print mode |
| D7 | Auth | **OAuth 2.0 device authorization grant** (RFC 8628) for headless/SSH and **authorization code + PKCE with loopback redirect** (RFC 8252) when a browser is available; tokens in the **OS keystore**; API tokens for CI | `PLAYBOOK_SECURITY.md` §4 |
| D8 | Local state | Files under OS-standard dirs (config/data/cache/state), JSON/JSONL, atomic writes, lock files for multi-instance safety; SQLite only if queries are needed | Debuggable, diffable |
| D9 | Realtime | SSE/WebSocket optional, same state machine and envelope as the desktop/mobile playbooks; polling fallback | `PLAYBOOK_DESKTOP_QT.md` §8 |
| D10 | Extensibility | Slash commands (markdown), skills, hooks, plugins, MCP client, all behind trust prompts and managed policy | `PLAYBOOK_CLAUDE_CODE.md` patterns applied to our own product |
| D11 | Telemetry | **Opt-in**, redacted, OpenTelemetry-compatible export; local debug logs always available | Privacy and enterprise trust |
| D12 | Updates | Signed manifest, channels (`stable`/`latest`/`canary`), staged rollout, atomic swap, rollback; admin can disable, pin or mirror | Section 12 |
| D13 | Supported platforms | Derive from the runtime's support matrix and your fleet: example floor used by a reference product is macOS 13+, Windows 10 1809+, Ubuntu 20.04+, Debian 10+, Alpine 3.19+ **(verify and document in `docs/support-matrix.md`)**; architectures x64 and arm64 on all three OSes; glibc and musl on Linux | Alpine/musl needs its own artifact |
| D14 | Shell execution | **Shell adapters per OS** (bash/zsh, PowerShell, cmd); no hard dependency on Git Bash on Windows | Reference products have required Git for Windows; avoid that coupling unless justified |
| D15 | Binary name | Short, unique, non-conflicting (check package managers and PATH collisions); example `hmd` | Collisions break installs |

---

## 2. Product surface

### 2.1 Modes

| Mode | Trigger | Behavior |
|---|---|---|
| **Interactive (TUI/REPL)** | TTY on stdin+stdout and no print flag | Conversation/command session, streaming transcript, slash commands, permission dialogs, status line |
| **Print / headless** | `-p, --print` (prompt or piped stdin) | Runs once and exits; `--output-format text\|json\|stream-json`, `--input-format text\|stream-json`, `--max-turns`, `--max-budget`, `--json-schema`, `--no-session-persistence` |
| **Resource subcommands** | `hmd <module> <resource> <verb>` | Deterministic CRUD and actions against the backend; table/JSON output; no TUI |
| **Serve (RPC)** | `hmd serve` | Headless engine over stdio JSON-RPC/NDJSON or a local socket for IDEs and other frontends |
| **MCP server (optional)** | `hmd mcp serve` | Expose selected capabilities as MCP tools **(verify product need)** |
| **Maintenance** | `doctor`, `update`, `install <channel\|version>`, `config`, `completion`, `version`, `auth login\|logout\|status` | Health, lifecycle and config |

`auth status` exits 0 when signed in and non-zero otherwise (scriptable).

### 2.2 Command taxonomy (aligned with `PLAYBOOK_URLS.md`)

```text
hmd [global flags] <module> <resource> <verb> [args]          # app experience (default userType)
hmd admin <module> <resource> <verb> --tenant <id> [args]     # admin experience (authorized server-side)
hmd partner <module> …                                        # partner experience

hmd funnels leads get <leadId>              ↔ GET   /api/v1/app/funnels/leads/{leadId}
hmd funnels leads list --stage won --limit 50 ↔ GET /api/v1/app/funnels/leads?stage=won&limit=50
hmd projects launch <projectId>             ↔ POST  /api/v1/app/projects/{projectId}/launch   (202 → --wait)
hmd admin tenants users suspend <tenantId> <userId> ↔ POST /api/v1/admin/tenants/{tenantId}/users/{userId}/suspend
```

Rules: commands are **generated from the route table and OpenAPI**, then wrapped with UX (flags, output shaping); the `admin` prefix never grants access, the backend validates token,
tenant, RBAC and ownership; verbs are `list|get|create|update|delete` plus action verbs; plural resources and IDs as positional arguments mirror the URL grammar.

### 2.3 Slash commands (inside the TUI)

Built-ins (examples): `/help`, `/status`, `/config`, `/doctor`, `/login`, `/logout`, `/clear`, `/compact` (if agentic), `/resume`, `/model`, `/permissions`, `/hooks`, `/skills`, `/plugins`, `/mcp`, `/tasks`, `/exit`.
Custom commands are markdown files (user/project scope) with frontmatter and are surfaced in autocomplete; each slash command is a thin adapter over a use case (same use case a subcommand calls).

---

## 3. Architecture: hexagonal + vertical slices + MVVM

### 3.1 Layers

```text
┌──────────────────────────────────────── CLI process ─────────────────────────────────────────┐
│ INBOUND ADAPTERS                                      OUTBOUND ADAPTERS                       │
│ argv parser · slash commands · TUI Views+ViewModels    SDK/HTTP · Realtime (SSE/WS/poll)       │
│ stdio RPC server · signal handlers · timers            Filesystem · OS keystore · Shell/process │
│                                                        Browser opener · Clipboard · Terminal   │
│            │                                           Telemetry · Updater · MCP · Plugins     │
│            ▼                                                   ▲                               │
│     APPLICATION (use cases, ports, policy, permission engine, session/context, event bus) ─────┘ │
│            ▼                                                                                   │
│     DOMAIN (entities, value objects, rules, domain events)                                     │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Contains | May depend on | Must not depend on |
|---|---|---|---|
| **domain** | Entities, value objects, rules, domain events | `shared-kernel` | Node/Bun APIs, Ink/React, SDK, filesystem, terminal |
| **application** | Use cases, **ports**, permission engine, config resolver, session manager, event bus | domain, `contracts`, `shared-kernel` | adapters, `process`, `fs`, Ink |
| **adapters/in** | argv/command handlers, slash handlers, TUI ViewModels+Views, RPC server, signal handlers | application, contracts | other slices' internals, SDK directly |
| **adapters/out** | SDK client, realtime transports, storage, keystore, shell executor, updater, MCP client, plugin loader, telemetry | application ports, `platform`, SDK | views |
| **platform** | Per-OS code behind interfaces (paths, signals, shell quoting, keychain, clipboard, terminal modes) | OS APIs | domain/application |
| **contracts** | DTOs, error codes, exit codes, event schemas, policy schema, route table | `shared-kernel` | everything else |

Rules:

* **`process.platform` branching appears only under `platform/`** (and build scripts).
* `application` talks only to ports; `adapters/in` never calls the SDK directly (always through use cases).
* A **typed event bus** carries engine events (`session.started`, `message.delta`, `tool.requested`, `permission.requested`, `tool.result`, `progress`, `result`, `error`). The TUI renders them; print mode serializes them; the RPC server forwards them.
* Enforce with dependency-cruiser/ESLint boundaries and a `no-restricted-globals`/import rule for `process`, `fs`, `child_process` outside adapters; violations fail CI.

### 3.2 Ports (examples)

```ts
export interface SessionStorePort   { append(sessionId: string, e: SessionEvent): Promise<void>; load(sessionId: string): AsyncIterable<SessionEvent>; list(): Promise<SessionInfo[]>; purge(opts: PurgeOpts): Promise<void>; }
export interface SecretStorePort    { get(key: string): Promise<string | null>; set(key: string, v: string): Promise<void>; remove(key: string): Promise<void>; }
export interface ConfigPort         { resolve(): Promise<EffectiveConfig>; set(scope: Scope, key: string, value: unknown): Promise<void>; onChange(cb: () => void): Unsubscribe; }
export interface ShellPort          { run(cmd: ShellCommand, opts: RunOpts): Promise<ShellResult>; kill(handle: ProcHandle): Promise<void>; }   // per-OS: bash/zsh/PowerShell/cmd
export interface TerminalPort       { caps(): TerminalCaps; enterRaw(): void; restore(): void; write(s: string): void; onResize(cb: (c: Size) => void): Unsubscribe; }
export interface OpenerPort         { openUrl(url: string): Promise<boolean>; openPath(p: string): Promise<boolean>; }
export interface ClipboardPort      { write(text: string): Promise<boolean>; }
export interface PermissionPort     { decide(req: ToolRequest, ctx: PermCtx): Promise<Decision>; }     // allow | deny | ask
export interface RealtimePort       { start(cfg: RtConfig): void; stop(): void; setTopics(t: Set<string>): void; /* state, events, resync */ }
export interface UpdaterPort        { check(channel: Channel): Promise<UpdateInfo | null>; apply(u: UpdateInfo): Promise<ApplyResult>; rollback(): Promise<void>; }
export interface TelemetryPort      { event(name: string, props?: Record<string, unknown>): void; flush(): Promise<void>; }
export interface ClockPort          { now(): number; }
```

Ports return `Result<T, E>`/typed errors; no exceptions across layers.

### 3.3 Presentation in the TUI (MVVM)

* **ViewModel** = a hook (or reducer) exposing state + commands, no Ink imports; where a screen exists in other clients (e.g., lead detail), reuse the shared TypeScript ViewModel from `packages/viewmodels`; CLI-specific ones live in the slice.
* **View** = Ink components (`Box`, `Text`) rendering ViewModel state; no business logic. Statuses are the shared set: `loading | ready | empty | forbidden | not-found | offline | error | policy-blocked`.
* For Rust/Go variants map to **Elm architecture** (model → message → update → view); ports and events stay identical.
* **Single source of truth** for the screen is the ViewModel/store; the transcript is derived from the event log.

### 3.4 Vertical slices

CLI slices mirror backend slices and URL modules: `auth`, `config`, `projects`, `funnels`, `campaigns`, `tenants` (admin), `usage`, `billing`, `session` (conversation/agent), `extensions` (skills/hooks/plugins/MCP), `update`. Each slice has
`domain/ application/ adapters/{in,out}/ contracts/ viewmodels/ views/`. Cross-slice access only via `contracts/`.

---

## 4. CLI UX contract (scriptable by design)

### 4.1 Streams and TTY behavior

| Situation | stdout | stderr | Interactivity |
|---|---|---|---|
| Resource subcommand, TTY | Result (table/text) | Warnings, progress | Prompts allowed (`--yes` skips) |
| Resource subcommand, piped | Result only (JSON if `--json`) | Warnings/errors | **No prompts**; fail with an actionable message |
| `-p` print mode | Final text / JSON / NDJSON | Diagnostics | None |
| Interactive TUI | UI (inline or fullscreen) | Debug logs go to file, not terminal | Full |
| `CI=true` or `TERM=dumb` | Plain text | Plain text | None; no spinners/animation |

* **Never** write progress bars, spinners or color to stdout when it is not a TTY. Progress goes to stderr only if stderr is a TTY; otherwise emit plain periodic lines or nothing.
* Respect `NO_COLOR`, `FORCE_COLOR`, `--no-color`, `--color=auto|always|never`.
* Read piped stdin when present (`cat log | hmd -p "summarize"`), with size limits and streaming.

### 4.2 Flags and environment

| Flag/env | Meaning |
|---|---|
| `-h/--help`, `-v/--version` | Standard; help is generated, includes examples and exit codes |
| `-p/--print` | Headless mode |
| `--output-format text\|json\|stream-json`, `--output <file>` | Output control |
| `--json`, `--query <expr>`, `--fields a,b` | Machine output shaping for subcommands |
| `--profile <name>`, `--tenant <id>` | Profile/tenant selection (tenant is sent as `X-Tenant-Id` and validated server-side) |
| `--yes/-y`, `--dry-run`, `--no-input` | Non-interactive control; destructive commands require `--yes` or confirmation |
| `--limit`, `--cursor`, `--all` | Pagination (`--all` streams pages, bounded) |
| `--wait`, `--timeout`, `--no-wait` | Long operations (`202` → poll/stream until terminal state) |
| `--idempotency-key <k>` | Safe retries for mutations |
| `--debug [categories]`, `--verbose`, `--quiet` | Diagnostics |
| `--config <path>`, `--settings <file>` | Config overrides |
| `--permission-mode`, `--allowedTools`, `--disallowedTools` | Permission control (agentic mode) |
| `HMD_TOKEN`, `HMD_API_URL`, `HMD_PROFILE` | Auth/config via env (token never printed) |
| `HTTP_PROXY/HTTPS_PROXY/NO_PROXY`, `SSL_CERT_FILE`/`NODE_EXTRA_CA_CERTS` | Corporate networking |

### 4.3 Exit codes (stable contract; documented in `--help`)

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments/usage |
| 3 | Authentication required/failed |
| 4 | Permission denied (`forbidden`; `requiredPermission` in the error output) |
| 5 | Not found |
| 6 | Conflict (version/idempotency) |
| 7 | Validation failed |
| 8 | Rate limited (honor `Retry-After`) |
| 9 | Network/timeout/offline |
| 10 | Server error |
| 11 | Upgrade required (`minVersion`) |
| 130 | Interrupted (SIGINT) |
| 143 | Terminated (SIGTERM) |

Scripts rely on these; changing them is a breaking change.

### 4.4 Output formats and schemas

* **Human text**: concise, aligned tables (width-aware, truncation with ellipsis, wide-character aware), never relies on color alone.
* **`--json`/`--output-format json`**: one envelope `{ "v": 1, "ok": true, "data": …, "meta": { "correlationId", "requestId", "pagination" } }` or `{ "ok": false, "error": { "code", "message", "requiredPermission?", "correlationId" } }` (same error envelope as the backend).
* **`stream-json`**: newline-delimited JSON, one event per line, versioned (`v`), ordered, flushed per event, no other text on stdout; partial-message events are opt-in; hook-lifecycle events opt-in.
* **`--json-schema`** (print mode): validates the final output against a user schema and exits non-zero on mismatch.
* Output schemas live in `contracts/`, have golden tests, and follow semver.

### 4.5 Help, completions and docs

Generated `--help` (usage, examples, flags, env vars, exit codes, links), `hmd completion bash|zsh|fish|powershell`, man pages/markdown reference generated from the command registry, "did you mean" suggestions, concise errors with next steps and a docs URL.

### 4.6 Accessibility and localization

`--accessible`/`HMD_ACCESSIBLE=1` mode: no spinners/animations, stable linear output, descriptive text instead of glyphs; no color-only meaning; ASCII fallback for box-drawing/emoji when the terminal or locale can't render them;
message keys with `Intl`-based formatting; correct width for CJK/emoji (grapheme-aware); RTL awareness for text output where applicable; honor `LANG/LC_*` and UTF-8 requirements (Windows code page handled).

---

## 5. Interactive TUI design

### 5.1 Layout and behavior

```text
┌ transcript (scrollback-friendly, streaming) ────────────────────────────────┐
│ user / assistant / tool blocks, diffs, tables, markdown (incremental render) │
├ permission / dialog region (modal, keyboard-first) ──────────────────────────┤
├ input (multiline, history, @-file mentions, / autocomplete, paste handling) ─┤
└ status line (profile · tenant · mode · tokens/cost · connection · version) ──┘
```

* **Inline mode by default**: render in the normal screen buffer so native scrollback, selection and copy/paste work; optional **fullscreen** mode (alternate screen) for dashboards/watch views. Never trap the user.
* **Flicker-free rendering**: differential updates, a static region for completed output, a dynamic region for the live area; **synchronized output** (DEC mode 2026) when supported; throttle redraws (≤ 30–60 fps); coalesce streaming deltas.
* **Streaming markdown**: render incrementally (append-only parsing, no full re-parse per token), code blocks with syntax highlighting (lazy-loaded), tables, links via OSC 8 when supported.
* **Long sessions**: window/virtualize the transcript, cap in-memory history, spill older entries to the session file, bounded diff rendering.
* **Resize**: handle size changes (`SIGWINCH` on POSIX; resize events on Windows), reflow wrapping, never crash on tiny terminals (minimum viable layout).
* **Input**: multiline editing, bracketed paste (large pastes summarized, never executed), history with search (Ctrl+R), word/line navigation, optional vim mode, IME and grapheme-correct cursor, `@` file mention with fuzzy search (respecting ignore files), `/` command palette.
* **Keybindings** are data (`keybindings.json`) with sane defaults and per-terminal fixes (Shift+Enter, Alt+Enter, Ctrl+Space issues on Windows/ConPTY, kitty keyboard protocol when available); document conflicts with multiplexers.
* **Interrupt semantics**: `Esc` cancels the current operation; first `Ctrl+C` cancels, second (or `Ctrl+D` on empty input) exits; never lose unsent input silently; background tasks continue only if designed (`/tasks`).
* **Permission dialogs**: show action, target, diff/preview, scope choices (allow once / always for this project / deny / deny and tell the assistant why), keyboard-first, timeouts configurable; every decision is logged to the session file.
* **Themes**: light/dark auto-detected (OSC 11 query with short timeout or `COLORFGBG`), high-contrast and colorblind-safe themes, 16-color fallback; no hard-coded absolute colors in components (theme tokens).
* **Notifications**: terminal bell/OSC 9 only if enabled and supported; completion sounds off by default.
* **Title and status**: set the terminal title (restore on exit) when supported.

### 5.2 Terminal capability negotiation

Never assume a feature; **detect, then enable, then fall back**.

| Capability | Detection (in order) | Used for | Fallback |
|---|---|---|---|
| TTY / size | `isatty`, `columns/rows`, resize events | Interactivity | Plain mode |
| Color depth | `NO_COLOR`, `COLORFGBG`, `COLORTERM`, `TERM`, terminfo | Themes | 16-color → monochrome |
| Unicode/emoji width | Locale/code page, grapheme width tables | Layout | ASCII symbols |
| Synchronized output (2026) | DECRQM query or `TERM_PROGRAM`/known list **(verify per terminal)** | Flicker-free redraw | Throttled redraw |
| Kitty keyboard | `CSI ? u` query with timeout, then progressive-enhancement flags | Disambiguated keys | Legacy key decoding |
| modifyOtherKeys / win32-input-mode | Terminal identity or behavioral detection (no query exists) | Disambiguated keys | Legacy key decoding |
| OSC 8 hyperlinks | Known terminals/env; no reliable query | Clickable links | Print URL |
| OSC 52 clipboard | Terminal identity; **write-only**, size-capped | Copy to clipboard | Native clipboard tool / print |
| OSC 11 background | Query with timeout | Theme | `COLORFGBG`/default dark |
| Mouse | Opt-in | Click/scroll in fullscreen | Keyboard only |
| Multiplexers (tmux/screen/zellij), SSH, containers | `TMUX`, `STY`, `ZELLIJ`, `SSH_TTY` | Conservative feature set | Disable risky protocols |

Rules: queries have **short timeouts (≈100–200 ms)** and never block; never query when not a TTY; cache the result; allow overrides (`HMD_TERMINAL_CAPS=…`) and a `doctor` report; support documentation of known-bad terminals. Terminal support claims vary across sources and versions, so keep a **tested matrix** instead of trusting tables.

### 5.3 Terminal lifecycle guard (mandatory)

```ts
export function installTerminalGuard(term: TerminalPort, onFatal: (e: unknown) => void) {
  let restored = false;
  const restore = () => { if (restored) return; restored = true; term.restore(); };   // raw off, cursor on, alt screen off, mouse/paste/kitty/sync off, title restored
  process.once('exit', restore);
  for (const sig of ['SIGINT', 'SIGTERM', 'SIGHUP'] as const) process.once(sig, () => { restore(); process.exit(128 + SIGNAL_NUM[sig]); });
  process.once('uncaughtException',  (e) => { restore(); onFatal(e); process.exit(1); });
  process.once('unhandledRejection', (e) => { restore(); onFatal(e); process.exit(1); });
}
```

Windows has no real `SIGTERM`/`SIGHUP` semantics (console close and Ctrl+Break map to events); the platform adapter normalizes them. Test crash paths: terminal must be usable after any exit.

---

## 6. Platform specifics (Linux · macOS · Windows)

### 6.1 Differences that must live in `platform/`

| Concern | Linux | macOS | Windows |
|---|---|---|---|
| Config/data/cache/state dirs | XDG (`$XDG_CONFIG_HOME`, `~/.config`, `~/.local/share`, `~/.cache`, `~/.local/state`) | `~/Library/Application Support`, `~/Library/Caches`, `~/Library/Logs` | `%APPDATA%`, `%LOCALAPPDATA%`, `%PROGRAMDATA%` (managed) |
| Managed policy | `/etc/<tool>/managed-settings.json` | `/Library/Application Support/<Tool>/managed-settings.json` or configuration profile | `%PROGRAMDATA%\<Tool>\managed-settings.json` or registry policy |
| Keystore | Secret Service/libsecret (D-Bus), KWallet; headless fallback = encrypted file 0600 | Keychain | Credential Manager (DPAPI) |
| Shell for commands | `$SHELL`/bash/zsh/sh | zsh (default)/bash | PowerShell (primary), cmd; Git Bash/WSL optional |
| Quoting/escaping | POSIX | POSIX | PowerShell/cmd rules differ; **never build command strings from untrusted input**; use argument arrays; special-case `.cmd/.bat` argument injection |
| Process tree kill | Process groups + `SIGTERM`→`SIGKILL` | Same | `taskkill /T`/Job Objects |
| Signals | POSIX signals, `SIGWINCH` | Same | Ctrl+C/Ctrl+Break events; no `SIGWINCH` |
| Paths | `/`, case-sensitive | `/`, usually case-insensitive FS | `\`, drive letters, UNC, case-insensitive, 260-char legacy limit, reserved names (`CON`, `NUL`) |
| Symlinks | Normal | Normal | Require privilege/developer mode; junctions differ |
| Executables | Mode bits | Mode bits + quarantine xattr | Extensions/PATHEXT |
| Line endings/encoding | LF, UTF-8 | LF, UTF-8 | CRLF in places; set UTF-8 handling; BOM awareness |
| Clipboard | `wl-copy`/`xclip`/`xsel`, OSC 52 | `pbcopy` | `clip.exe`, PowerShell, OSC 52 |
| Open URL/path | `xdg-open` | `open` | `start`/`rundll32` (via safe API) |
| File locking | `flock`/`fcntl` | same | `LockFileEx`/exclusive open semantics differ |
| Replace running binary | Allowed (unlink/rename) | Allowed | **Cannot overwrite a running .exe**: stage new file, swap on next launch via helper |
| Terminal host | Many emulators, multiplexers, SSH | Terminal.app, iTerm2, etc. | Windows Terminal/ConPTY, legacy conhost, VS Code terminal |

### 6.2 Windows-specific rules

* Enable virtual-terminal processing on Windows consoles; handle legacy conhost limits (no alternate-screen behavior in some hosts, limited keyboard protocols).
* Keyboard input arrives through VT sequences or Win32 input records depending on terminal/ConPTY mode; some key combos (Shift+Enter, Ctrl+Space) need the terminal's enhanced mode; keep keybindings configurable.
* Test in **Windows Terminal, conhost, PowerShell 5.1/7, cmd.exe, Git Bash and WSL**; document supported hosts.
* Never rely on Git for Windows being installed unless the ADR (D14) says so; provide PowerShell/cmd shell adapters.
* Long paths: enable `\\?\` handling where needed; avoid reserved names in generated files; case-insensitive collisions in generated paths.
* Antivirus/SmartScreen: signed binaries, stable install locations, avoid self-modifying behavior that looks like malware.

### 6.3 macOS and Linux rules

* macOS: Developer ID signing + hardened runtime + notarization; first-run Gatekeeper checks; Apple silicon and Intel builds (or universal); respect Full Disk Access boundaries; quarantine attribute on downloads.
* Linux: glibc **and** musl artifacts (Alpine); minimum glibc baseline by building on an older distro; Wayland/X11 clipboard tooling; container-friendly (no TTY, read-only FS, non-root user).

### 6.4 Paths and filesystem rules

All path handling goes through `PathPort` (normalization, containment checks with `realpath`, case rules, long-path support). **Containment check before any read/write** of user-supplied paths: resolve symlinks, reject escapes from allowed roots,
treat Windows drive/UNC specially. Atomic writes (temp file + rename), `0600`/`0700` permissions for state/config on POSIX and restricted ACLs on Windows, lock files for concurrent instances.

---

## 7. Backend integration

### 7.1 SDK and transport

* Generated SDK mirrors the URL contract; interceptors: auth (single-flight refresh), tenant (`X-Tenant-Id` validated server-side), correlation (`X-Correlation-Id`), client info (`X-Client: cli/<version> (<os>; <arch>; <runtime>)`), idempotency (`Idempotency-Key`), retry with backoff/jitter (network/429/5xx on idempotent calls, honor `Retry-After`), error mapping to the shared taxonomy
  (`unauthenticated | forbidden(requiredPermission) | not-found | conflict | validation | rate-limited | network | server | upgrade-required`) → **exit codes** (section 4.3).
* HTTP: keep-alive, HTTP/2 where available, explicit timeouts, abort on Ctrl+C, proxy env support, custom CA bundle support, TLS verification always on.
* **Pagination**: cursor-based; `--all` streams pages with bounded memory and a hard cap; backpressure-aware writes to stdout (`EPIPE` handled when piped into `head`).
* **Long operations** (`202`): `--wait` polls or subscribes to realtime until a terminal state with backoff and `--timeout`; progress to stderr; `Ctrl+C` stops waiting without cancelling the server operation unless `--cancel-on-interrupt`.
* **Version gate**: `upgrade-required` → exit 11 with a message and (if allowed) offer `hmd update`.

### 7.2 Authentication flows

| Flow | When | Notes |
|---|---|---|
| **Device authorization (RFC 8628)** | SSH, containers, no browser/TTY-limited | Print URL + user code; poll with `interval`, back off on `slow_down`; QR optional |
| **Auth code + PKCE with loopback (RFC 8252)** | Local desktop sessions | Random port on `127.0.0.1`, single-use state, short timeout, system browser (never an embedded webview) |
| **API token / service account** | CI | `HMD_TOKEN` env or `--token-file`; scoped, revocable; **never echo or log**; prefer OIDC federation in CI when available |
| **SSO/enterprise** | Managed fleets | IdP issuer/client from managed policy |

* Tokens live in the **OS keystore** (`SecretStorePort`); if none exists, require an explicit opt-in to an encrypted file with restrictive permissions and show a warning in `doctor`.
* Refresh rotation with reuse detection (server), single-flight refresh, `auth logout` revokes server-side and wipes local copies; multiple profiles/tenants (`--profile`).
* `auth status` prints identity, tenant, scopes, expiry (no secrets) and exits 0/non-zero.

### 7.3 Realtime in the CLI

`hmd events tail`, `hmd runs watch <id>`, live `--wait` use the same transports and state machine as `PLAYBOOK_DESKTOP_QT.md` §8 (SSE via streaming `fetch`, WebSocket, polling fallback), snapshot + stream, dedupe by event ID, resume (`Last-Event-ID`), heartbeats,
backoff with jitter, bounded buffers, output coalescing so a burst never floods the terminal. Parse SSE from the raw byte stream with a spec-compliant parser; token in the `Authorization` header only.

### 7.4 Offline and caching

Optional read cache with TTL for slow lookups (`--no-cache`, `hmd cache clear`); never cache secrets; outbox for mutations is **not** the default in a CLI (fail fast with a clear message) unless a spec requires it.

---

## 8. Configuration and state

### 8.1 Layers and precedence

```text
managed policy (OS-level; can lock keys)  >  CLI flags  >  environment  >  project local (.hmd/settings.local.json, gitignored)
   >  project (.hmd/settings.json, committed)  >  user (config dir)  >  defaults
```

* Schema-validated (JSON Schema/zod), versioned with migrations, `hmd config list|get|set|unset|path|edit` showing the **source layer** of each effective value.
* **No secrets in config files**; tokens only in the keystore or env.
* **Project config is untrusted until trusted**: hooks, plugins, MCP servers, custom commands and shell-executing settings from a repository require a **workspace trust** confirmation (remembered per path/hash); `-p` mode must not silently trust arbitrary repos (explicit `--trust-workspace` or a policy).
* Instruction files for the product's own assistant features (`HMD.md`, `.hmd/rules/*.md`) follow `PLAYBOOK_CLAUDE_CODE.md`: short, scoped, versioned.

### 8.2 State and housekeeping

| Data | Location | Retention |
|---|---|---|
| Sessions/transcripts (JSONL, append-only) | state dir | Configurable (default ~30 days); `--no-session-persistence`; plaintext on disk, so offer encryption/redaction options and document it |
| Logs | state/log dir | Rotated, size-capped, redacted |
| Cache | cache dir | TTL/LRU, safe to delete |
| Update staging | data dir | Cleaned after apply |
| Locks/PIDs | runtime dir | Auto-cleaned (stale detection) |

`hmd purge`/`hmd project purge` remove local state for a project; atomic writes; file locks make **multiple simultaneous CLI instances** safe (see `PLAYBOOK_CLAUDE_CODE.md` multi-session patterns).

### 8.3 Managed settings (enterprise)

Policy keys (examples): `apiBaseUrl` (allowlisted patterns), `idp` (issuer/clientId), `updates` (`disabled`, `channel`, `pinnedVersion`, `mirrorUrl`, `window`), `telemetry` (`off|errors|full`), `allowedPlugins`/`allowedMcpServers`/`deniedMcpServers`,
`permissions` (deny rules that users can't override), `disableBypassPermissionsMode`, `allowManagedHooksOnly`, `proxy`, `caBundle`, `killSwitch`, `featureFlags`, `retentionDays`, `supportContact`. Delivered by MDM/GPO/config management;
`hmd status`/`doctor` shows active sources. Managed settings **win over** user and project settings for the keys they set.

---

## 9. Extensibility and agentic capabilities (optional modules)

If the CLI includes an assistant/agent loop in the style of Claude Code, implement it as a **slice** (`session`) with the same hexagon; if not, skip to section 10. Components:

| Component | Design |
|---|---|
| **Model port** | `ModelPort` streaming events (text, tool calls, usage); provider adapters behind the port; cancellation, retries, budgets (`--max-turns`, `--max-budget`) |
| **Tools** | Registry of typed tools (JSON Schema/zod): read-only vs mutating classification; each tool declares required permission, path/command arguments and output size cap; results are **untrusted data** |
| **Permission engine** | Modes (`default`, `acceptEdits`, `plan`, `bypass` gated by policy); allow/deny rules with patterns (`Shell(git status *)`, `Read(./.env)`); deny wins; managed rules immutable; every decision logged |
| **Sandbox (optional)** | OS adapters (macOS Seatbelt, Linux bubblewrap/Landlock, Windows Job Objects/AppContainer or WSL) restricting filesystem/network for shell tools **(verify per OS)**; permissions are the policy layer, sandbox is containment |
| **Sessions** | JSONL event log, `--continue`, `--resume <id\|name>`, naming, fork, retention; resume rebuilds state from events |
| **Context management** | Token accounting, compaction (`/compact`), instruction files loaded on demand (scoped), file mentions, ignore files |
| **Skills** | Directories of instructions (`SKILL.md` + files) discovered from user/project scope, description-matched or invoked by slash command; trust and size budgets |
| **Hooks** | Lifecycle events (`SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`, `SessionEnd`, …) running commands/HTTP with JSON in/out; exit-code semantics; fail-closed option for policy hooks; trust-gated |
| **Plugins** | Versioned bundles (commands, skills, hooks, MCP servers) from allowlisted marketplaces, signed, with explicit trust prompts and managed allowlists |
| **MCP client** | stdio/HTTP/SSE servers; per-server auth tokens in the keystore; tool names prefixed by server; timeouts and output caps; managed allow/deny; no automatic trust of project-defined servers |
| **Background tasks** | Long commands/agents tracked in `/tasks` with logs and cancellation |
| **Headless/stream-json** | The same event bus serialized as NDJSON for IDEs and automation |

Security rules for this module: tool output and file contents may contain **prompt injection**: never let them change permissions, config or trust; require user approval for sensitive operations; redact secrets before sending to any model; log denials.

---

## 10. Security

### 10.1 Threat model (CLI-specific)

| Threat | Controls |
|---|---|
| **Malicious repository** (hooks, plugins, MCP, commands, config executing on open) | Workspace trust gate; managed allowlists; no auto-execution before trust; clear prompts showing what will run |
| **Terminal escape injection** (ANSI/OSC/CSI in server/LLM/file/filename content: title changes, fake prompts, clipboard writes, cursor tricks, hyperlink spoofing) | **Sanitize all untrusted text** before rendering (strip C0/C1 control chars and 7-bit ESC-prefixed CSI/OSC/DCS sequences; re-emit only sequences generated by the renderer); OSC 52 write-only and size-capped; OSC 8 URLs validated and shown |
| **Command injection** | Argument arrays, no shell interpolation of untrusted input; per-shell quoting adapters; `.cmd/.bat` special handling on Windows; allowlisted executables where possible |
| **Path traversal / symlink escape** | Containment checks with `realpath` against allowed roots; refuse writes outside; handle junctions/UNC |
| **Secret leakage** (logs, transcripts, telemetry, `ps`, env of child processes, crash dumps) | Redaction filters; never pass secrets in argv; scrub child-process environments (allowlist); mask token patterns in output; `doctor` warnings |
| **Token theft** | OS keystore, short-lived tokens, refresh rotation, `auth logout` revocation, file permissions 0600/ACLs |
| **Update hijack/downgrade** | Signed manifest + signed binaries, HTTPS, monotonic version, rollback protection, pinned update key distinct from the code-signing key |
| **Installer tampering (`curl \| sh`)** | Hash/signature verification, versioned URLs, offline installers, package-manager channels, documented checksum verification |
| **Supply chain** (npm/crates/go deps, build tooling) | Lockfiles, frozen installs, cooldown for new versions, SCA/SBOM, provenance attestations, hermetic CI, minimal dependency tree (`PLAYBOOK_SECURITY.md` §11) |
| **Privilege misuse** | Never require root/admin; user-writable install dir; no silent PATH/rc-file edits; no setuid |
| **Malicious plugin/MCP server** | Signing, allowlists, per-server permissions, timeouts/output caps, sandboxing where available |
| **Network interception** | TLS verification, proxy/CA support without disabling checks, no `--insecure` by default (if provided, loud warning and policy-gated) |
| **Local IPC abuse** (RPC socket) | Per-user permissions, random socket path in the runtime dir, auth token, schema validation |

### 10.2 Controls checklist

* Strict TypeScript, no `eval`/`new Function`, prototype-pollution-safe parsing, `child_process` only via `execFile`/`spawn` with argument arrays (language profiles in `PLAYBOOK_SECURITY.md` §16 for Node/Python/Go/Rust).
* All inbound data validated by schemas (API responses, event streams, config, plugin manifests, MCP messages) with size limits.
* No telemetry or crash upload without consent/policy; transcripts and logs are plaintext at rest unless encryption is enabled: documented.
* Secrets scanning of output before it leaves the process for shell tools or models (optional but recommended).
* `--dangerously-*` style bypass flags exist only if needed, are named loudly, disabled by managed policy and refused in untrusted workspaces.

### 10.3 Privacy

Opt-in telemetry with a clear schema, no file contents/prompts/paths in telemetry by default, retention limits for local transcripts, `purge` commands, data-subject requests handled by the backend, `doctor` explains what is stored where.

---

## 11. Performance and resource budgets

| Metric | Budget (starting points; calibrate) |
|---|---|
| `--version`/`--help` | ≤ 50 ms (compiled/fast path), ≤ 150 ms worst case on Node |
| Interactive cold start to first frame | ≤ 500 ms (Ink/Node-class), ≤ 150 ms (Rust/Go-class) |
| Time to first streamed token/line after sending | Dominated by network; client overhead ≤ 20 ms |
| Keypress to echo | ≤ 16–30 ms |
| Redraw cost during streaming | Bounded; ≤ 30–60 fps; no full-screen repaint per token |
| Idle CPU | ≈ 0% (no timers/animations when idle) |
| Memory | Idle ≤ ~150 MB (Ink/Node-class) / ≤ ~40 MB (Rust/Go-class); no growth across a 4-hour session; transcript windowed |
| Large outputs | Stream to terminal/file with backpressure; handle 100 MB piped input without buffering everything |
| Binary size | Track per release; compiled JS runtimes are large (tens to ~100+ MB); budgets and trends in CI |
| Install/update download | Delta or compressed artifacts; resumable |

Techniques: split the **entrypoint** so print/subcommand paths don't load Ink/React/markdown/highlighters; lazy-load heavy modules; precompiled bytecode/snapshots where the runtime supports them (verify stability and signing impact); avoid synchronous I/O in render paths; cache terminal capability results;
use streaming JSON/NDJSON parsers; avoid regex backtracking on untrusted input; keep dependency count low. Old/low-end machines: no GPU, small RAM budgets, slow disks (lazy file scanning, bounded watchers).

---

## 12. Distribution, install and update

### 12.1 Channels

| Channel | Notes |
|---|---|
| **Native installer script** (`curl -fsSL … \| sh`, PowerShell `irm … \| iex`) | Convenient but high-trust: HTTPS only, versioned URLs, published checksums/signatures, non-root install into a user dir, no silent rc edits; offer manual download + verify instructions |
| **Homebrew** (formula/cask), **WinGet**, **Scoop**, **Chocolatey** | Preferred by many users/admins; manifests generated by CI |
| **apt/dnf repos**, `.deb`/`.rpm` | Signed repos for Linux fleets |
| **npm** (`npm i -g`) with **optional platform dependencies** pulling the right native binary | Developers and CI; package provenance enabled |
| **MSI/pkg** | Managed deployment, silent install, per-machine policy |
| **Container image** | CI usage; non-root, minimal base |
| **GitHub Releases** | Archives + checksums + SBOM + provenance attestation |

### 12.2 Signing and trust

| OS | Requirement |
|---|---|
| Windows | **Authenticode** signing (key in HSM/cloud signing), timestamping, signed MSI; SmartScreen reputation planning |
| macOS | **Developer ID + hardened runtime + notarization** for CLI binaries (distribute as a .pkg or .dmg so the notarization ticket can be stapled; a ZIP can be notarized but never stapled, and a bare Mach-O binary cannot carry a stapled ticket, so it relies on Apple's online lookup at first launch) **(verify current Apple tooling)**; arm64 binaries must be (at least ad-hoc) signed to run |
| Linux | GPG-signed repositories; `cosign`/minisign signatures for archives; checksums |
| All | SLSA-style provenance, SBOM (CycloneDX/SPDX), reproducible builds as far as practical |

Signing happens in a **dedicated CI job** with isolated credentials and approval for releases; keys never on developer machines. Runtime toolchains can regress signing (a 2026 report mentions a Bun version producing macOS arm64 binaries killed at launch **(verify)**): add a **post-build smoke test on a clean machine** per OS/arch.

### 12.3 Updates

* **Channels**: `stable` (default), `latest`, `canary`; `hmd install <channel|version>` pins a specific version; staged rollout by cohort with halt on regression.
* **Manifest**: signed (Ed25519), lists version, channel, platform/arch/libc, URLs, hashes, size, min OS, release notes, `minSupportedVersion`, cohort; clients verify signature, hash and **monotonic version** before applying.
* **Mechanism**: background check (throttled, opt-out by policy), download to a staging dir, verify, **atomic swap**; on Windows swap on next launch or via a small helper because a running executable can't be replaced; keep the previous version for **rollback** (`hmd update --rollback`); never update while a long operation is running without consent.
* **Admin controls**: `updates.disabled`, `pinnedVersion`, `channel`, `mirrorUrl` (internal artifact server), update windows; air-gapped bundles with offline verification.
* **No update without trust**: unsigned, mismatched or downgrade manifests are rejected and reported.
* **Uninstall**: documented per channel; leaves user data unless `--purge`.

### 12.4 Support matrix

Publish `docs/support-matrix.md`: OS and minimum version, architectures, libc, terminals tested, shells, runtime requirements, known limitations (for example legacy consoles), end-of-support dates.

---

## 13. Observability and diagnostics

| Signal | Implementation |
|---|---|
| `hmd doctor` | Checks: version/channel, install method and path, PATH shadowing, runtime, auth state, API reachability (proxy/TLS/CA), config validity and sources, terminal capabilities report, shell detection, dir permissions, disk space, update status, plugin/MCP health; exit non-zero on failures; machine-readable `--json` |
| Debug logging | `--debug [categories]` (`api`, `mcp`, `render`, `permissions`, `updater`), written to a rotating file (path printed), redacted; never to stdout |
| Crash handling | Terminal guard + friendly message with log path and correlation ID; optional crash report upload (opt-in, scrubbed) |
| Diagnostics bundle | `hmd bug-report` (user-initiated): version, OS, terminal, caps, effective config (secrets removed), recent logs, last errors with correlation IDs |
| Metrics (opt-in) | Startup time, first-frame time, render stalls, API latency/error rates, realtime reconnects, update outcomes, command usage counts (no arguments/content) |
| Tracing | `X-Correlation-Id` per command/turn, printed on errors and included in logs |
| Server-side | Error rates by CLI version/OS, `upgrade-required` rates, update failures, auth failures |

---

## 14. Testing strategy

Follows `PLAYBOOK_TESTS.md`; adds a `cli` side:

```text
tests/<slice>/cli/{unit,integration,contract,e2e,perf,security}/
tests/_platform/{architecture,terminal-matrix,installer,updater,policy,realtime,security}/
```

| Level | Scope | Tooling (examples) |
|---|---|---|
| Unit | domain, use cases, permission engine, config resolver with fake ports | Vitest/Jest (or `cargo test`/`go test`) |
| ViewModel/TUI | State/commands; Ink rendering to a virtual terminal; keybinding handling | `ink-testing-library`, snapshot of rendered frames |
| Adapter integration | SDK vs mock server, SSE/WebSocket conformance (reconnect, resume, duplicates, slow consumer), keystore adapters per OS, config layering, file locking, shell adapters | local test servers, OS runners |
| Contract | OpenAPI/route-table parity (commands ↔ endpoints), exit-code mapping, `stream-json` schema, JSON envelope, policy schema | schema tests, golden files |
| Golden output | `--help`, tables, JSON, error messages (width variants, no-color, ASCII fallback) | snapshot tests |
| **PTY e2e** | Real terminal sessions: spawn the binary in a pseudo-terminal (ConPTY on Windows), type keys, assert screen content via a headless terminal emulator; permission dialogs, interrupts, resize, paste | `node-pty`/`pexpect`-style drivers + xterm-headless |
| Cross-OS matrix | Ubuntu (glibc), Alpine (musl), macOS (x64/arm64), Windows (x64/arm64) | CI runners |
| Terminal matrix | Windows Terminal, conhost, PowerShell 5.1/7, cmd, Git Bash, WSL; macOS Terminal.app, iTerm2; GNOME Terminal/Konsole/xterm; Kitty/WezTerm/Alacritty/Ghostty; tmux/screen/zellij; SSH; VS Code terminal; CI (no TTY) | scripted + manual smoke |
| Encoding/locale | UTF-8, CJK/emoji widths, RTL, non-UTF-8 locales, Windows code pages | fixtures |
| Paths/filesystem | Spaces, Unicode, UNC, drive letters, long paths, symlinks/junctions, permissions, read-only FS | fixtures |
| Signals/lifecycle | SIGINT/SIGTERM/SIGHUP, Windows Ctrl+C/Break, crash → terminal restored, piped `EPIPE` | process tests |
| Concurrency | Multiple instances: config/session locking, no corruption | stress tests |
| Security | Escape-sequence injection corpus, path traversal/symlink escape, command injection (incl. `.cmd`), secrets-in-logs scans, update tamper/downgrade, workspace-trust bypass attempts, fuzzing of parsers (SSE, NDJSON, config, manifests) | custom + fuzzers |
| Performance | Startup, first frame, render throughput, memory soak, large pipe | benchmark harness, regression thresholds |
| Install/upgrade | Clean VM install, upgrade N-1 → N, rollback, uninstall, PATH, per-channel | VM images per OS |
| Policy | Managed settings per OS (registry/plist/JSON), locked keys, kill switch, update pinning | scripted |
| Accessibility | `--accessible`, `NO_COLOR`, screen-reader-friendly output, ASCII fallback | snapshot + manual |
| Authorization parity | Each command maps to a route/permission; authz matrix rows exist | `PLAYBOOK_TESTS.md` §8 |

Rules: no production data or real tenants; deterministic PTY tests (wait on screen conditions, never sleeps); flaky tests quarantined with owner/expiry; every command has contract + golden tests; every exit code has a test.

### 14.1 TDD loop with Claude Code

The TDD runner (`PLAYBOOK_CLAUDE_CODE.md` §10) detects the CLI package and runs unit/contract tests per slice; PTY/e2e suites run in CI; roles stay separate (test-writer edits tests, implementer edits source). Golden files are reviewed like code.

---

## 15. Build and release engineering

| Stage | Gate |
|---|---|
| Pre-commit | Format, lint, typecheck, secrets scan |
| PR | Unit/ViewModel/contract/golden tests, architecture rules, SAST/SCA, command ↔ route-table parity, size/startup smoke on Linux |
| Main | Three-OS matrix (build + integration + PTY smoke), SBOM, license report |
| Nightly | Full PTY/terminal matrix, perf suite, soak, installer/upgrade, fuzzing, dependency refresh |
| Release candidate | All above + signing (isolated job), notarization, update manifest dry run, install tests on clean VMs, evidence bundle |
| Release | Staged rollout (canary → latest → stable), health gates, auto-halt |
| Post-release | Crash/update/auth metrics, rollback plan (higher version + kill switch) |

Build rules: lockfile + frozen install; pinned runtime/toolchain versions; per-target builds (linux x64/arm64 × glibc/musl, macOS x64/arm64, Windows x64/arm64); reproducible archives; release smoke test executes the produced binary (`--version`, `doctor`, a PTY hello) on each target;
generate checksums, SBOM, provenance; publish manifests to every channel from one pipeline.

---

## 16. Governance and compliance

* **Ownership**: CODEOWNERS for `src/platform/**`, `src/infrastructure/{updater,config,auth}/**`, permission engine, `contracts/**`, packaging, signing workflows, `docs/permissions-register.md`; security review for those paths.
* **Documents**: ADRs (D1–D15…), support matrix, threat model, data-flow diagrams, admin guide (managed settings per OS), runbooks, command reference (generated).
* **Compliance mapping** (`PLAYBOOK_AUDIT.md`): secure SDLC (CI gates, reviews, analysis), change management (release approvals, staged rollout), cryptography/key custody (signing and update keys), vulnerability management (SBOM, scanning, SLAs), logging (telemetry/audit), vendor management (runtime, dependencies, crash/telemetry services), availability (kill switch, rollback), privacy (telemetry policy, retention, purge).
* **CRA**: a distributed CLI is a product with digital elements for EU purposes; reporting obligations for manufacturers apply from 11 September 2026 and most other obligations from 11 December 2027 **(confirm applicability with counsel)**; maintain a vulnerability disclosure policy, SBOM and a security-update support period.
* **Runbooks**: broken release/rollback, signing key compromise, update server outage, malicious plugin/MCP server report, auth/IdP outage, terminal-corruption bug, Windows AV false-positive wave, notarization failure.
* **Quality KPIs**: crash-free sessions, startup p50/p95 per OS, update success rate, command error rate by exit code, support tickets per release, defect escape rate.

---

## 17. Working with Claude Code on this project

Per `PLAYBOOK_CLAUDE_CODE.md`:

* A path-scoped rule for `apps/cli/**` carrying the layer rules, the stdout/stderr contract, exit codes, "no `process.platform` outside `platform/`", terminal-sanitization and trust rules (linked, not `@`-imported).
* **Role policy**: the integrator owns `packaging/**`, signing/release workflows, dependency baselines and generated contracts/route table; the security reviewer checks permission engine, updater, auth and sanitizer changes; hooks/settings stay human-owned.
* **Skills**: `new-cli-command` (command + route-table entry + use case + tests + help golden), `new-slash-command`, `new-tui-view` (ViewModel + View + frame snapshot), `new-policy-key` (schema + per-OS delivery + UI/doctor display + test), `cli-perf-check` (startup/render budgets).
* **Specs** for each command list: usage, flags, positional args, output schemas, exit codes, permissions/route, platform notes, TTY vs piped behavior, and test levels per AC.

---

## 18. Anti-patterns (reject in review)

| Anti-pattern | Why it fails |
|---|---|
| Business logic in the TUI/Ink components or in argv handlers | Untestable, not scriptable |
| Printing UI chrome, spinners or color to stdout when piped | Breaks scripts |
| Prompting in non-TTY contexts | Hangs CI |
| Writing raw server/LLM/file text to the terminal | Escape-sequence injection |
| Leaving the terminal in raw/alt-screen state after errors | Unusable shell |
| `process.platform` checks scattered through code | Platform drift |
| Building shell command strings from untrusted input; ignoring `.cmd/.bat` quirks on Windows | Command injection |
| Assuming Git Bash/WSL/Unix tools on Windows | Breaks native Windows users |
| Hard-coding terminal features (OSC 8, 2026, kitty keys) without negotiation/fallback | Garbage output in other terminals |
| Full-screen repaint per streamed token | Flicker and CPU burn |
| Alternate-screen-only UI by default | Loses scrollback/copy-paste |
| Auto-executing hooks/plugins/MCP from a freshly cloned repo | Remote code execution |
| Tokens/secrets in config files, argv, logs or transcripts | Credential leaks |
| `curl \| sh` installer without verification, or editing shell rc files silently | Supply-chain risk |
| Update that can downgrade or applies unsigned artifacts; overwriting a running `.exe` | Takeover/corruption |
| Heavy imports at startup for `--version`/`--help` | Slow CLI |
| Hand-written API paths/DTOs and commands | Contract drift |
| Undocumented/unstable exit codes and JSON shapes | Breaks automation |
| Testing only on the developer's OS/terminal | Field failures on others |
| Telemetry on by default or without redaction | Privacy/enterprise rejection |

---

## 19. Definition of done and checklists

### 19.1 Command/feature DoD

- [ ] Spec lists usage, flags, args, output schemas, exit codes, permissions/route, TTY vs piped behavior, platform notes, and test levels per AC.
- [ ] Logic lives in use cases; adapters thin; works headless; `--json`/`stream-json` schema and golden tests exist.
- [ ] Errors map to the shared taxonomy and documented exit codes; `requiredPermission` and correlation ID surfaced.
- [ ] No prompts when not a TTY; `--yes`/`--dry-run` for destructive actions; `Ctrl+C` handled; terminal restored.
- [ ] Output sanitized; widths/Unicode handled; `NO_COLOR`/`--accessible` verified.
- [ ] Works on Linux, macOS and Windows (paths, shells, signals); covered by CI matrix.
- [ ] Policy-aware (managed settings, kill switch); telemetry opt-in and redacted.
- [ ] Help/completions/docs generated; startup budget unaffected (lazy imports verified).

### 19.2 Release checklist

- [ ] Frozen builds for every target; checksums, SBOM and provenance generated.
- [ ] Unit/contract/golden/PTY/security/perf suites green on the OS/terminal matrix.
- [ ] Binaries signed (Authenticode/Developer ID+notarization/GPG), post-build smoke tests pass on clean machines.
- [ ] Update manifest signed; rollback tested; staged rollout and halt criteria configured.
- [ ] Install/upgrade/uninstall tested per channel; admin guide and support matrix updated.
- [ ] Canary soak complete; crash/update/auth metrics healthy.

### 19.3 Architecture acceptance checklist

- [ ] Headless core with ports; TUI, print, subcommands, RPC as separate adapters over the same use cases and event bus.
- [ ] Dependency rules enforced in CI; `platform/` is the only OS-specific code.
- [ ] stdout/stderr, exit-code and output-schema contracts documented and tested.
- [ ] Terminal capability negotiation with fallbacks; terminal guard; sanitization of untrusted text.
- [ ] SDK-generated commands mirroring the URL contract; auth via device code/PKCE; tokens in the keystore.
- [ ] Config layers with managed policy, workspace trust, schema validation; state locking for multi-instance use.
- [ ] Realtime optional with the shared state machine; long operations with `--wait`.
- [ ] Extensibility (skills/hooks/plugins/MCP) trust-gated and policy-controlled (if included).
- [ ] Single-binary packaging per OS/arch, signed and notarized, auto-update with rollback and admin controls.
- [ ] `doctor`, debug logging, bug-report bundle, opt-in telemetry.
- [ ] Three-OS CI, PTY e2e, terminal matrix, installer/updater and security suites.

---

## Appendix A — Directory structure (TypeScript variant)

```text
apps/cli/
├── bin/hmd.ts                      # fast path: --version/--help; lazy-loads the rest
├── src/
│   ├── app/                        # composition root, mode router (interactive | print | subcommand | serve)
│   ├── shared-kernel/              # Result, ids, errors, time, branded types
│   ├── contracts/                  # DTOs, error codes, exit codes, event schemas (stream-json), policy schema, route table
│   ├── platform/                   # ONLY OS-specific code
│   │   ├── common/  linux/  macos/  windows/
│   │   │   paths  signals  shell  process-tree  keystore  clipboard  opener  terminal-modes  fs-lock  console-encoding
│   ├── infrastructure/
│   │   ├── sdk/ (adapter over packages/sdk)   realtime/ (sse, websocket, polling)   storage/ (sessions, cache, locks)
│   │   ├── config/  auth/  telemetry/  updater/  mcp/  plugins/  sandbox/  terminal/ (caps, renderer helpers, sanitizer)
│   ├── features/<slice>/
│   │   ├── domain/ application/ adapters/{in/{cli,slash,tui,rpc}, out}/ contracts/ viewmodels/ views/
│   └── ui-kit/                     # Ink components, themes, markdown renderer, table, diff view, dialogs
├── packaging/{npm,homebrew,winget,scoop,deb,rpm,msi,pkg,docker}/
packages/{sdk,core,viewmodels}/     # shared with other clients
tests/<slice>/cli/{unit,integration,contract,e2e,perf,security}/    tests/_platform/{architecture,terminal-matrix,installer,updater,policy,realtime,security}/
docs/{adr,support-matrix.md,admin-guide.md,runbooks/,permissions-register.md}
```

## Appendix B — Fast-path entrypoint and mode router (sketch)

```ts
// bin/hmd.ts — keep imports minimal so --version/--help stay instant
const argv = process.argv.slice(2);
if (argv[0] === '--version' || argv[0] === '-v') { process.stdout.write(`${VERSION}\n`); process.exit(0); }

const { run } = await import('../src/app/main.js');                  // lazy: parser, container, adapters
process.exitCode = await run(argv, {
  stdin: process.stdin, stdout: process.stdout, stderr: process.stderr,
  env: process.env, platform: process.platform, isTTY: !!process.stdout.isTTY && !!process.stdin.isTTY,
});
```

```ts
// src/app/main.ts
export async function run(argv: string[], io: Io): Promise<number> {
  const container = await buildContainer(io);                        // wires ports → adapters per OS
  const cmd = parse(argv, container.commandRegistry);                // generated from route table + hand-written commands
  switch (selectMode(cmd, io)) {                                     // interactive | print | subcommand | serve
    case 'interactive': return (await import('./modes/interactive.js')).start(container, cmd);   // loads Ink only here
    case 'print':       return (await import('./modes/print.js')).start(container, cmd);
    case 'serve':       return (await import('./modes/serve.js')).start(container, cmd);
    default:            return (await import('./modes/subcommand.js')).start(container, cmd);
  }
}
```

## Appendix C — Stream-JSON event envelope (v1)

```json
{"v":1,"type":"session.started","ts":"2026-11-10T12:00:00Z","sessionId":"s_01","correlationId":"c_01","payload":{"cwd":"/work/app","version":"1.8.0","mode":"print"}}
{"v":1,"type":"message.delta","sessionId":"s_01","payload":{"role":"assistant","text":"Analyzing…"}}
{"v":1,"type":"tool.requested","sessionId":"s_01","payload":{"id":"t_1","tool":"Read","input":{"path":"src/a.ts"}}}
{"v":1,"type":"permission.requested","sessionId":"s_01","payload":{"id":"p_1","tool":"Shell","summary":"git status","options":["allow_once","allow_always","deny"]}}
{"v":1,"type":"tool.result","sessionId":"s_01","payload":{"id":"t_1","ok":true,"truncated":false}}
{"v":1,"type":"progress","sessionId":"s_01","payload":{"phase":"running","percent":40}}
{"v":1,"type":"result","sessionId":"s_01","payload":{"ok":true,"exitCode":0,"usage":{"turns":2}}}
{"v":1,"type":"error","sessionId":"s_01","payload":{"code":"FORBIDDEN","message":"…","requiredPermission":"funnels.leads.update","correlationId":"c_01"}}
```

Rules: one JSON object per line on stdout; unknown event types are ignored by consumers; additive changes only within `v`; breaking changes bump `v`; partial-message and hook events are opt-in flags.

## Appendix D — Output sanitizer (sketch; prefer a vetted library plus this control-character pass)

```ts
const CSI = /\u001b\[[0-?]*[ -/]*[@-~]/g;
const OSC = /\u001b\][^\u0007\u001b]*(?:\u0007|\u001b\\)/g;                 // includes OSC 8/52/0 etc.
const DCS_PM_APC_SOS = /\u001b[PX^_][\s\S]*?(?:\u001b\\)/g;
const OTHER_ESC = /\u001b[@-Z\\-_]/g;
const CONTROL = /[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g;                   // keep \n \t; strip \r (line overwrite)

export const toSafeTerminalText = (s: string): string =>
  s.replace(OSC, '').replace(DCS_PM_APC_SOS, '').replace(CSI, '').replace(OTHER_ESC, '').replace(CONTROL, '');
// The renderer re-adds its own styling/hyperlinks AFTER sanitizing; URLs are validated (scheme allowlist) and displayed.
```

## Appendix E — Device-code sign-in (sketch)

```ts
export async function deviceLogin(idp: IdpPort, ui: UiPort, clock: ClockPort): Promise<Tokens> {
  const d = await idp.deviceAuthorize({ scope: 'openid offline_access hmd.api' });          // POST …/device_authorization
  ui.info(`Open ${d.verificationUri} and enter code: ${d.userCode}`);
  let interval = d.interval ?? 5;
  const deadline = clock.now() + d.expiresIn * 1000;
  while (clock.now() < deadline) {
    await sleep(interval * 1000);
    const r = await idp.pollToken(d.deviceCode);
    if (r.ok) return r.tokens;                                                              // store in SecretStorePort, never print
    if (r.error === 'slow_down') interval += 5;
    else if (r.error !== 'authorization_pending') throw new AuthError(r.error);             // access_denied, expired_token, …
  }
  throw new AuthError('expired_token');
}
```

## Appendix F — Config precedence resolver (sketch)

```ts
export function resolveConfig(layers: { managed: Cfg; flags: Cfg; env: Cfg; projectLocal: Cfg; project: Cfg; user: Cfg; defaults: Cfg }): EffectiveConfig {
  const order = [layers.defaults, layers.user, layers.project, layers.projectLocal, layers.env, layers.flags] as const;   // low → high
  const merged = mergeWithSources(order);                    // records the source layer of every key
  return applyManagedLocks(merged, layers.managed);          // managed values win and mark keys as locked
}
```

## Appendix G — Managed settings example (JSON; same keys via registry/plist)

```json
{
  "apiBaseUrl": "https://acme.example.com",
  "idp": { "issuer": "https://login.acme.example/", "clientId": "hmd-cli" },
  "updates": { "channel": "stable", "disabled": false, "mirrorUrl": "https://mirror.acme.example/hmd" },
  "telemetry": "errors",
  "permissions": { "deny": ["Shell(curl * | sh*)", "Read(./.env)"] },
  "allowManagedHooksOnly": true,
  "allowedMcpServers": ["acme-internal"],
  "proxy": { "mode": "system" },
  "killSwitch": false
}
```

## Appendix H — Update manifest example

```json
{
  "v": 1, "channel": "stable", "version": "1.8.2", "minSupportedVersion": "1.6.0", "rollout": { "cohort": "stable", "percent": 25 },
  "artifacts": [
    { "os": "linux", "arch": "x64", "libc": "glibc", "url": "https://dl.example.com/hmd/1.8.2/hmd-linux-x64.tar.gz", "sha256": "…", "size": 62914560 },
    { "os": "darwin", "arch": "arm64", "url": "https://dl.example.com/hmd/1.8.2/hmd-darwin-arm64.zip", "sha256": "…", "size": 60817408 },
    { "os": "win32", "arch": "x64", "url": "https://dl.example.com/hmd/1.8.2/hmd-win32-x64.zip", "sha256": "…", "size": 66060288 }
  ],
  "notes": "https://example.com/hmd/changelog#1.8.2",
  "signature": "<ed25519 over canonical JSON>"
}
```

## Appendix I — CI matrix (sketch)

```yaml
strategy:
  matrix:
    include:
      - { os: ubuntu-latest,  target: linux-x64-glibc }
      - { os: ubuntu-latest,  target: linux-x64-musl, container: alpine }   # musl artifact tested in Alpine
      - { os: ubuntu-24.04-arm, target: linux-arm64-glibc }
      - { os: macos-13,       target: darwin-x64 }
      - { os: macos-latest,   target: darwin-arm64 }
      - { os: windows-latest, target: win32-x64 }
      - { os: windows-11-arm, target: win32-arm64 }
steps: [checkout, setup-runtime (pinned), frozen-install, lint+typecheck, unit+contract+golden, build-binary, smoke(--version, doctor, PTY hello), upload-artifact]
```

## Appendix J — Default keybindings (data-driven; per-terminal overrides)

| Action | Default | Notes |
|---|---|---|
| Submit | `Enter` | |
| Newline | `Shift+Enter` / `Alt+Enter` / `\` + `Enter` | Shift+Enter needs enhanced keyboard modes in some terminals/ConPTY |
| Cancel current operation | `Esc` / first `Ctrl+C` | |
| Exit | second `Ctrl+C` / `Ctrl+D` on empty input | |
| History search | `Ctrl+R` | |
| Slash palette | `/` at start of input | |
| File mention | `@` | respects ignore files |
| Clear screen | `Ctrl+L` | |
| Toggle detail view | `Ctrl+O` | |
| Scroll (fullscreen mode) | `PageUp/PageDown`, mouse wheel (opt-in) | |
