#!/usr/bin/env bash
#
# lint-agents.sh — validate .claude/agents/<division>/<slug>.md agent files.
#   Pure bash (grep/awk). NO external dependency, NO JSON Schema lib.
#   Template-local: reads .claude/schemas/agents.schema.md for the required
#   frontmatter fields, required body sections, allowed divisions, so the
#   linter and the schema doc CANNOT drift.
#
# Extracted from the former inline lint_agents() block in verify-template.sh
# so the check can be invoked standalone (`bash scripts/lint-agents.sh [root]`)
# as well as from the gate. Behavior matches the former inline block plus one
# addition: an explicit dir==division check (the directory an agent file lives
# under must equal its own `division:` frontmatter value) — the inline block
# only globbed per-division directories, which could silently accept a file
# whose frontmatter division disagreed with its directory. The former block
# also only WARNed on a missing body section or a short body; both are
# promoted to ERROR here (all shipped agents already comply). Membership
# checks use `grep -q <<< "$var"` (here-string), NOT `echo "$var" | grep -q`
# — the pipe form races under `set -o pipefail`: grep -q can exit as soon as
# it finds a match, sending SIGPIPE to echo, and pipefail then reports the
# pipeline as failed even though grep matched. A here-string has no pipe to
# race, so the check is deterministic.
#
# Checks:
#   1. file exists
#   2. LF line endings (no CRLF)
#   3. leading --- frontmatter fence
#   4. non-empty frontmatter
#   5. every required frontmatter field present
#   6. division is in the allowed enum
#   7. division equals the directory the file lives under
#   8. every required body section heading present
#   9. body >= 50 words
#
# Usage: bash scripts/lint-agents.sh [root]
#   root  optional root directory containing .claude/ (default: the skeleton
#         root this script lives under). Lets verify-template.sh invoke this
#         against a staged/renamed copy via `[root]`, or a caller can `cd`
#         into the copy and omit the argument.
# Exit: 0 PASS, 1 errors, 2 misuse.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
ROOT="${1:-$DEFAULT_ROOT}"

SCHEMA="$ROOT/.claude/schemas/agents.schema.md"
[[ -f "$SCHEMA" ]] || { echo "lint-agents: ERROR — schema not found: $SCHEMA" >&2; exit 2; }

# extract_list <heading> — print the "- value" bullets that follow a
# "## <heading>" line in the schema doc (skipping the blank separator line),
# stopping at the next non-bullet, non-blank line. mawk-safe: no 3-arg
# match()/gensub(), plain regex + sub()/gsub() only.
extract_list() {
  local heading="$1"
  awk -v h="## ${heading}" '
    $0 == h { capturing=1; next }
    capturing && /^- / { sub(/^- /, ""); gsub(/[[:space:]]+$/, ""); print; next }
    capturing && /^[[:space:]]*$/ { next }
    capturing { capturing=0 }
  ' "$SCHEMA"
}

mapfile -t REQUIRED_FRONTMATTER < <(extract_list "Required Frontmatter Fields")
mapfile -t REQUIRED_SECTIONS    < <(extract_list "Required Body Sections")
mapfile -t ALLOWED_DIVISIONS    < <(extract_list "Allowed Divisions")

if [[ ${#REQUIRED_FRONTMATTER[@]} -eq 0 || ${#REQUIRED_SECTIONS[@]} -eq 0 || ${#ALLOWED_DIVISIONS[@]} -eq 0 ]]; then
  echo "lint-agents: ERROR — could not parse Required Frontmatter Fields/Required Body Sections/Allowed Divisions from $SCHEMA" >&2
  exit 2
fi
echo "lint-agents: schema parsed — ${#REQUIRED_FRONTMATTER[@]} frontmatter field(s), ${#REQUIRED_SECTIONS[@]} section(s), ${#ALLOWED_DIVISIONS[@]} division(s)."

errors=0
agent_files=()
shopt -s nullglob
for d in "${ALLOWED_DIVISIONS[@]}"; do
  for f in "$ROOT/.claude/agents/$d"/*.md; do
    agent_files+=("$f")
  done
done
shopt -u nullglob
if [[ ${#agent_files[@]} -eq 0 ]]; then
  echo "lint-agents: ERROR — no agent files under .claude/agents/{${ALLOWED_DIVISIONS[*]}}" >&2
  exit 1
fi

for f in "${agent_files[@]}"; do
  [[ -f "$f" ]] || { echo "ERROR $f: missing-file — not found"; errors=$((errors+1)); continue; }
  if LC_ALL=C grep -q $'\r' "$f"; then
    echo "ERROR $f: crlf — CRLF line endings, convert to LF"; errors=$((errors+1)); continue; fi
  [[ "$(head -1 "$f")" == "---" ]] || { echo "ERROR $f: missing-frontmatter-open — first line is not ---"; errors=$((errors+1)); continue; }
  fm="$(awk 'NR==1{next} /^---$/{exit} {print}' "$f")"
  [[ -n "$fm" ]] || { echo "ERROR $f: empty-frontmatter — frontmatter block is empty"; errors=$((errors+1)); continue; }
  for field in "${REQUIRED_FRONTMATTER[@]}"; do
    grep -qE "^${field}:" <<< "$fm" || { echo "ERROR $f: missing-frontmatter-field '${field}'"; errors=$((errors+1)); }
  done
  div="$(awk -v f=division 'NR==1{next} /^---$/{exit} index($0, f ": "){ sub("^[^:]*:[[:space:]]*",""); gsub(/["'"'"']/,""); print; exit }' "$f")"
  found=0
  for d in "${ALLOWED_DIVISIONS[@]}"; do [[ "$div" == "$d" ]] && found=1 && break; done
  if [[ -n "$div" && "$found" -eq 0 ]]; then
    echo "ERROR $f: bad-division — division '$div' not in Allowed Divisions [${ALLOWED_DIVISIONS[*]}]"; errors=$((errors+1)); fi
  # dir==division: the immediate parent directory name must equal the
  # frontmatter division value, so a file cannot be misfiled under a
  # different division's directory even if that division is itself allowed.
  dir_name="$(basename "$(dirname "$f")")"
  if [[ -n "$div" && "$found" -eq 1 && "$dir_name" != "$div" ]]; then
    echo "ERROR $f: bad-division — file lives under '${dir_name}/' but declares division '${div}' (dir must equal division)"; errors=$((errors+1)); fi
  body="$(awk 'BEGIN{n=0} /^---$/{n++; next} n>=2{print}' "$f")"
  for section in "${REQUIRED_SECTIONS[@]}"; do
    grep -qiF "$section" <<< "$body" || { echo "ERROR $f: missing-body-section '${section}'"; errors=$((errors+1)); }
  done
  wc="$(echo "$body" | wc -w | awk '{print $1}')"
  [[ "${wc:-0}" -ge 50 ]] || { echo "ERROR $f: short-body — ${wc} words (< 50)"; errors=$((errors+1)); }
done

echo "lint-agents: ${#agent_files[@]} file(s), ${errors} error(s)."
if [[ $errors -gt 0 ]]; then echo "lint-agents: FAILED — fix errors above."; exit 1; fi
echo "lint-agents: PASSED"; exit 0