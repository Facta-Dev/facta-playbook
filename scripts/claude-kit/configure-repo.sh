#!/usr/bin/env bash
#
# configure-repo.sh — wire one repository to the playbook (idempotent).
#
# It never deletes and never overwrites a user-owned file: every write is
# additive and marker-guarded, and the only file it rewrites (settings.json)
# is backed up first. --check verifies a repo without writing; --dry-run
# prints the diff it would apply.
#
# Usage:
#   bash scripts/claude-kit/configure-repo.sh <repo-path> [--kit=golang|nodejs|python|rust]
#        [--check] [--dry-run]
#     <repo-path   repository root (must be a git work tree)
#     --kit=NAME   also install that language kit (golang|nodejs|python|rust)
#     --check      verify the repo, write nothing; exit 1 on drift
#     --dry-run    print the diff it would apply, write nothing
#
# Both `--flag=value` and `--flag value` are accepted.
#
# Exit 0 = configured (or already configured), 1 = drift, bad usage or a
# failed write.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
ROUTING_DOC="$ROOT_DIR/CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md"

REPO=""
KIT=""
CHECK=0
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --kit=*)   KIT="${1#--kit=}"; shift;;
    --kit)     KIT="${2:-}"; shift 2;;
    --check)   CHECK=1; shift;;
    --dry-run) DRY_RUN=1; shift;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    -*)        echo "configure-repo: unknown arg '$1'" >&2; exit 1;;
    *)         REPO="$1"; shift;;
  esac
done

say() { echo "configure-repo: $*"; }
die() { echo "configure-repo: ERROR — $*" >&2; exit 1; }

[[ -n "$REPO" ]] || die "repo path required"
[[ -d "$REPO" ]] || die "no such directory: $REPO"
if [[ -n "$KIT" && ! -d "$ROOT_DIR/$KIT" ]]; then
  die "unknown kit '$KIT' (golang|nodejs|python|rust)"
fi

# The repo root is the git work tree root, not the path as typed.
REPO_ROOT="$(git -C "$REPO" rev-parse --show-toplevel 2>/dev/null)" \
  || die "not a git repository: $REPO"
REPO="$REPO_ROOT"

MARKER='<!-- model-routing:ollama -->'
CLAUDE_MD="$REPO/CLAUDE.md"
DOTCLAUDE="$REPO/.claude"
SETTINGS="$DOTCLAUDE/settings.json"
GITIGNORE="$REPO/.gitignore"
BACKUP_DIR="$DOTCLAUDE/backups/configure-$(date -u +%Y-%m-%d)"

# The routing block, verbatim from the runbook (section 6). Extracted at run
# time so the two files can never drift apart.
[[ -f "$ROUTING_DOC" ]] || die "routing runbook not found: $ROUTING_DOC"
ROUTING_BLOCK="$(awk '/^<!-- model-routing:ollama -->$/{f=1;print;next} f&&/^```$/{exit} f{print}' "$ROUTING_DOC")"
[[ -n "$ROUTING_BLOCK" ]] || die "no model-routing block in $ROUTING_DOC"

# Project-scope deny list. User-scope rules anchor // or ~; a project
# settings.json resolves them from the repo root (PLAYBOOK_CLAUDE_CODE A.6).
DENY_RULES=(
  'Read(./.env)'
  'Read(./.env.*)'
  'Read(./secrets/**)'
  'Read(**/*.pem)'
  'Read(**/*.key)'
  'Bash(git push --force*)'
  'Bash(git reset --hard*)'
)

GITIGNORE_ENTRIES=(
  'CLAUDE.local.md'
  '.claude/settings.local.json'
  '.claude/install-state.json'
  '.claude/backups/'
  '.claude/logs/'
)

DOTCLAUDE_README="$(cat <<'EOF'
# .claude — this repo's Claude Code kit

The playbooks live in the playbook repo, not here. Read them there; do not
restate or import them into this file.

| Playbook | Governs |
| --- | --- |
| [PLAYBOOK_CLAUDE_CODE.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_CLAUDE_CODE.md) | the kit this repo must have |
| [PLAYBOOK_TESTS.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_TESTS.md) | testing contract and gates |
| [PLAYBOOK_SECURITY.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_SECURITY.md) | security contract, per-language profiles |

