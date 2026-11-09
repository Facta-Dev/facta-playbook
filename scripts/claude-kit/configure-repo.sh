#!/usr/bin/env bash
#
# configure-repo.sh — wire one repository to the playbook (idempotent).
#
# It never clobbers: existing files are patched only where the change is
# additive and marker-guarded. --check verifies a repo without writing.
#
# Usage:
#   bash scripts/claude-kit/configure-repo.sh <repo> [--kit=golang|nodejs|python|rust]
#        [--dry-run] [--check]
#     <repo>       path to the repository root
#     --kit=NAME   also install that language kit (golang|nodejs|python|rust)
#     --dry-run    print the diff it would apply, write nothing
#     --check      verify the repo, write nothing; exit 1 if anything is missing
#
# Exit 0 = configured (or already configured), 1 = a check failed or a write failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

REPO=""
KIT=""
DRY_RUN=0
CHECK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --kit=*)   KIT="${1#--kit=}"; shift;;
    --dry-run) DRY_RUN=1; shift;;
    --check)   CHECK=1; shift;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    -*)        echo "configure-repo: unknown arg '$1'" >&2; exit 1;;
    *)         REPO="$1"; shift;;
  esac
done

if [[ -z "$REPO" ]]; then
  echo "configure-repo: ERROR — repo path required" >&2
  exit 1
fi
if [[ ! -d "$REPO" ]]; then
  echo "configure-repo: ERROR — no such directory: $REPO" >&2
  exit 1
fi
REPO="$(cd "$REPO" && pwd)"
if [[ -n "$KIT" && ! -d "$ROOT_DIR/$KIT" ]]; then
  echo "configure-repo: ERROR — unknown kit '$KIT' (golang|nodejs|python|rust)" >&2
  exit 1
fi

MARKER='<!-- model-routing:ollama -->'
CLAUDE_MD="$REPO/CLAUDE.md"
DOTCLAUDE="$REPO/.claude"
GITIGNORE="$REPO/.gitignore"

say() { echo "configure-repo: $*"; }
warn() { echo "configure-repo: WARN — $*" >&2; }

# --- the per-repo routing block (CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md §6) ------
ROUTING_BLOCK="$(cat <<'EOF'
<!-- model-routing:ollama -->
## Model routing (Claude Code + Ollama)

Runbook: `facta-playbook/CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md`.

- Tier aliases resolve through `~/.claude/settings.json`: `opus` = escalation
  (`deepseek-v4-pro:cloud`), `sonnet` = workhorse (`glm-5.3-flash:cloud`),
  `haiku` = cheap digging (`minimax-m3.1-flash:cloud`).
- Consult the `advisor` subagent before locking a plan, when the same error
  reappears twice, and before declaring a task done. Pass concrete context
  (plan, diff, test output) — not the whole repo.
- Escalate with `/model opus` for design decisions or after two failed
  attempts; return to `sonnet` when the task is done.
- All code, comments, tests, specs, and documentation are generated in English.
EOF
)"

# --- the project-scope deny list (PLAYBOOK_CLAUDE_CODE.md A.6) -----------------
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
  '.claude/install-state.json'
  '.claude/settings.local.json'
  'logs/'
)

DOTCLAUDE_README="$(cat <<'EOF'
# .claude — this repo's Claude Code kit

Do not restate the playbooks here; read them from the playbook repo.

- `PLAYBOOK_CLAUDE_CODE.md` — the kit this repo must have
- `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md` — model tiers and the per-repo routing block
- `PLAYBOOK_INSTALL.md` — install and bootstrap runbook

Repo-local kit files live next to this file: `agents/`, `skills/`, `commands/`,
`rules/`, `settings.json`. Verify the wiring with:

```bash
bash ~/facta-playbook/scripts/claude-kit/configure-repo.sh <repo> --check
```
EOF
)"

# --- checks --------------------------------------------------------------------
check_all() {
  local rc=0 entry missing
  if [[ -f "$CLAUDE_MD" ]] && grep -qF "$MARKER" "$CLAUDE_MD"; then
    say "ok   routing block"
  else
    say "MISS routing block ($CLAUDE_MD)"; rc=1
  fi
  if [[ -f "$DOTCLAUDE/README.md" ]]; then
    say "ok   .claude/README.md"
  else
    say "MISS .claude/README.md"; rc=1
  fi
  if [[ -f "$DOTCLAUDE/settings.json" ]] && grep -qF 'Read(./.env)' "$DOTCLAUDE/settings.json"; then
    say "ok   deny list"
  else
    say "MISS deny list (.claude/settings.json)"; rc=1
  fi
  missing=0
  [[ -f "$GITIGNORE" ]] || missing=1
  if [[ "$missing" -eq 0 ]]; then
    for entry in "${GITIGNORE_ENTRIES[@]}"; do
      grep -qxF "$entry" "$GITIGNORE" || { say "MISS gitignore entry: $entry"; missing=1; }
    done
  else
    say "MISS .gitignore"
  fi
  if [[ "$missing" -eq 0 ]]; then
    say "ok   .gitignore entries"
  else
    rc=1
  fi
  if [[ -n "$KIT" ]]; then
    if [[ -d "$DOTCLAUDE/agents" ]]; then
      say "ok   $KIT kit agents"
    else
      say "MISS $KIT kit agents (.claude/agents)"; rc=1
    fi
  fi
  return "$rc"
}

