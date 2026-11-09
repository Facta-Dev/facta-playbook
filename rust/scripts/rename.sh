#!/usr/bin/env bash
# Rename the template placeholders across the entire Rust workspace.
#
# Usage: bash scripts/rename.sh <new-app-slug> <new-org-or-scope>
#   <new-app-slug>      replaces <app>  (lowercase crate/package slug, e.g. mycompany)
#   <new-org-or-scope>  replaces <scope> (without leading @, e.g. acme) — the org
#                      in repo URLs (github.com/<scope>/<app>) and the domain-scope
#                      vocabulary used in code comments.
#
# Notes:
#   - <APP> (uppercase) is replaced in env-var prefixes / display names only
#     (e.g. <APP>_DATABASE_URL -> MYCOMPANY_DATABASE_URL).
#   - Substitution order is <scope> -> <APP> -> <app> so the lowercase <app> pass
#     never partially matches the <scope>/<APP> pass (COLLABORATION.md §3.1).
#   - Run this BEFORE the first `cargo generate-lockfile` so crate names resolve.
#   - TEXT ONLY: this script rewrites file contents (sed -i) and renames the
#     crates/<app>-* member directories so the rewritten workspace `members`
#     paths resolve. It does NOT generate or commit Cargo.lock — that is the
#     instantiator's next step. After renaming, run:
#       cargo generate-lockfile && cargo build   # generates Cargo.lock
#       git add Cargo.lock && git commit -m "chore: commit Cargo.lock post-rename"

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: bash scripts/rename.sh <new-app-slug> <new-org-or-scope>" >&2
  echo "  e.g. bash scripts/rename.sh mycompany acme" >&2
  exit 1
fi

NEW_APP="$1"
NEW_SCOPE="$2"

# Uppercase variant for env-var prefixes / display names only.
NEW_APP_UPPER="$(echo "$NEW_APP" | tr '[:lower:]' '[:upper:]')"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "rename: <scope> -> ${NEW_SCOPE}  <APP> -> ${NEW_APP_UPPER}  <app> -> ${NEW_APP}"

# Find all text files, excluding .git, target/ (build artifacts), lockfiles,
# the evals/fixtures/ frozen test data, and the pure-bash tooling scripts whose
# <app>/<APP>/<scope> literals are PATTERN TOKENS (grep patterns for the leak
# detector), not template placeholders to substitute. Fixtures are frozen
# contract inputs (valid and invalid .claude samples); the invalid leak fixture
# deliberately keeps literal <app>/<scope> tokens to exercise the leak
# detector, so it must NOT be rewritten by rename. evals/Cargo.toml +
# evals/src + evals/tests ARE rewritten (they carry the <app> crate name).
files=$(find . \
  -type f \
  -not -path './.git/*' \
  -not -path './target/*' \
  -not -path '*/target/*' \
  -not -name '*.lock' \
  -not -path './evals/fixtures/*' \
  -not -path './evals/src/*' \
  -not -path './evals/tests/*' \
  -not -path './scripts/lint-skills.sh' \
  -not -path './scripts/lint-commands.sh' \
  -not -path './scripts/leak-guard.sh' \
  -not -path './scripts/install-claude-runtime.sh' \
  -not -name '*.png' -not -name '*.jpg' -not -name '*.ico' \
  -not -name '*.woff' -not -name '*.woff2' -not -name '*.ttf' \
  | sort)

for f in $files; do
  if grep -qE '<app>|<APP>|<scope>' "$f" 2>/dev/null; then
    # Order matters: <scope> first, <APP> second, <app> last (§3.1).
    sed -i \
      -e "s/<scope>/${NEW_SCOPE}/g" \
      -e "s/<APP>/${NEW_APP_UPPER}/g" \
      -e "s/<app>/${NEW_APP}/g" \
      "$f"
    echo "  patched: $f"
  fi
done

# Rename the crates/<app>-* member directories so the rewritten workspace
# `members = ["crates/<app>-core", ...]` / `[workspace.dependencies]` paths
# resolve. The directories are literally named crates/<app>-core in the
# template (a placeholder is a valid Unix path component but an invalid crate
# name); cargo cannot find a member whose path was rewritten in Cargo.toml but
# whose directory was not renamed. This is a directory rename (mv), not
# lockfile generation — INV-T5 (text only, no lockfile commit) still holds.
shopt -s nullglob
for d in crates/'<app>'-*; do
  base="${d#crates/}"                  # <app>-core
  new_base="${base/<app>/${NEW_APP}}"   # mycompany-core
  if [[ -d "crates/$base" && "$base" != "$new_base" ]]; then
    mv "crates/$base" "crates/$new_base"
    echo "  renamed dir: crates/$base -> crates/$new_base"
  fi
done
shopt -u nullglob

echo "rename: done. Run 'cargo generate-lockfile && cargo build' next (generates Cargo.lock), then: git add Cargo.lock && git commit -m \"chore: commit Cargo.lock post-rename\""