Runbook: [CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md)
— model tiers, the routing block, `~/.claude/settings.json`.

Repo-local kit files live next to this file: `agents/`, `skills/`, `commands/`,
`rules/`, `settings.json`. Verify the wiring with:

```bash
bash ~/ai-coding-playbook/scripts/claude-kit/configure-repo.sh <repo> --check
```
EOF
)"

TOUCHED=()
SKIPPED=()
DRIFT=()

note_touched() { TOUCHED+=("$1"); }
note_skipped() { SKIPPED+=("$1"); }
note_drift()   { DRIFT+=("$1"); }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- plan --------------------------------------------------------------------
# Every planned write lands in $WORK first. The plan is identical in apply,
# --dry-run and --check mode; only the last step differs.

# 1. the language kit installer (run before the merge: the kit installer skips
#    destinations that already exist, so a settings.json written earlier would
#    keep the kit's hook settings from landing).
KIT_INSTALLER=""
if [[ -n "$KIT" ]]; then
  if [[ -f "$ROOT_DIR/$KIT/scripts/install-claude-runtime.sh" ]]; then
    KIT_INSTALLER="$ROOT_DIR/$KIT/scripts/install-claude-runtime.sh"
  elif [[ -f "$REPO/scripts/install-claude-runtime.sh" ]]; then
    KIT_INSTALLER="$REPO/scripts/install-claude-runtime.sh"
  else
    die "kit script not found: $ROOT_DIR/$KIT/scripts/install-claude-runtime.sh"
  fi
fi

# 2. CLAUDE.md: the routing block, only when the marker is absent.
PLAN_CLAUDE_MD=""
if [[ -f "$CLAUDE_MD" ]] && grep -qF "$MARKER" "$CLAUDE_MD"; then
  PLAN_CLAUDE_MD="skip"
else
  if [[ -f "$CLAUDE_MD" ]]; then
    { cat "$CLAUDE_MD"; printf '\n%s\n' "$ROUTING_BLOCK"; } > "$WORK/claude.md"
  else
    { printf '# %s — agent instructions\n\n' "$(basename "$REPO")"
      printf '%s\n' "$ROUTING_BLOCK"; } > "$WORK/claude.md"
  fi
  PLAN_CLAUDE_MD="$WORK/claude.md"
fi

# 3. .claude/README.md: only when absent.
PLAN_DOTREADME=""
if [[ -f "$DOTCLAUDE/README.md" ]]; then
  PLAN_DOTREADME="skip"
else
  printf '%s\n' "$DOTCLAUDE_README" > "$WORK/dotclaude-readme.md"
  PLAN_DOTREADME="$WORK/dotclaude-readme.md"
fi

# 4. permissions.deny merged into .claude/settings.json. Deny only: allow,
#    env, model and hooks keys are left untouched.
#    $1 = file to merge into (defaults to the repo's settings.json).
merge_deny() {
  python3 - "${1:-$SETTINGS}" "$WORK/settings.json" "${DENY_RULES[@]}" <<'PY'
import json, sys
from collections import OrderedDict

target, planned = sys.argv[1], sys.argv[2]
rules = sys.argv[3:]

try:
    with open(target) as fh:
        settings = json.load(fh, object_pairs_hook=OrderedDict)
except FileNotFoundError:
    settings = OrderedDict()
except (OSError, ValueError) as exc:
    sys.exit("configure-repo: ERROR — invalid settings.json: %s" % exc)

if not isinstance(settings, dict):
    sys.exit("configure-repo: ERROR — settings.json is not a JSON object")

perms = settings.get("permissions")
if not isinstance(perms, dict):
    perms = OrderedDict()
    settings["permissions"] = perms
deny = perms.get("deny")
if not isinstance(deny, list):
    deny = []
    perms["deny"] = deny

changed = False
for rule in rules:
    if rule not in deny:
        deny.append(rule)
        changed = True

with open(planned, "w") as fh:
    json.dump(settings, fh, indent=2)
    fh.write("\n")

print("changed" if changed else "unchanged")
PY
}

