#!/usr/bin/env bash
#
# lint-skills.sh — validate .claude/skills/<slug>/SKILL.md against
# .claude/schemas/skills.schema.md (template-local; no dependency on the
# repo-root divisions.json or scripts/lint-agents.sh).
#
# The required frontmatter fields, required body sections, allowed divisions,
# and documented failure modes are parsed FROM the schema doc so the linter
# and the schema cannot drift. Validates:
#   1. required frontmatter fields present (name, description, division)
#   2. division is in the allowed enum
#   3. required body sections present (## headings, case-insensitive)
#   4. zero literal placeholder leak (<app>/<APP>/<scope>)
#   5. LF line endings, non-empty frontmatter
#
# Membership checks use `grep -q <<< "$var"` (here-string), NOT
# `echo "$var" | grep -q` — the pipe form races under `set -o pipefail`:
# grep -q can exit as soon as it finds a match, sending SIGPIPE to echo, and
# pipefail then reports the pipeline as failed even though grep matched. A
# here-string has no pipe to race, so the check is deterministic.
#
# Usage: bash scripts/lint-skills.sh [file ...]   (no args = scan all skills)

set -euo pipefail
cd "$(dirname "$0")/.."

SCHEMA=".claude/schemas/skills.schema.md"
SKILLS_DIR=".claude/skills"

if [[ ! -f "$SCHEMA" ]]; then
  echo "lint-skills: ERROR — schema doc $SCHEMA not found" >&2
  exit 2
fi
if [[ ! -d "$SKILLS_DIR" ]]; then
  echo "lint-skills: ERROR — skills dir $SKILLS_DIR not found" >&2
  exit 2
fi

# --- parse required fields/sections/divisions from the schema doc -------------
# Extract bullet list items under a given ## section header.
schema_section_items() {
  local header="$1"
  awk -v h="$header" '
    /^## / { in_sec = ($0 == "## " h) ; next }
    in_sec && /^- / { sub(/^- /, ""); print }
    in_sec && /^## / { exit }
  ' "$SCHEMA"
}

mapfile -t REQUIRED_FM < <(schema_section_items "Required Frontmatter Fields")
mapfile -t REQUIRED_SECTIONS < <(schema_section_items "Required Body Sections")
mapfile -t ALLOWED_DIV < <(schema_section_items "Allowed Divisions")

if [[ ${#REQUIRED_FM[@]} -eq 0 || ${#REQUIRED_SECTIONS[@]} -eq 0 || ${#ALLOWED_DIV[@]} -eq 0 ]]; then
  echo "lint-skills: ERROR — could not parse required sections from $SCHEMA" >&2
  exit 2
fi

# Failure modes are parsed and echoed as the documented catalog; each linter
# check below corresponds 1:1 to a documented mode.
mapfile -t FAILURE_MODES < <(schema_section_items "Validation Failure Modes")

errors=0
warnings=0
count=0

get_field() {
  # value of a YAML frontmatter field (first match), quotes stripped
  awk -v f="$1" 'NR==1{next} /^---$/{exit} index($0, f ": "){ sub("^[^:]*:[[:space:]]*",""); gsub(/["'"'"']/,""); print; exit }' "$2"
}

lint_file() {
  local file="$1"
  count=$((count + 1))
  if LC_ALL=C grep -q $'\r' "$file"; then
    echo "ERROR $file: CRLF line endings — convert to LF" >&2; errors=$((errors+1)); return; fi
  [[ "$(head -1 "$file")" == "---" ]] || { echo "ERROR $file: missing frontmatter opening ---" >&2; errors=$((errors+1)); return; }
  local fm; fm=$(awk 'NR==1{next} /^---$/{exit} {print}' "$file")
  [[ -n "$fm" ]] || { echo "ERROR $file: empty frontmatter" >&2; errors=$((errors+1)); return; }
  for field in "${REQUIRED_FM[@]}"; do
    grep -qE "^${field}:" <<< "$fm" || { echo "ERROR $file: missing frontmatter field '${field}'" >&2; errors=$((errors+1)); }
  done
  local div; div="$(get_field division "$file")"
  if [[ -n "$div" ]]; then
    local ok=0
    for d in "${ALLOWED_DIV[@]}"; do [[ "$div" == "$d" ]] && ok=1 && break; done
    [[ "$ok" -eq 1 ]] || { echo "ERROR $file: division '$div' not in allowed enum [${ALLOWED_DIV[*]}]" >&2; errors=$((errors+1)); }
  fi
  local body; body=$(awk 'BEGIN{n=0} /^---$/{n++; next} n>=2{print}' "$file")
  for section in "${REQUIRED_SECTIONS[@]}"; do
    grep -qi "# ${section}" <<< "$body" || { echo "ERROR $file: missing body section '${section}'" >&2; errors=$((errors+1)); }
  done
  # literal placeholder leak — fatal after rename, 0 residual.
  # Default is strict (post-rename). Set ALLOW_PLACEHOLDERS=1 to lint the
  # canonical template pre-rename (where <app>/<APP>/<scope> are by design).
  if [[ "${ALLOW_PLACEHOLDERS:-0}" != "1" ]]; then
    if grep -qE '<app>|<APP>|<scope>' "$file"; then
      echo "ERROR $file: literal placeholder leak (<app>/<APP>/<scope>) — rename.sh must substitute all placeholders" >&2
      errors=$((errors+1))
    fi
  fi
  local wc; wc=$(echo "$body" | wc -w | awk '{print $1}')
  [[ "${wc:-0}" -ge 50 ]] || { echo "WARN $file: body < 50 words" >&2; warnings=$((warnings+1)); }
}

files=()
if [[ $# -gt 0 ]]; then files=("$@")
else
  while IFS= read -r -d '' f; do files+=("$f"); done < <(find "$SKILLS_DIR" -type f -name 'SKILL.md' -print0 | sort -z)
fi

if [[ ${#files[@]} -eq 0 ]]; then
  echo "lint-skills: ERROR — no SKILL.md files found under $SKILLS_DIR" >&2
  exit 1
fi

echo "lint-skills: schema=$SCHEMA"
echo "lint-skills: required_fm=[${REQUIRED_FM[*]}] required_sections=[${REQUIRED_SECTIONS[*]}] allowed_div=[${ALLOWED_DIV[*]}]"
echo "lint-skills: documented failure modes: ${FAILURE_MODES[*]}"
echo "lint-skills: linting ${#files[@]} skill file(s)..."
echo ""
for file in "${files[@]}"; do lint_file "$file"; done
echo ""
echo "lint-skills: ${errors} error(s), ${warnings} warning(s) in ${#files[@]} file(s)."
if [[ $errors -gt 0 ]]; then
  echo "lint-skills: FAILED — fix errors before merging." >&2
  exit 1
fi
echo "lint-skills: PASSED"
exit 0
