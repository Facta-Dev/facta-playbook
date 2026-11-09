#!/usr/bin/env bash
# verify-template.sh — smoke-test the backend-rust skeleton end to end.
#
# The skeleton ships with literal <app>/<APP>/<scope> placeholders that make a
# committed Cargo.lock impossible (<app> is not a valid crate name and the
# workspace will not parse) and block `cargo build`. This script stages a
# throwaway renamed copy, runs the full CI gate against it, then discards the
# copy. The real working tree is never mutated. Exits nonzero on any failure.
# Idempotent — safe to re-run.
#
# Usage: bash scripts/verify-template.sh
#
# What it does:
#   1. Copy the skeleton to a temp dir (excluding target/.git).
#   2. Substitute placeholders via scripts/rename.sh (example: examplewidget / exampleorg).
#   3. Fail fast if any literal placeholder survives the rename.
#   4. cargo generate-lockfile  (fresh resolve — no Cargo.lock in the template by design).
#   5. cargo build + clippy + nextest  (runtime sqlx queries: no DB needed).
#   6. Throwaway Postgres + `<app>-api migrate` (one-shot schema runner asserts
#      the DDL applies against a live server).
#   7. Stop the container + remove the temp dir.
#
# NOTE on sqlx: all queries are RUNTIME queries (no compile-time query!/query_as!
# macros, no committed .sqlx/ cache) — the build needs no database, and the
# migrations ride inside the binary via the embedded sqlx::migrate! migrator.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Example substitution values — must be valid lowercase crate/url segments.
EXAMPLE_APP="examplewidget"
EXAMPLE_SCOPE="exampleorg"

# Throwaway Postgres credentials (live only for the duration of this script).
PGUSER="$EXAMPLE_APP"
PGPASSWORD="$EXAMPLE_APP"
PGDB="$EXAMPLE_APP"
PG_IMAGE="postgres:17-alpine"
PG_CID=""

TMP="$(mktemp -d -t verify-template.XXXXXX)"
cleanup() {
  if [[ -n "$PG_CID" ]]; then
    docker rm -f "$PG_CID" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP"
}
trap cleanup EXIT

# --- prerequisites -----------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  echo "verify-template: cargo not found on PATH. Install Rust via https://rustup.rs first." >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "verify-template: docker not found on PATH. Required to run the throwaway Postgres." >&2
  exit 1
fi

echo "verify-template: staging copy -> $TMP"
# Portable copy (no rsync dependency); excludes heavy/regenerable dirs for speed.
(
  cd "$ROOT_DIR"
  tar \
    --exclude=./target \
    --exclude=./.git \
    --exclude='./.sqlx' \
    --exclude='./**/target' \
    -cf - .
) | (
  cd "$TMP"
  tar -xf -
)

echo "verify-template: renaming placeholders (${EXAMPLE_APP} / ${EXAMPLE_SCOPE})"
bash "$TMP/scripts/rename.sh" "$EXAMPLE_APP" "$EXAMPLE_SCOPE"

# Guard: no literal placeholder may survive the rename. The exclusion set
# mirrors leak-guard.sh's placeholder allowlist (and rename.sh's substitution
# scope): evals/, fixtures/, and the pure-bash linter/leak-guard/installer
# scripts carry <app>/<APP>/<scope> as PATTERN TOKENS (scanner logic + the
# frozen invalid-leak fixture), not substitutable placeholders, so rename.sh
# deliberately skips them and this guard must too — otherwise it false-fails
# on a clean rename.
leftover="$(grep -rIl -e '<app>' -e '<APP>' -e '<scope>' "$TMP" \
  --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
  --exclude-dir=fixtures --exclude-dir=evals \
  --exclude='*.lock' --exclude='*.png' --exclude='*.jpg' --exclude='*.ico' \
  --exclude='*.woff' --exclude='*.woff2' --exclude='*.ttf' \
  --exclude='lint-skills.sh' --exclude='lint-commands.sh' \
  --exclude='leak-guard.sh' --exclude='install-claude-runtime.sh' \
  2>/dev/null || true)"
