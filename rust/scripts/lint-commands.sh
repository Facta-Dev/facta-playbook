#!/usr/bin/env bash
#
# lint-commands.sh — validate .claude/commands/<cmd>.md against
# .claude/schemas/commands.schema.md (template-local; no dependency on the
# repo-root divisions.json or scripts/lint-agents.sh).
#
# The required frontmatter fields, required body sections, allowed divisions,
# and documented failure modes are parsed FROM the schema doc so the linter
# and the schema cannot drift. Validates:
#   1. required frontmatter fields present (name, description, division)
#   2. division is in the allowed enum
#   3. required body section present (Delegate)
#   4. command slug == a skill slug (1:1 mapping)
#   5. zero literal placeholder leak (<app>/<APP>/<scope>)
#   6. LF line endings, non-empty frontmatter
#
# Membership checks use `grep -q <<< "$var"` (here-string), NOT
# `echo "$var" | grep -q` — the pipe form races under `set -o pipefail`:
# grep -q can exit as soon as it finds a match, sending SIGPIPE to echo, and
# pipefail then reports the pipeline as failed even though grep matched. A
# here-string has no pipe to race, so the check is deterministic.
#
# Usage: bash scripts/lint-commands.sh [file ...]   (no args = scan all commands)

set -euo pipefail
cd "$(dirname "$0")/.."

SCHEMA=".claude/schemas/commands.schema.md"
COMMANDS_DIR=".claude/commands"
SKILLS_DIR=".claude/skills"

if [[ ! -f "$SCHEMA" ]]; then
  echo "lint-commands: ERROR — schema doc $SCHEMA not found" >&2
  exit 2
fi
if [[ ! -d "$COMMANDS_DIR" ]]; then
  echo "lint-commands: ERROR — commands dir $COMMANDS_DIR not found" >&2
  exit 2
fi

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
mapfile -t FAILURE_MODES < <(schema_section_items "Validation Failure Modes")

if [[ ${#REQUIRED_FM[@]} -eq 0 || ${#REQUIRED_SECTIONS[@]} -eq 0 || ${#ALLOWED_DIV[@]} -eq 0 ]]; then
  echo "lint-commands: ERROR — could not parse required sections from $SCHEMA" >&2
  exit 2
fi

errors=0
warnings=0
count=0

get_field() {
  awk -v f="$1" 'NR==1{next} /^---$/{exit} index($0, f ": "){ sub("^[^:]*:[[:space:]]*",""); gsub(/["'"'"']/,""); print; exit }' "$2"
}

# Skill slugs present in the canonical skills dir (1:1 mapping target set).
skill_slugs=()
if [[ -d "$SKILLS_DIR" ]]; then
  while IFS= read -r -d '' d; do
    skill_slugs+=("$(basename "$d")")
  done < <(find "$SKILLS_DIR" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)
fi

has_skill() {
  local s="$1"
  for sl in "${skill_slugs[@]}"; do [[ "$sl" == "$s" ]] && return 0; done
  return 1
}

lint_file() {
  local file="$1"
  count=$((count + 1))
  local cmd_slug; cmd_slug="$(basename "$file" .md)"
  if LC_ALL=C grep -q $'\r' "$file"; then
    echo "ERROR $file: CRLF line endings — convert to LF" >&2; errors=$((errors+1)); return; fi
  [[ "$(head -1 "$file")" == "---" ]] || { echo "ERROR $file: missing frontmatter opening ---" >&2; errors=$((errors+1)); return; }
  local fm; fm=$(awk 'NR==1{next} /^---$/{exit} {print}' "$file")
  [[ -n "$fm" ]] || { echo "ERROR $file: empty frontmatter" >&2; errors=$((errors+1)); return; }
  for field in "${REQUIRED_FM[@]}"; do
    grep -qE "^${field}:" <<< "$fm" || { echo "ERROR $file: missing frontmatter field '${field}'" >&2; errors=$((errors+1)); }
  done
  # name field must equal the command slug (1:1 with skill)
  local name_val; name_val="$(get_field name "$file")"
  if [[ -n "$name_val" && "$name_val" != "$cmd_slug" ]]; then
    echo "ERROR $file: name '$name_val' != command slug '$cmd_slug' (must equal skill slug)" >&2
    errors=$((errors+1))
  fi
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
  # command slug must map 1:1 to a skill slug
  if [[ ${#skill_slugs[@]} -gt 0 ]]; then
    if ! has_skill "$cmd_slug"; then
      echo "ERROR $file: command slug '$cmd_slug' has no matching skill in $SKILLS_DIR (1:1 mapping required)" >&2
      errors=$((errors+1))
    fi
  else
    echo "WARN $file: no skills dir found — cannot verify 1:1 slug mapping" >&2
    warnings=$((warnings+1))
  fi
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
  [[ "${wc:-0}" -ge 5 ]] || { echo "WARN $file: body < 5 words" >&2; warnings=$((warnings+1)); }
}

files=()
if [[ $# -gt 0 ]]; then files=("$@")
else
  while IFS= read -r -d '' f; do files+=("$f"); done < <(find "$COMMANDS_DIR" -maxdepth 1 -type f -name '*.md' -print0 | sort -z)
fi

if [[ ${#files[@]} -eq 0 ]]; then
  echo "lint-commands: ERROR — no command .md files found under $COMMANDS_DIR" >&2
  exit 1
fi

echo "lint-commands: schema=$SCHEMA"
echo "lint-commands: required_fm=[${REQUIRED_FM[*]}] required_sections=[${REQUIRED_SECTIONS[*]}] allowed_div=[${ALLOWED_DIV[*]}]"
echo "lint-commands: documented failure modes: ${FAILURE_MODES[*]}"
echo "lint-commands: linting ${#files[@]} command file(s)..."
echo ""
for file in "${files[@]}"; do lint_file "$file"; done
echo ""
echo "lint-commands: ${errors} error(s), ${warnings} warning(s) in ${#files[@]} file(s)."
if [[ $errors -gt 0 ]]; then
  echo "lint-commands: FAILED — fix errors before merging." >&2
  exit 1
fi
echo "lint-commands: PASSED"
exit 0
