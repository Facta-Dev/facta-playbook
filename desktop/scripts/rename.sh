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
#   - Substitution order is <scope> -> <APP> -> <app>: CMake project names,
#     binary names and display strings resolve in one pass, without
#     partial-token overlap.
#   - Run BEFORE cmake configure (the skeleton ships no build tree).
#   - scripts/*.sh are EXCLUDED: they are the template machinery and keep
#     their literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/* are EXCLUDED: frozen test data whose deliberate residual
#     <app> tokens exercise the evals leak detector (evals/contracts_test.cpp
#     asserts the validator still rejects them); rewriting them would defuse the test.
#   - evals/contracts_test.cpp + evals/run-live.sh carry no placeholders;
#     evals/CMakeLists.txt IS rewritten (its project name is <app>-evals — the
#     rename-state marker read by the post-rename sweep in contracts_test.cpp).
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
  -not -path './build/*' \
  -not -path './build-*/*' \
  -not -path './scripts/*' \
  -not -path './evals/fixtures/*' \
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

echo "rename: done. Next: make ci (cmake configure, build, ctest, evals)"
echo "rename: pnpm format is required once — substituted token lengths re-align markdown tables."