if [ -n "$leftover" ]; then
  echo "verify-template: ERROR — unresolved placeholders remain after rename:" >&2
  printf '  %s\n' "$leftover" >&2
  exit 1
fi

# --- agent-frontmatter lint (post-rename, pre-build) ------------------------
# Validate the shipped .claude/agents/ layer in the STAGED, renamed copy via
# the standalone, schema-driven scripts/lint-agents.sh (extracted from the
# former inline lint_agents() block here). Schema-driven from
# .claude/schemas/agents.schema.md so the linter and the schema doc cannot
# drift; fail-closed. Template-local and self-contained: no dependency on the
# repo's divisions.json (the repo's 27 divisions are a different namespace).
echo "verify-template: linting shipped .claude/agents/ (post-rename)"
( cd "$TMP" && bash scripts/lint-agents.sh ) || {
  echo "verify-template: ERROR — lint-agents.sh failed" >&2
  exit 1
}

# --- skills + commands lint (post-rename, pre-build) ------------------------
# Validate the shipped .claude/skills/ + .claude/commands/ layers in the STAGED,
# renamed copy. The linters parse .claude/schemas/*.schema.md for required
# frontmatter fields, required body sections, allowed divisions, and the
# documented failure modes — so the linter and the schema doc cannot drift.
# Strict mode (no ALLOW_PLACEHOLDERS): after rename, 0 residual <app>/<APP>/<scope>.
echo "verify-template: linting shipped .claude/skills/ (post-rename, strict)"
( cd "$TMP" && bash scripts/lint-skills.sh ) || {
  echo "verify-template: ERROR — lint-skills.sh failed" >&2
  exit 1
}

echo "verify-template: linting shipped .claude/commands/ (post-rename, strict)"
( cd "$TMP" && bash scripts/lint-commands.sh ) || {
  echo "verify-template: ERROR — lint-commands.sh failed" >&2
  exit 1
}

# --- native contract evals (post-rename, pre-build) --------------------------
# Run the native `cargo test` contract suite over the renamed .claude layer.
# evals/Cargo.toml is a standalone package (NOT a workspace member) with zero
# external dependencies, so this runs offline with no network. The validator
# parses frontmatter + body sections + division enum + placeholder leak; the
# fixtures exercise distinct failure modes (missing key, bad division, missing
# body section, literal-placeholder leak). Mirrors the gtm valid+invalid
# fixture pair + regex-match-on-failure pattern in the stack's native runner.
echo "verify-template: test:contracts (cargo test --manifest-path evals/Cargo.toml)"
( cd "$TMP" && cargo test --manifest-path evals/Cargo.toml ) || {
  echo "verify-template: ERROR — test:contracts failed" >&2
  exit 1
}

# --- installer syntax check (post-rename, pre-build) ------------------------
# `bash -n` parses the installer without executing it (no target runtime is
# available in verify). The installer is pure-bash (no node/npm); a syntax
# error here is a build break before the stack pipeline runs.
echo "verify-template: bash -n scripts/install-claude-runtime.sh (syntax check)"
bash -n "$TMP/scripts/install-claude-runtime.sh" || {
  echo "verify-template: ERROR — install-claude-runtime.sh syntax error" >&2
  exit 1
}

