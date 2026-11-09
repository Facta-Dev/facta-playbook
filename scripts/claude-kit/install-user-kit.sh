#!/usr/bin/env bash
#
# install-user-kit.sh — install the canonical user-scope Claude Code kit
# (kit/) into a machine's ~/.claude. PURE BASH: no jq, no node. python3 is
# used only for the settings.json deep merge.
#
# Idempotent and non-destructive: a file that already exists at the
# destination is skipped with a recorded warning unless --force is given.
# Anything overwritten is backed up to <dest>/backups/install-<UTC stamp>/.
#
# Usage:
#   bash scripts/claude-kit/install-user-kit.sh [--dry-run] [--force]
#        [--no-settings] [--dest=DIR]
#     --dry-run      print the actions, change nothing
#     --force        overwrite existing destination files (backed up)
#     --no-settings  do not merge kit/settings.fragment.json
#     --dest=DIR     destination root (default: $HOME/.claude)
#
# Exit 0 = installed (or no-op), exit 1 = fatal (missing kit/, bad JSON).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT_DIR="$(cd "$SCRIPT_DIR/../../kit" && pwd)"

DEST="${HOME}/.claude"
DRY_RUN=0
FORCE=0
MERGE_SETTINGS=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)     DRY_RUN=1; shift;;
    --force)       FORCE=1; shift;;
    --no-settings) MERGE_SETTINGS=0; shift;;
    --dest=*)      DEST="${1#--dest=}"; shift;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0;;
    *) echo "install-user-kit: unknown arg '$1'" >&2; exit 1;;
  esac
done

if [[ ! -d "$KIT_DIR" ]]; then
  echo "install-user-kit: ERROR — kit directory $KIT_DIR not found" >&2
  exit 1
fi

STAMP="$(date -u +%Y%m%d-%H%M%SZ)"
BACKUP_DIR="$DEST/backups/install-$STAMP"

COPIED=0
SKIPPED=0
OVERWRITTEN=0
WARNINGS=0

say() { echo "install-user-kit: $*"; }
warn() { echo "install-user-kit: WARN — $*" >&2; WARNINGS=$((WARNINGS + 1)); }

backup_file() {
  # backup_file <abs source path> <path relative to dest>
  local src="$1" rel="$2"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "backup $src -> $BACKUP_DIR/$rel"
    return 0
  fi
  mkdir -p "$BACKUP_DIR/$(dirname "$rel")"
  cp -p "$src" "$BACKUP_DIR/$rel"
  say "backup $src -> $BACKUP_DIR/$rel"
}

install_file() {
  # install_file <src> <relative path under dest>
  local src="$1" rel="$2" dst="$DEST/$2"
  if [[ ! -f "$src" ]]; then
    warn "missing source $src"
    return 0
  fi
  if [[ -e "$dst" ]]; then
    if [[ "$FORCE" -eq 1 ]]; then
      backup_file "$dst" "$rel"
      if [[ "$DRY_RUN" -eq 1 ]]; then
        say "overwrite $dst"
      else
        mkdir -p "$(dirname "$dst")"
        cp -p "$src" "$dst"
        say "overwrite $dst"
      fi
      OVERWRITTEN=$((OVERWRITTEN + 1))
    else
      say "skip (exists) $dst"
      warn "skipped $rel — pass --force to overwrite"
      SKIPPED=$((SKIPPED + 1))
    fi
    return 0
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "install $dst"
  else
    mkdir -p "$(dirname "$dst")"
    cp -p "$src" "$dst"
    say "install $dst"
  fi
  COPIED=$((COPIED + 1))
}

# --- copy the kit files ------------------------------------------------------
install_file "$KIT_DIR/CLAUDE.md" "CLAUDE.md"

for f in "$KIT_DIR"/agents/*.md; do
  [[ -e "$f" ]] || continue
  install_file "$f" "agents/$(basename "$f")"
done

for d in "$KIT_DIR"/skills/*/; do
  [[ -d "$d" ]] || continue
  name="$(basename "$d")"
  install_file "$d/SKILL.md" "skills/$name/SKILL.md"
done

for f in "$KIT_DIR"/templates/sdd/*.md; do
  [[ -e "$f" ]] || continue
  install_file "$f" "templates/sdd/$(basename "$f")"
done

# --- merge settings.fragment.json into settings.json --------------------------
if [[ "$MERGE_SETTINGS" -eq 1 ]]; then
  FRAGMENT="$KIT_DIR/settings.fragment.json"
  if [[ ! -f "$FRAGMENT" ]]; then
    echo "install-user-kit: ERROR — $FRAGMENT not found" >&2
    exit 1
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "install-user-kit: ERROR — python3 is required for the settings merge" >&2
    exit 1
  fi
  TARGET="$DEST/settings.json"
  if [[ -f "$TARGET" ]]; then
    backup_file "$TARGET" "settings.json"
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "merge $FRAGMENT -> $TARGET"
  else
    mkdir -p "$DEST"
    python3 - "$FRAGMENT" "$TARGET" <<'PY'
import json, sys

fragment_path, target_path = sys.argv[1], sys.argv[2]

try:
    with open(fragment_path) as fh:
        fragment = json.load(fh)
except (OSError, ValueError) as exc:
    sys.exit("install-user-kit: ERROR — invalid fragment JSON: %s" % exc)

try:
    with open(target_path) as fh:
        target = json.load(fh)
except FileNotFoundError:
    target = {}
except (OSError, ValueError) as exc:
    sys.exit("install-user-kit: ERROR — invalid settings.json: %s" % exc)

if not isinstance(target, dict):
    sys.exit("install-user-kit: ERROR — settings.json is not a JSON object")


def merge(base, frag, key=None):
    """Deep merge: existing user values win; deny lists are unioned."""
    for k, v in frag.items():
        if k not in base:
            base[k] = v
        elif isinstance(base[k], dict) and isinstance(v, dict):
            merge(base[k], v, k)
        elif k == "deny" and isinstance(base[k], list) and isinstance(v, list):
            for rule in v:
                if rule not in base[k]:
                    base[k].append(rule)


merge(target, fragment)

with open(target_path, "w") as fh:
    json.dump(target, fh, indent=2, sort_keys=False)
    fh.write("\n")
PY
    say "merge $FRAGMENT -> $TARGET"
  fi
else
  say "skip settings merge (--no-settings)"
fi

# --- summary -----------------------------------------------------------------
say "done — installed=$COPIED overwritten=$OVERWRITTEN skipped=$SKIPPED warnings=$WARNINGS"
exit 0
