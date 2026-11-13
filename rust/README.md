# `<app>` — Rust backend skeleton

> Companion repo: [beautifulrust](https://github.com/Facta-Dev/beautifulrust) —
> the Agent Skills and linter config for this stack.

An enterprise-grade axum 0.8 + sqlx 0.8 workspace template. Every `<app>` and
`<scope>` token is a **literal placeholder** — it is NOT a valid crate name and
the workspace will not parse until you rename it. See [How to apply](#how-to-apply).

## Layout

```
crates/
  <app>-core/     pure domain types (no I/O) — includes the identity types
  <app>-ports/    ports: domain traits + Pg adapters + mock impls
  <app>-widgets/  slice: widget CRUD (depends on ports, not adapters)
  <app>-auth/     identity services: signup, sessions, password flows, OAuth
  <app>-adminusers/  admin roster: paginated users + rank-gated patches
  <app>-warden/   auth infrastructure (JWT + RBAC + password hashing)
  <app>-mailer/   email infra adapter (SMTP sender, console sender, offline guard)
  <app>-loom/     realtime hub + outbox worker + janitor ("worker" bin)
  <app>-api/      axum HTTP server (composition root: AppState + router)
  <app>-sdk/      client SDK
  <app>-cli/      CLI entrypoint
migrations/       paired up/down SQL (embedded via sqlx::migrate!, applied
                  by the one-shot `migrate` subcommand)
.github/workflows/ci.yml   fmt / clippy / test / audit / eval
deny.toml         cargo-deny: advisories + bans + licenses + sources
rustfmt.toml      shared format config
justfile          local + CI task runner (`just ci` mirrors the gate)
Dockerfile        multi-stage build (builder + distroless nonroot runtime)
docker-compose.yml  postgres + migrations + api + worker local stack
```

## Container stack

```sh
docker compose up --build      # postgres + migrations + api + worker
docker compose down            # stop (volume persists)
docker compose down -v         # stop + drop the postgres volume
```

The Dockerfile is a multi-stage build: `rust:1-bookworm` builder (no database
is needed — all sqlx queries are runtime queries), then a
`gcr.io/distroless/cc-debian12:nonroot` runtime shipping two binaries:
`/app/server` (`<app>-api`) and `/app/worker` (`<app>-worker`), running as
UID/GID 65532 with no shell. `[profile.release]` applies thin LTO, symbol
stripping, and `panic=abort` to the shipped binaries. A one-shot `migrations`
compose service (go release-step parity) applies the schema before
api/worker boot. For a fully static musl build, `Dockerfile.static` builds
the same two binaries against `distroless/static`.

## Prerequisites

- Rust **1.93.1** (pinned in `rust-toolchain.toml`; MSRV 1.88+).
- `just` (`cargo install just`).
- Postgres 15+ for `just migrate` / integration testing.
- Optional supply-chain tools: `cargo install cargo-deny cargo-audit cargo-machete`.

## The gate

```sh
just ci          # fmt-check + lint + check + test (+ the native contract evals)
just audit       # cargo-audit + cargo-deny + cargo-machete (optional side gate)
just migrate     # apply migrations to the dev database (one-shot subcommand)
```

CI (`.github/workflows/ci.yml`) runs the same gate as five jobs — `fmt`,
`clippy`, `test`, `eval`, `audit` — with a real Postgres service for the
`test` job, where the one-shot `migrate` subcommand applies the schema
before `nextest` runs.

## How to apply

1. **Rename** every `<app>` and `<scope>` placeholder to your project's name.
   Prefer the script — it rewrites file contents and crate directory names in
   one pass:

   ```sh
   bash scripts/rename.sh myapp myscope
   ```

   The manual equivalent, if you need to see each edit:

   ```sh
   # from the repo root, after copying this skeleton out:
   find . -type f -not -path './target/*' \
     -exec sed -i 's/<app>/myapp/g; s/<scope>/myscope/g' {} +
   ```

   Also rename the crate directories (`crates/<app>-core` -> `crates/myapp-core`,
   etc.) and the `[workspace] members` + `[workspace.dependencies]` entries in
   `Cargo.toml` to match.

2. **Generate and commit `Cargo.lock`**. The workspace cannot resolve a
   lockfile while `<app>` is still a placeholder (it is not a valid crate
   name), so no `Cargo.lock` is shipped in the template. Immediately after the
   rename:

   ```sh
   cargo generate-lockfile
   git add Cargo.lock
   git commit -m "chore: commit Cargo.lock post-rename"
   ```

   `Cargo.lock` is committed for binaries/applications (reproducible builds).
   Do NOT gitignore it.

3. **Bring the schema up** once dependencies resolve (or `docker compose up` —
   the one-shot `migrations` service applies it):

   ```sh
   export <APP>_DATABASE_URL=postgres://myapp:myapp@localhost:5432/myapp
   just migrate            # one-shot subcommand, no sqlx-cli needed
   ```

4. **Copy `.env.example` to `.env`** and fill in real values. `.env` is
   gitignored; never commit real secrets. Tighten permissions on `.env` and
   any `credentials/` (`chmod 600`).

5. **Re-pin the toolchain** in `rust-toolchain.toml` (single source of truth —
   CI inherits it via `rustup show`) if you move off 1.93.1.

## Working with agents

`AGENTS.md` is the root instruction file; `CLAUDE.md` is a symlink to it, so
Claude Code, Codex, Cursor, Gemini and the rest read the same rules. `MEMORY.md`
carries the product premise, actors, crate→table map and gates.

The deep contracts are next to this README; read the one you need instead of
importing all of them:

| File | Covers |
|---|---|
| `PLAYBOOK_BACKEND.md` | crates, dependency edges, ports/adapters, SQL, workers, realtime |
| `PLAYBOOK_URLS.md` | route naming, namespaces, error envelope, streaming |
| `PLAYBOOK_TESTS.md` | test pyramid, nextest, contract evals, audit evidence |
| `PLAYBOOK_SECURITY.md` | threat model, authn/authz, input, secrets, supply chain |
| `PLAYBOOK_AUDIT.md` | self-hosting a SOC 2 / ISO 27001 audit program |
| `PLAYBOOK_CLAUDE_CODE.md` | the `.claude` kit, SDD + TDD loop, multi-session governance |

The kit ships 11 agents (`backend`, `core`, `security`, `testing`), 9 skills
and 9 matching `/commands` (`add-handler`, `add-module`, `ownership-author`,
`unsafe-guard`, `review`, `spec`, `plan`, `learn`, `ship`), a `PreToolUse`
Bash guard, and offline contract evals.

```sh
bash scripts/lint-agents.sh        # frontmatter + body sections
bash scripts/lint-skills.sh
bash scripts/lint-commands.sh
cargo test --manifest-path evals/Cargo.toml   # 9 contract tests, offline
bash scripts/verify-template.sh    # staged renamed copy + full gate
```

Before instantiation, `lint-skills.sh` and `lint-commands.sh` reject the
intentional `<app>` placeholders; set `ALLOW_PLACEHOLDERS=1` to lint the
template itself.

## Supply chain

- `deny.toml` gates advisories (`yanked = "deny"`), duplicate crates, license
  allowlist, and crate sources. Run `cargo deny check` or `just audit`.
- `cargo audit` scans the advisory database.
- `cargo machete --workspace` flags unused dependencies.
- The CI `audit` job runs all three on every push/PR.

## Offline guard (INV-T7)

`<APP>_OFFLINE=true` boots the server with `NoopMailer` selected
unconditionally — `<APP>_MAILER_ENDPOINT` is never read and no
`reqwest::Client` (socket) is constructed. `crates/<app>-mailer`'s
`mailer_from_config(offline, endpoint)` is the selector: `offline` wins over
everything; otherwise `Some(endpoint)` picks `ProviderMailer`, `None` falls
back to `NoopMailer`. `AppState::from_env` (`crates/<app>-api/src/state.rs`)
is the only call site. No new trait was introduced — this is a plain
selection function over the existing `NoopMailer` / `ProviderMailer`
Null-Object pair, mirroring the backend-python template's `config.offline`
guard (`state.py`'s `AppState.from_env`).

To exercise `ProviderMailer` against a mock server in adapter tests, set
`<APP>_MAILER_ENDPOINT` to the mock's URL (with `<APP>_OFFLINE=false`, the
default) and call `mailer_from_config` / `ProviderMailer::new` directly — see
`crates/<app>-mailer/src/lib.rs`'s `#[cfg(test)] mod tests` for examples.

## ponytail stubs

Deliberate stubs are marked with `// ponytail:` comments naming the ceiling
and upgrade path. They are real, compiling, offline-guarded seams — not silent
no-ops in the runtime path.
