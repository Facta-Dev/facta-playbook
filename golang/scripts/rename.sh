#!/usr/bin/env bash
# rename.sh — substitute the <app>/<APP>/<scope> placeholders across the
# skeleton (TEXT ONLY, INV-T5: never generates or commits lockfiles).
#
# Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>
#   <new-app-slug>  replaces <app>  (lowercase slug, e.g. mycompany)
#   <new-org-scope> replaces <scope> (org/repo scope, e.g. acme)
#
# Notes:
#   - <APP> (uppercase) lands in env-var prefixes and display names only.
#   - Substitution order is <scope> -> <APP> -> <app> (COLLABORATION.md §3.1):
#     the module path "github.com/<scope>/<app>" becomes
#     "github.com/acme/mycompany" in one pass, without partial-token overlap.
#   - Run BEFORE `go mod tidy` (the skeleton ships no go.sum — INV-T3).
#   - scripts/*.sh are EXCLUDED: they are the template machinery and keep
#     their literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/* are EXCLUDED: frozen test data whose deliberate residual
#     <app> tokens exercise the evals leak detector (contracts_test.go asserts
#     the validator still rejects them); rewriting them would defuse the test.
#   - evals/*.go + evals/run-live.sh carry no placeholders; evals/go.mod IS
#     rewritten (its module path is github.com/<scope>/<app>/evals).
#   - Text rewrites only; no directory renames (slices use clean names).

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>" >&2
  echo "  e.g. bash scripts/rename.sh mycompany acme" >&2
  exit 1
fi

NEW_APP="$1"
NEW_SCOPE="$2"
NEW_APP_UPPER="$(echo "$NEW_APP" | tr '[:lower:]' '[:upper:]')"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "rename: <scope> -> ${NEW_SCOPE}  <APP> -> ${NEW_APP_UPPER}  <app> -> ${NEW_APP}"

cd "$ROOT_DIR"
files=$(find . \
  -type f \
  -not -path './.git/*' \
  -not -path '*/node_modules/*' \
  -not -path './scripts/*' \
  -not -path './evals/fixtures/*' \
  -not -name '*.go.sum' \
  -not -name '*.png' -not -name '*.jpg' -not -name '*.ico' \
  -not -name '*.woff' -not -name '*.woff2' -not -name '*.ttf' \
  | sort)

for f in $files; do
  if grep -qE '<app>|<APP>|<scope>' "$f" 2>/dev/null; then
    sed -i \
      -e "s/<scope>/${NEW_SCOPE}/g" \
      -e "s/<APP>/${NEW_APP_UPPER}/g" \
      -e "s/<app>/${NEW_APP}/g" \
      "$f"
    echo "  patched: $f"
  fi
done

echo "rename: done. Next: go mod tidy && make ci (then commit go.mod + go.sum)."