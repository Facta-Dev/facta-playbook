#!/usr/bin/env bash
#
# clone-repos.sh — clone the FACTA working set listed in install/repos.txt.
#
# The inventory is the single source of truth (derived from README.md §4):
# one `group|org/repo|why` per line; '#' comments and blank lines are
# ignored. Clones are shallow and single-branch: the working set is
# consumed from here, not authored in it.
#
# Usage:
#   bash scripts/claude-kit/clone-repos.sh [--group=selected|core|products|tools|clients|reference|all]
#        [--dir=DIR] [--depth=N] [--dry-run]
#     --group=NAME  group to clone (default: selected -- the install set)
#     --dir=DIR     destination root (default: $HOME/work)
#     --depth=N     clone depth (default: 1)
#     --dry-run     print the exact commands, clone nothing
#
# Both `--flag=value` and `--flag value` are accepted.
#
# Exit 0 = cloned or already present, 1 = bad usage or a clone failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
INVENTORY="$ROOT_DIR/install/repos.txt"

GROUP="selected"
DEST="${HOME}/work"
DEPTH=1
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --group=*) GROUP="${1#--group=}"; shift;;
    --group)   GROUP="${2:-}"; shift 2;;
    --dir=*)   DEST="${1#--dir=}"; shift;;
    --dir)     DEST="${2:-}"; shift 2;;
    --depth=*) DEPTH="${1#--depth=}"; shift;;
    --depth)   DEPTH="${2:-}"; shift 2;;
    --dry-run) DRY_RUN=1; shift;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "clone-repos: unknown arg '$1'" >&2; exit 1;;
  esac
done

case "$DEPTH" in
  ''|*[!0-9]*) echo "clone-repos: ERROR — --depth must be a number" >&2; exit 1;;
esac

if [[ ! -f "$INVENTORY" ]]; then
  echo "clone-repos: ERROR — inventory not found: $INVENTORY" >&2
  exit 1
fi

# The valid groups come from the inventory, not from a hardcoded list, so a
# group added to install/repos.txt is clonable without editing this script.
# (Do not name this variable GROUPS: bash owns that one as the user's group ids.)
KNOWN_GROUPS="$(awk -F'|' '
  /^[[:space:]]*#/ || NF < 2 { next }
  { gsub(/^[[:space:]]+|[[:space:]]+$/, "", $1); if ($1 != "") print $1 }
' "$INVENTORY" | sort -u | tr '\n' ' ')"
if [[ " $KNOWN_GROUPS " != *" $GROUP "* && "$GROUP" != "all" ]]; then
  echo "clone-repos: ERROR — unknown group '$GROUP' ($KNOWN_GROUPS|all)" >&2
  exit 1
fi

say()  { echo "clone-repos: $*"; }
warn() { echo "clone-repos: WARN — $*" >&2; }

CLONED=0
SKIPPED=0
FAILED=0

clone_one() {
  # clone_one <group> <org/repo> <why>
  local group="$1" slug="$2" why="$3"
  local repo="${slug##*/}"
  local dest="$DEST/$repo"
  local cmd=(git clone)
  [[ "$DEPTH" -gt 0 ]] && cmd+=("--depth=$DEPTH")
  cmd+=(--single-branch --filter=blob:none "https://github.com/$slug.git" "$dest")

  if [[ -d "$dest" ]]; then
    SKIPPED=$((SKIPPED + 1))
    [[ "$DRY_RUN" -eq 1 ]] && return 0
    say "skip  $slug -> $dest (already present)"
    return 0
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "${cmd[*]}"
    return 0
  fi

  mkdir -p "$DEST"
  if "${cmd[@]}" >/dev/null 2>&1; then
    say "clone $slug -> $dest  ($why)"
    CLONED=$((CLONED + 1))
  else
    warn "clone failed: $slug"
    FAILED=$((FAILED + 1))
  fi
}

say "group=$GROUP dest=$DEST depth=$DEPTH dry_run=$DRY_RUN"

SEEN=" "
while IFS='|' read -r group slug why; do
  group="${group#"${group%%[![:space:]]*}"}"
  slug="${slug#"${slug%%[![:space:]]*}"}"
  why="${why#"${why%%[![:space:]]*}"}"
  [[ -z "$group" || "$group" == \#* ]] && continue
  [[ "$GROUP" != "all" && "$group" != "$GROUP" ]] && continue
  [[ -z "$slug" ]] && continue
  # A repo can belong to several groups (selected duplicates the reference
  # entries); clone it once per run.
  [[ "$SEEN" == *" $slug "* ]] && continue
  SEEN+="$slug "
  clone_one "$group" "$slug" "$why"
done < "$INVENTORY"

say "done — cloned=$CLONED skipped=$SKIPPED failed=$FAILED"
[[ "$FAILED" -gt 0 ]] && exit 1
exit 0