# --- hooks lane (post-rename, pre-build) -------------------------------------
# Hooks are never executed in this gate (no live Claude Code session); syntax
# only. `bash -n` every scripts/hooks/*.sh, and assert .claude/settings.json
# exists and wires a PreToolUse hook (grep only — no jq/node, INV-T15).
echo "verify-template: bash -n scripts/hooks/*.sh (syntax check)"
shopt -s nullglob
hook_files=("$TMP"/scripts/hooks/*.sh)
shopt -u nullglob
if [[ ${#hook_files[@]} -eq 0 ]]; then
  echo "verify-template: ERROR — no scripts/hooks/*.sh found in staged copy" >&2
  exit 1
fi
for hf in "${hook_files[@]}"; do
  bash -n "$hf" || {
    echo "verify-template: ERROR — $hf syntax error" >&2
    exit 1
  }
done

echo "verify-template: checking .claude/settings.json wires PreToolUse"
if [[ ! -f "$TMP/.claude/settings.json" ]] || ! grep -q "PreToolUse" "$TMP/.claude/settings.json"; then
  echo "verify-template: ERROR — .claude/settings.json missing or does not wire PreToolUse" >&2
  exit 1
fi

# --- leak guard (post-rename) ------------------------------------------------
# Re-affirm 0 residual placeholders + no secret patterns in the staged copy.
echo "verify-template: leak-guard (post-rename)"
( cd "$TMP" && bash scripts/leak-guard.sh --placeholders-only ) || {
  echo "verify-template: ERROR — leak-guard found residual placeholders" >&2
  exit 1
}

echo "verify-template: cargo generate-lockfile (fresh resolve)"
( cd "$TMP" && cargo generate-lockfile )

echo "verify-template: cargo build"
( cd "$TMP" && cargo build )

echo "verify-template: cargo clippy --all-targets -- -D warnings"
( cd "$TMP" && cargo clippy --all-targets -- -D warnings )

echo "verify-template: cargo nextest run --workspace"
( cd "$TMP" && cargo nextest run --workspace )

# --- throwaway Postgres: DDL assertion ----------------------------------------
# All queries are runtime — compile needs no DB — but the schema itself is
# asserted against a live server: the `<app>-api migrate` subcommand (go
# cmd/migrate parity) applies the embedded migrations; failing here means the
# DDL or the runner regress, not the compile.
echo "verify-template: starting throwaway Postgres (${PG_IMAGE})"
# -P publishes the image's exposed port (5432) to a random host port.
PG_CID="$(docker run -d --rm -P \
  -e POSTGRES_USER="$PGUSER" \
  -e POSTGRES_PASSWORD="$PGPASSWORD" \
  -e POSTGRES_DB="$PGDB" \
  "$PG_IMAGE")"

# Resolve the host port the container published.
PG_PORT="$(docker port "$PG_CID" 5432/tcp | awk -F: '{print $2}' | head -1)"
if [[ -z "$PG_PORT" ]]; then
  echo "verify-template: ERROR — could not resolve Postgres host port." >&2
  exit 1
fi

DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@127.0.0.1:${PG_PORT}/${PGDB}"
export DATABASE_URL
# The one-shot runner reads <APP>_DATABASE_URL first (post-rename spelling).
export "${EXAMPLE_APP^^}_DATABASE_URL=$DATABASE_URL"

echo "verify-template: waiting for Postgres on 127.0.0.1:${PG_PORT}"
ready=0
for _ in $(seq 1 30); do
  if docker exec "$PG_CID" pg_isready -U "$PGUSER" -d "$PGDB" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "$ready" -ne 1 ]]; then
  echo "verify-template: ERROR — Postgres did not become ready in time." >&2
  exit 1
fi

echo "verify-template: applying migrations (one-shot \`migrate\` subcommand)"
( cd "$TMP" && cargo run --quiet --bin "${EXAMPLE_APP}-api" -- migrate )

# OpenAPI generation: the `openapi` subcommand must emit a non-empty spec
# listing the health + auth + widget (incl. update) paths. Confirms the
# utoipa builder + every #[utoipa::path] annotation resolve post-rename.
echo "verify-template: openapi generation (cargo run --bin ${EXAMPLE_APP}-api -- openapi)"
( cd "$TMP" && cargo run --quiet --bin "${EXAMPLE_APP}-api" -- openapi > /tmp/verify-openapi.json )
if ! grep -q '/widgets' /tmp/verify-openapi.json || ! grep -q '/readyz' /tmp/verify-openapi.json; then
  echo "verify-template: ERROR — generated openapi spec missing required paths" >&2
  exit 1
fi
rm -f /tmp/verify-openapi.json

echo "verify-template: OK — all checks passed in the staged copy."