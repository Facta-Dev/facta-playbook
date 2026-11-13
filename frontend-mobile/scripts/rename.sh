#!/usr/bin/env bash
# rename.sh — substitute the <app>/<APP>/<scope> placeholders across the
# mobile skeleton (TEXT ONLY, INV-T5: never generates or commits lockfiles).
#
# Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>
#   <new-app-slug>  replaces <app>  (lowercase identifier, e.g. mycompany)
#   <new-org-scope> replaces <scope> (lowercase identifier, e.g. acme)
#
# Notes:
#   - <APP> (uppercase) lands in the display name and in env-var prefixes only.
#   - Substitution order is <scope> -> <APP> -> <app>: the package name
#     "@<scope>/<app>" becomes "@acme/mycompany" in one pass, without
#     partial-token overlap.
#   - Run BEFORE pnpm install (the skeleton ships no lockfile — INV-T3); the
#     script removes a pnpm-lock.yaml if one is lying around.
#   - scripts/*.sh are EXCLUDED: they are the template machinery and keep
#     their literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/* are EXCLUDED: frozen test data whose deliberate residual
#     <app> tokens exercise the evals leak detector (evals/contracts.test.mjs
#     asserts the validator still rejects them); rewriting them would defuse the test.
#   - evals/contracts.test.mjs + evals/run-live.sh carry no placeholders;
#     evals/package.json IS rewritten (its name is @<scope>/<app>-evals — the
#     rename-state marker read by the post-rename sweep in contracts.test.mjs).
#   - Text rewrites only; no directory renames (slices use clean names).

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>" >&2
  echo "  e.g. bash scripts/rename.sh mycompany acme" >&2
  exit 1
fi

NEW_APP="$1"
NEW_SCOPE="$2"

# Both values become identifiers that ship as-is (the npm package
# "@<scope>/<app>", the Android applicationId and the iOS bundle identifier
# "com.<scope>.<app>"), so only a lowercase identifier is buildable. Validate
# BEFORE the tree is touched: a hyphenated slug yields an invalid package name
# and a slash aborts the substitution mid-tree, leaving a half-renamed copy.
if [[ ! "$NEW_APP" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "rename: invalid app slug '$NEW_APP' — expected a lowercase identifier (letter first, then letters, digits or underscore)." >&2
  exit 1
fi
if [[ ! "$NEW_SCOPE" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "rename: invalid org scope '$NEW_SCOPE' — expected a lowercase identifier (letter first, then letters, digits or underscore)." >&2
  exit 1
fi

NEW_APP_UPPER="$(echo "$NEW_APP" | tr '[:lower:]' '[:upper:]')"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "rename: <scope> -> ${NEW_SCOPE}  <APP> -> ${NEW_APP_UPPER}  <app> -> ${NEW_APP}"

cd "$ROOT_DIR"
rm -f pnpm-lock.yaml
files=$(find . \
  -type f \
  -not -path './.git/*' \
  -not -path '*/node_modules/*' \
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

echo "rename: done. Next: pnpm install && pnpm format && make ci"
echo "rename: pnpm format is required once — substituted token lengths re-align markdown tables."