if [[ "$CHECK" -eq 1 ]]; then
  say "check $REPO"
  check_all
  exit $?
fi

# --- 1. language kit (must run first: the kit installer skips existing files) ---
if [[ -n "$KIT" ]]; then
  INSTALLER="$ROOT_DIR/$KIT/scripts/install-claude-runtime.sh"
  if [[ ! -f "$INSTALLER" ]]; then
    echo "configure-repo: ERROR — kit installer not found: $INSTALLER" >&2
    exit 1
  fi
  say "installing $KIT kit"
  # The guard hook (scripts/hooks + the PreToolUse entry in settings.json) is
  # installed by the claude-project target only, so both targets are needed.
  # ponytail: the kit installer resolves runtime_config_dirs from the manifest
  # with a broken awk lookup, so claude-project lands in a duplicate
  # '.claude-project/' instead of '.claude/'. We drop that duplicate below;
  # upgrade path = fix runtime_dir() in <kit>/scripts/install-claude-runtime.sh.
  HAD_PROJECT_DIR=0
  [[ -d "$REPO/.claude-project" ]] && HAD_PROJECT_DIR=1
  bash "$INSTALLER" --targets=claude,claude-project --dest="$REPO"
  if [[ "$HAD_PROJECT_DIR" -eq 0 && -d "$REPO/.claude-project" ]]; then
    rm -rf "$REPO/.claude-project"
    say "removed the duplicate .claude-project/ (kit installer manifest bug)"
  fi
fi

# --- 2. routing block (marker-guarded) ----------------------------------------
if [[ -f "$CLAUDE_MD" ]] && grep -qF "$MARKER" "$CLAUDE_MD"; then
  say "skip routing block (marker present)"
elif [[ "$DRY_RUN" -eq 1 ]]; then
  say "would append the routing block to CLAUDE.md"
else
  printf '\n%s\n' "$ROUTING_BLOCK" >> "$CLAUDE_MD"
  say "routing block appended to CLAUDE.md"
fi

# --- 3. .claude/README.md ------------------------------------------------------
if [[ -f "$DOTCLAUDE/README.md" ]]; then
  say "skip .claude/README.md (exists)"
elif [[ "$DRY_RUN" -eq 1 ]]; then
  say "would write $DOTCLAUDE/README.md"
else
  mkdir -p "$DOTCLAUDE"
  printf '%s\n' "$DOTCLAUDE_README" > "$DOTCLAUDE/README.md"
  say "wrote $DOTCLAUDE/README.md"
fi

# --- 4. deny list into .claude/settings.json (deny only, never allow/env/hooks) --
if [[ "$DRY_RUN" -eq 1 ]]; then
  say "would merge permissions.deny into $DOTCLAUDE/settings.json"
else
  mkdir -p "$DOTCLAUDE"
  python3 - "$DOTCLAUDE/settings.json" "${DENY_RULES[@]}" <<'PY'
import json, sys

path = sys.argv[1]
rules = sys.argv[2:]

try:
    with open(path) as fh:
        settings = json.load(fh)
except FileNotFoundError:
    settings = {}
except (OSError, ValueError) as exc:
    sys.exit("configure-repo: ERROR — invalid settings.json: %s" % exc)

if not isinstance(settings, dict):
    sys.exit("configure-repo: ERROR — settings.json is not a JSON object")

perms = settings.setdefault("permissions", {})
if not isinstance(perms, dict):
    sys.exit("configure-repo: ERROR — permissions is not an object")
deny = perms.setdefault("deny", [])
if not isinstance(deny, list):
    sys.exit("configure-repo: ERROR — permissions.deny is not a list")
for rule in rules:
    if rule not in deny:
        deny.append(rule)

with open(path, "w") as fh:
    json.dump(settings, fh, indent=2)
    fh.write("\n")
PY
  say "merged deny list into $DOTCLAUDE/settings.json"
fi

# --- 5. .gitignore entries -----------------------------------------------------
if [[ -f "$GITIGNORE" ]]; then
  missing=0
  for entry in "${GITIGNORE_ENTRIES[@]}"; do
    grep -qxF "$entry" "$GITIGNORE" || missing=1
  done
  if [[ "$missing" -eq 1 ]]; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
      say "would append the missing .gitignore entries"
    else
      for entry in "${GITIGNORE_ENTRIES[@]}"; do
        grep -qxF "$entry" "$GITIGNORE" || printf '%s\n' "$entry" >> "$GITIGNORE"
      done
      say "appended the missing .gitignore entries"
    fi
  else
    say "skip .gitignore (entries present)"
  fi
elif [[ "$DRY_RUN" -eq 1 ]]; then
  say "would create .gitignore"
else
  : > "$GITIGNORE"
  for entry in "${GITIGNORE_ENTRIES[@]}"; do
    printf '%s\n' "$entry" >> "$GITIGNORE"
  done
  say "created .gitignore"
fi

say "done — repo=$REPO"
exit 0
