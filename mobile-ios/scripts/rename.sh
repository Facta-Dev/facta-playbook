#!/usr/bin/env bash
# rename.sh — substitute the <app>/<APP>/<scope> placeholders across the
# skeleton (text files and the app directory).
#
# Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>
#   <new-app-slug>  replaces <app>  (lowercase identifier, e.g. mycompany)
#   <new-org-scope> replaces <scope> (lowercase identifier, e.g. acme)
#
# Notes:
#   - <APP> (uppercase) lands in the Xcode target, the display name and the
#     generated .xcodeproj name.
#   - Substitution order is <scope> -> <APP> -> <app>.
#   - scripts/** are EXCLUDED: they are the template machinery and keep their
#     literal placeholder patterns so the gates keep working post-rename.
#   - evals/fixtures/** are EXCLUDED: frozen test data whose deliberate
#     residual tokens exercise the evals leak detector.
#   - Binary files and generated output (.xcodeproj, DerivedData, .build) are
#     never touched.
#   - File names carrying a placeholder (Sources/<APP>App.swift) are renamed
#     after the content pass.
#   - The directory pass runs deepest-first so Apps/<APP> becomes
#     Apps/Mycompany without breaking children.

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: bash scripts/rename.sh <new-app-slug> <new-org-scope>" >&2
  echo "  e.g. bash scripts/rename.sh mycompany acme" >&2
  exit 1
fi

NEW_APP="$1"
NEW_SCOPE="$2"

# Both values become Xcode target and bundle-id segments
# (com.<scope>.<app>): only a lowercase alphanumeric identifier is buildable.
# Validate BEFORE the tree is touched: a hyphenated slug yields an invalid
# bundle id and a slash aborts the substitution mid-tree, leaving a
# half-renamed copy behind.
if [[ ! "$NEW_APP" =~ ^[a-z][a-z0-9]*$ ]]; then
  echo "rename: invalid app slug '$NEW_APP' — expected a lowercase identifier (letter first, then letters or digits)." >&2
  exit 1
fi
if [[ ! "$NEW_SCOPE" =~ ^[a-z][a-z0-9]*$ ]]; then
  echo "rename: invalid org scope '$NEW_SCOPE' — expected a lowercase identifier (letter first, then letters or digits)." >&2
  exit 1
fi

NEW_APP_UPPER="$(printf '%s' "$NEW_APP" | tr '[:lower:]' '[:upper:]')"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "rename: <scope> -> ${NEW_SCOPE}  <APP> -> ${NEW_APP_UPPER}  <app> -> ${NEW_APP}"

cd "$ROOT_DIR"

files=$(find . \
  -type f \
  -not -path './.git/*' \
  -not -path './.build/*' \
  -not -path './DerivedData/*' \
  -not -path '*.xcodeproj/*' \
  -not -path './scripts/*' \
  -not -path './evals/fixtures/*' \
  -not -name '*.png' -not -name '*.jpg' -not -name '*.ico' \
  -not -name '*.woff' -not -name '*.woff2' -not -name '*.ttf' \
  | sort)

while IFS= read -r f; do
  if grep -qE '<app>|<APP>|<scope>' "$f" 2>/dev/null; then
    sed -i \
      -e "s/<scope>/${NEW_SCOPE}/g" \
      -e "s/<APP>/${NEW_APP_UPPER}/g" \
      -e "s/<app>/${NEW_APP}/g" \
      "$f"
    echo "  patched: $f"
  fi
done <<< "$files"

# File pass: the entry point ships as <APP>App.swift. The content pass above
# substituted its body; the basename has to follow, or a placeholder survives
# in the tree and XcodeGen writes an angle-bracketed file reference.
find . -depth -type f \
  -not -path './.git/*' \
  -not -path './.build/*' \
  -not -path './DerivedData/*' \
  -not -path '*.xcodeproj/*' \
  -not -path './scripts/*' \
  -not -path './evals/fixtures/*' \
  | while IFS= read -r file; do
    base=$(basename "$file")
    case "$base" in
      *'<scope>'*|*'<APP>'*|*'<app>'*)
        new_base=$(printf '%s' "$base" | sed \
          -e "s/<scope>/${NEW_SCOPE}/g" \
          -e "s/<APP>/${NEW_APP_UPPER}/g" \
          -e "s/<app>/${NEW_APP}/g")
        mv "$file" "$(dirname "$file")/$new_base"
        echo "  renamed: $file -> $(dirname "$file")/$new_base"
        ;;
    esac
  done

# Directory pass: Apps/<APP> carries the placeholder in its own name. Match
# the directory's own name only — depth-first order renames children before
# their parents move.
find . -depth -type d \
  -not -path './.git*' \
  -not -path './.build*' \
  -not -path './DerivedData*' \
  -not -path '*.xcodeproj*' \
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

echo "rename: done. Next: make ci (and xcodegen generate on macOS)"
