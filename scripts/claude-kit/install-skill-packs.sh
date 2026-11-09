#!/usr/bin/env bash
#
# install-skill-packs.sh — register and install the skill-pack plugins.
#
# Shell only (no model tokens). Idempotent: an installed plugin is skipped.
# The marketplace source is the local clone when it exists, so `git pull` in
# the clone updates the plugin; otherwise the GitHub repo is registered.
#
# Usage:
#   bash scripts/claude-kit/install-skill-packs.sh [--dir=DIR] [--remote]
#        [--only=NAME[,NAME]] [--dry-run] [--self-test]
#     --dir=DIR    working-set root (default ~/work)
#     --remote     always register the GitHub source, never the local clone
#     --only=...   run only these marketplace names
#     --dry-run    print the install commands, change nothing
#     --self-test  check the table against the local marketplace.json files
#
# Exit 0 = all installed or skipped, 1 = at least one install failed.
#
# ponytail: `ponytail` is deliberately absent -- the baseline already enables
# ponytail@ponytail from upstream DietrichGebert/ponytail, and the Facta-Dev
# fork shares the marketplace name. `awesome-claude-skills` has no
# .claude-plugin/marketplace.json, so it is clone-only.

set -euo pipefail

DIR="$HOME/work"
REMOTE=0
DRY_RUN=0
ONLY=""
SELF_TEST=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir=*)   DIR="${1#--dir=}"; shift;;
    --dir)     DIR="${2:-}"; shift 2;;
    --remote)  REMOTE=1; shift;;
    --only=*)  ONLY="${1#--only=}"; shift;;
    --only)    ONLY="${2:-}"; shift 2;;
    --dry-run) DRY_RUN=1; shift;;
    --self-test) SELF_TEST=1; shift;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "install-skill-packs: unknown arg '$1'" >&2; exit 1;;
  esac
done

# repo|marketplace|comma-separated plugins (verified against marketplace.json)
PACKS=(
  "beautifulpython|beautiful-python|beautiful-python"
  "beautifulrust|beautiful-rust|beautiful-rust"
  "beautifulgo|beautiful-go|beautiful-go"
  "beautifulnodejs|beautiful-nodejs|beautiful-nodejs"
  "skills|mattpocock|mattpocock-skills"
  "claude-skill|motionsites|motionsites-web-builder"
  "garden-skills|garden-skills|presentation-skills,web-design-skills,knowledge-base-skills,image-generation-skills,motion-showreel-skills,beautiful-article-skills"
  "ui-ux-pro-max-skill|ui-ux-pro-max-skill|ui-ux-pro-max"
  "superpowers|superpowers-dev|superpowers"
)

if [[ "$SELF_TEST" -eq 1 ]]; then
  python3 - "$DIR" "${PACKS[@]}" <<'PY'
import json, os, sys
root, rows = sys.argv[1], sys.argv[2:]
bad = checked = 0
for row in rows:
    repo, market, plugins = row.split("|")
    path = os.path.join(root, repo, ".claude-plugin", "marketplace.json")
    if not os.path.isfile(path):
        continue  # not cloned here: nothing to compare
    checked += 1
    data = json.load(open(path))
    names = {p.get("name") for p in data.get("plugins", [])}
    if data.get("name") != market:
        print(f"FAIL {repo}: marketplace {data.get('name')} != {market}")
        bad = 1
    for plugin in plugins.split(","):
        if plugin not in names:
            print(f"FAIL {repo}: plugin {plugin} not in marketplace.json")
            bad = 1
print(f"self-test: {checked} marketplace(s) checked")
sys.exit(bad)
PY
  exit $?
fi

command -v claude >/dev/null || { echo "install-skill-packs: claude not on PATH" >&2; exit 2; }

INSTALLED="$(claude plugin list --json 2>/dev/null \
  | python3 -c 'import json,sys;print("\n".join(p["id"] for p in json.load(sys.stdin)))' || true)"

install=0
skip=0
fail=0

for row in "${PACKS[@]}"; do
  IFS='|' read -r repo market plugins <<<"$row"
  if [[ -n "$ONLY" && ",$ONLY," != *",$market,"* ]]; then continue; fi
  src="$repo"
  if [[ "$REMOTE" -eq 0 && -d "$DIR/$repo/.claude-plugin" ]]; then src="$DIR/$repo"; fi
  IFS=',' read -r -a list <<<"$plugins"
  for plugin in "${list[@]}"; do
    id="$plugin@$market"
    if grep -qxF "$id" <<<"$INSTALLED"; then
      echo "skip     $id (installed)"
      skip=$((skip + 1))
      continue
    fi
    cmd=(claude plugin install "$plugin" --marketplace "$src" --scope user -y --json)
    if [[ "$DRY_RUN" -eq 1 ]]; then
      printf 'would run'; printf ' %q' "${cmd[@]}"; printf '\n'
      install=$((install + 1))
      continue
    fi
    if out="$("${cmd[@]}" 2>&1)"; then
      echo "install  $id <- $src"
      install=$((install + 1))
    else
      echo "FAIL     $id <- $src: $(printf '%s' "$out" | tail -1)" >&2
      fail=$((fail + 1))
    fi
  done
done

echo "done — install=$install skipped=$skip failed=$fail"
[[ "$fail" -eq 0 ]]
