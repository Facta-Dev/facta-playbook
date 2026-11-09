#!/usr/bin/env bash
#
# clone-repos.sh — clone the FACTA working set by group.
#
# The inventory lives in install/repos.txt (one `group|org/repo|why` per line),
# which mirrors README §4. Shallow by default: the working set is consumed,
# not authored from here.
#
# Usage:
#   bash scripts/claude-kit/clone-repos.sh --group=core --dir=~/work
#        [--depth=1] [--dry-run]
#     --group=NAME  core | products | tools | clients | reference | all
#     --dir=DIR     destination root (default: $HOME/work)
#     --depth=N     clone depth (default 1; 0 = full history)
#                  both `--flag=value` and `--flag value` are accepted
#     --dry-run     print the plan and the exact commands, clone nothing
#
# Exit 0 = cloned or already present, 1 = fatal (bad group, missing inventory)
# or a clone failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

GROUP="core"
DEST="${HOME}/work"
DEPTH=1
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --group=*)  GROUP="${1#--group=}"; shift;;
    --group)    GROUP="${2:-}"; shift 2;;
    --dir=*)    DEST="${1#--dir=}"; shift;;
    --dir)      DEST="${2:-}"; shift 2;;
    --depth=*)  DEPTH="${1#--depth=}"; shift;;
    --depth)    DEPTH="${2:-}"; shift 2;;
    --dry-run)  DRY_RUN=1; shift;;
    -h|--help)  sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "clone-repos: unknown arg '$1'" >&2; exit 1;;
  esac
done

case "$DEPTH" in
  ''|*[!0-9]*) echo "clone-repos: ERROR — --depth must be a number" >&2; exit 1;;
esac

INVENTORY="$ROOT_DIR/install/repos.txt"
if [[ ! -f "$INVENTORY" ]]; then
  echo "clone-repos: ERROR — inventory not found: $INVENTORY" >&2
  exit 1
fi

say() { echo "clone-repos: $*"; }
warn() { echo "clone-repos: WARN — $*" >&2; }

GROUPS="core products tools clients reference"
if [[ "$GROUP" != "all" ]] && ! grep -qE "^$GROUP\|" "$INVENTORY"; then
  echo "clone-repos: ERROR — unknown group '$GROUP' (core|products|tools|clients|reference|all)" >&2
  exit 1
fi

CLONED=0
SKIPPED=0
FAILED=0

clone_one() {
  # clone_one <org/repo>
  local slug="$1" repo="${1##*/}"
  local dest="$DEST/$repo"
  if [[ -d "$dest/.git" ]]; then
    say "skip (exists) $dest"
    SKIPPED=$((SKIPPED + 1))
    return 0
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    if command -v gh >/dev/null 2>&1; then
      say "would clone gh repo clone $slug $dest -- --depth=$DEPTH"
    else
      say "would clone git clone --depth $DEPTH https://github.com/$slug.git $dest"
    fi
    return 0
  fi
  mkdir -p "$DEST"
  if command -v gh >/dev/null 2>&1; then
    gh repo clone "$slug" "$dest" -- --depth="$DEPTH" >/dev/null 2>&1 \
      || { warn "clone failed: $slug"; FAILED=$((FAILED + 1)); return 0; }
  else
    git clone --depth "$DEPTH" "https://github.com/$slug.git" "$dest" >/dev/null 2>&1 \
      || { warn "clone failed: $slug"; FAILED=$((FAILED + 1)); return 0; }
  fi
  say "cloned $slug -> $dest"
  CLONED=$((CLONED + 1))
}

say "group=$GROUP dest=$DEST depth=$DEPTH dry_run=$DRY_RUN"

while IFS='|' read -r grp slug _why; do
  [[ -z "${grp// }" || "$grp" == \#* ]] && continue
  if [[ "$GROUP" != "all" && "$grp" != "$GROUP" ]]; then
    continue
  fi
  slug="${slug#"${slug%%[![:space:]]*}"}"
  [[ -z "$slug" ]] && continue
  clone_one "$slug"
done < "$INVENTORY"

say "done — cloned=$CLONED skipped=$SKIPPED failed=$FAILED"
[[ "$FAILED" -gt 0 ]] && exit 1
exit 0
