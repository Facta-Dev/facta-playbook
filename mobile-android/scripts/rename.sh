#!/usr/bin/env bash
# rename.sh — substitute the <app>/<APP>/<scope> placeholders across the
# skeleton (text files and the package directories).
#
# Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>
#   <new-app-slug>  replaces <app>  (lowercase identifier, e.g. mycompany)
#   <new-org-scope> replaces <scope> (lowercase identifier, e.g. acme)
#
# Notes:
#   - <APP> (uppercase) lands in theme names and display labels only.
#   - Substitution order is <scope> -> <APP> -> <app>.
#   - scripts/*.sh are EXCLUDED: they are the template machinery and keep
#     their literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/* are EXCLUDED: frozen test data whose deliberate residual
#     tokens exercise the evals leak detector; rewriting them would defuse it.
#   - Binary files (the Gradle wrapper jar) and build output are never touched.
#   - The directory pass runs deepest-first so the package path
#     dev/<scope>/<app> becomes dev/acme/mycompany without breaking children.
#   - No lockfile exists: the template ships none, and Gradle resolves the
#     wrapper from gradle/wrapper/gradle-wrapper.properties.

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>" >&2
  echo "  e.g. bash scripts/rename.sh mycompany acme" >&2
  exit 1
fi

NEW_APP="$1"
NEW_SCOPE="$2"

# Both values become Java package segments (dev.<scope>.<app>), so only a
# lowercase identifier is buildable. Validate BEFORE the tree is touched: a
# hyphenated slug yields "Namespace 'dev.acme.my-app' is not a valid Java
# package name" and a slash aborts the substitution mid-tree, leaving a
# half-renamed copy behind.
if [[ ! "$NEW_APP" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "rename: invalid app slug '$NEW_APP' — expected a lowercase identifier (letter first, then letters, digits or underscore)." >&2
  exit 1
fi
if [[ ! "$NEW_SCOPE" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "rename: invalid org scope '$NEW_SCOPE' — expected a lowercase identifier (letter first, then letters, digits or underscore)." >&2
  exit 1
fi

NEW_APP_UPPER="$(printf '%s' "$NEW_APP" | tr '[:lower:]' '[:upper:]')"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "rename: <scope> -> ${NEW_SCOPE}  <APP> -> ${NEW_APP_UPPER}  <app> -> ${NEW_APP}"

cd "$ROOT_DIR"
rm -f local.properties
files=$(find . \
  -type f \
  -not -path './.git/*' \
  -not -path './.gradle/*' \
  -not -path '*/build/*' \
  -not -path './scripts/*' \
  -not -path './evals/fixtures/*' \
  -not -name '*.jar' -not -name '*.png' -not -name '*.jpg' -not -name '*.ico' \
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

# Directory pass: the package path carries the placeholders. Match the
# directory's own name only — matching the whole path would try to rename
# every descendant of dev/<scope>/<app> before its ancestors moved, and
# depth-first order renames the package components bottom-up anyway.
find . -depth -type d \
  -not -path './.git*' \
  -not -path './.gradle*' \
  -not -path '*/build*' \
  | while IFS= read -r dir; do
    base=$(basename "$dir")
    case "$base" in
      *'<scope>'*|*'<APP>'*|*'<app>'*)
        new_base=$(printf '%s' "$base" | sed \
          -e "s/<scope>/${NEW_SCOPE}/g" \
          -e "s/<APP>/${NEW_APP_UPPER}/g" \
          -e "s/<app>/${NEW_APP}/g")
        mv "$dir" "$(dirname "$dir")/$new_base"
        echo "  moved: $dir -> $(dirname "$dir")/$new_base"
        ;;
    esac
  done

echo "rename: done. Next: ./gradlew :app:assembleDebug && make ci"