# The kit installer writes .claude/settings.json (hooks) and skips existing
# destinations, so in apply mode the merge must be planned AFTER the kit ran.
# In --check/--dry-run the kit does not run, so the merge is planned against
# the file the kit would write (its own settings.json) when the repo has none.
KIT_SETTINGS=""
[[ -n "$KIT_INSTALLER" ]] && KIT_SETTINGS="$(cd "$(dirname "$KIT_INSTALLER")/.." && pwd)/.claude/settings.json"

plan_settings() {
  local source="$SETTINGS"
  if [[ ! -f "$source" && -n "$KIT_SETTINGS" && -f "$KIT_SETTINGS" ]]; then
    source="$KIT_SETTINGS"
  fi
  MERGE_STATUS="$(merge_deny "$source")" || die "settings.json is not valid JSON"
}

# 5. .gitignore entries, one per line, each only when missing.
{
  if [[ -f "$GITIGNORE" && -s "$GITIGNORE" ]]; then
    cat "$GITIGNORE"
    # A file without a trailing newline would glue the first new entry onto
    # the last existing line; add the separator only when it is missing.
    if [[ "$(tail -c 1 "$GITIGNORE" | wc -l)" -eq 0 ]]; then
      printf '\n'
    fi
  fi
  for entry in "${GITIGNORE_ENTRIES[@]}"; do
    if [[ -f "$GITIGNORE" ]]; then
      grep -qxF "$entry" "$GITIGNORE" || printf '%s\n' "$entry"
    else
      printf '%s\n' "$entry"
    fi
  done
} > "$WORK/gitignore"

# --- diffs -------------------------------------------------------------------
print_diff() {
  # print_diff <label> <current-file> <planned-file>
  local label="$1" cur="$2" new="$3"
  if [[ -f "$cur" ]]; then
    diff -u --label "$label" --label "$label" "$cur" "$new" || true
  else
    diff -u --label /dev/null --label "$label" /dev/null "$new" || true
  fi
}

# --- apply -------------------------------------------------------------------
# The kit runs first: it writes .claude/settings.json (hooks) and skips
# existing destinations, so the deny merge must see the kit's file.
if [[ "$CHECK" -eq 0 && "$DRY_RUN" -eq 0 && -n "$KIT_INSTALLER" ]]; then
  # ponytail: the kit installer resolves "claude-project" from the manifest
  # with a broken awk lookup and falls back to .claude-project/, so a duplicate
  # tree is written next to .claude/. We drop only that duplicate, and only
  # when this run created it. Upgrade path: fix runtime_dir() in
  # <kit>/scripts/install-claude-runtime.sh and delete this cleanup.
  had_project_dir=0
  [[ -d "$REPO/.claude-project" ]] && had_project_dir=1
  bash "$KIT_INSTALLER" --targets=claude,claude-project --dest="$REPO" >/dev/null
  if [[ "$had_project_dir" -eq 0 && -d "$REPO/.claude-project" ]]; then
    rm -rf "$REPO/.claude-project"
    say "dropped the duplicate .claude-project/ (kit installer manifest bug)"
    # Keep provenance honest: drop the state entries that pointed at the
    # directory we just removed.
    if [[ -f "$REPO/.claude/install-state.json" ]] && command -v python3 >/dev/null 2>&1; then
      python3 - "$REPO/.claude/install-state.json" <<'PY'
import json, sys
path = sys.argv[1]
try:
    with open(path) as fh:
        state = json.load(fh)
    ops = state.get("operations", [])
    kept = [op for op in ops if ".claude-project/" not in str(op.get("destination", ""))]
    if len(kept) != len(ops):
        state["operations"] = kept
        with open(path, "w") as fh:
            json.dump(state, fh, indent=2)
            fh.write("\n")
except (OSError, ValueError):
    pass
PY
    fi
  fi
  note_touched "$KIT kit ($KIT_INSTALLER)"
fi

plan_settings

