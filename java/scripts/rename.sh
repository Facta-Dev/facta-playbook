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
#   - Substitution order is <scope> -> <APP> -> <app>, without partial-token
#     overlap; the Java package stays `com.example.app` (rename it by hand).
#   - Run before the first `mvn verify`.
#   - scripts/*.sh are EXCLUDED: they are the template machinery and keep
#     their literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/* are EXCLUDED: frozen test data whose deliberate residual
#     <app> tokens exercise the evals leak detector (contracts_test.py asserts
#     the validator still rejects them); rewriting them would defuse the test.
#   - pom.xml is EXCLUDED: its Maven <scope> element is not the placeholder.
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
  -not -path './pom.xml' \
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

echo "rename: done. Next: mvn verify (then commit pom.xml changes)."