# --- --check: verify, write nothing -----------------------------------------
if [[ "$CHECK" -eq 1 ]]; then
  if [[ "$PLAN_CLAUDE_MD" == "skip" ]]; then
    say "ok   routing block in CLAUDE.md"
  else
    note_drift "routing block missing from CLAUDE.md"
  fi
  if [[ "$PLAN_DOTREADME" == "skip" ]]; then
    say "ok   .claude/README.md"
  else
    note_drift ".claude/README.md missing"
  fi
  if [[ "$MERGE_STATUS" == "unchanged" && -f "$SETTINGS" ]]; then
    say "ok   permissions.deny in .claude/settings.json"
  else
    note_drift "permissions.deny incomplete in .claude/settings.json"
  fi
  if [[ -f "$GITIGNORE" ]] && diff -q "$GITIGNORE" "$WORK/gitignore" >/dev/null; then
    say "ok   .gitignore entries"
  else
    note_drift ".gitignore entries missing"
  fi
  if [[ -n "$KIT" && ! -d "$DOTCLAUDE/agents" ]]; then
    note_drift "$KIT kit not installed (.claude/agents missing)"
  fi
  if [[ ${#DRIFT[@]} -eq 0 ]]; then
    say "done — no drift"
    exit 0
  fi
  for d in "${DRIFT[@]}"; do say "DRIFT $d"; done
  say "done — drift=${#DRIFT[@]}"
  exit 1
fi

# --- --dry-run: print the diff, write nothing -------------------------------
if [[ "$DRY_RUN" -eq 1 ]]; then
  [[ -n "$KIT_INSTALLER" ]] && say "would run bash $KIT_INSTALLER --targets=claude,claude-project --dest=$REPO"
  [[ "$PLAN_CLAUDE_MD" != "skip" ]] && print_diff "CLAUDE.md" "$CLAUDE_MD" "$PLAN_CLAUDE_MD"
  [[ "$PLAN_DOTREADME" != "skip" ]] && print_diff ".claude/README.md" "$DOTCLAUDE/README.md" "$PLAN_DOTREADME"
  print_diff ".claude/settings.json" "$SETTINGS" "$WORK/settings.json"
  print_diff ".gitignore" "$GITIGNORE" "$WORK/gitignore"
  say "done — dry run, nothing written"
  exit 0
fi

if [[ "$PLAN_CLAUDE_MD" == "skip" ]]; then
  note_skipped "CLAUDE.md (routing block present)"
else
  cp "$PLAN_CLAUDE_MD" "$CLAUDE_MD"
  note_touched "CLAUDE.md (routing block)"
fi

if [[ "$PLAN_DOTREADME" == "skip" ]]; then
  note_skipped ".claude/README.md (exists)"
else
  mkdir -p "$DOTCLAUDE"
  cp "$PLAN_DOTREADME" "$DOTCLAUDE/README.md"
  note_touched ".claude/README.md"
fi

if [[ "$MERGE_STATUS" == "changed" ]]; then
  if [[ -f "$SETTINGS" ]]; then
    mkdir -p "$BACKUP_DIR"
    cp -p "$SETTINGS" "$BACKUP_DIR/settings.json"
    note_touched ".claude/settings.json (backed up to ${BACKUP_DIR#"$REPO"/})"
  fi
  mkdir -p "$DOTCLAUDE"
  cp "$WORK/settings.json" "$SETTINGS"
  note_touched ".claude/settings.json (permissions.deny)"
else
  note_skipped ".claude/settings.json (permissions.deny present)"
fi

if [[ -f "$GITIGNORE" ]] && diff -q "$GITIGNORE" "$WORK/gitignore" >/dev/null; then
  note_skipped ".gitignore (entries present)"
else
  cp "$WORK/gitignore" "$GITIGNORE"
  note_touched ".gitignore"
fi

# --- summary -----------------------------------------------------------------
say "done — touched=${#TOUCHED[@]} skipped=${#SKIPPED[@]} drift=${#DRIFT[@]}"
for f in "${TOUCHED[@]:-}"; do [[ -n "$f" ]] && say "  touched $f"; done
for f in "${SKIPPED[@]:-}"; do [[ -n "$f" ]] && say "  skipped $f"; done
for f in "${DRIFT[@]:-}"; do [[ -n "$f" ]] && say "  drift  $f"; done
exit 0
