#!/usr/bin/env bash
# instruction-budget.sh — fail when the always-on instruction set exceeds the
# harness budget, or when one file would be truncated.
#
# The harness loads CLAUDE.md (or AGENTS.md), every `@file` import it resolves,
# and every `.claude/rules/*.md` without a `paths:` frontmatter on *every*
# turn. Two limits apply: 40,000 chars per file (a larger file is truncated to
# its start) and 120,000 chars for the whole always-on set. Playbooks that are
# consulted on demand must be pointed at from a path-scoped rule, never
# `@`-imported wholesale.
#
# Usage: bash scripts/instruction-budget.sh [ROOT]
# Exit: 0 under budget (warnings allowed), 1 over budget or a truncated file.
set -euo pipefail

ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
ROOT="$(cd "$ROOT" && pwd -P)"
PER_FILE_LIMIT=40000
TOTAL_LIMIT=120000
WARN_LIMIT=100000

abs() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$(pwd -P)" "$(basename "$1")"); }

files=()
seen='
'
# add FILE — dedupe by physical path (CLAUDE.md is usually a symlink to
# AGENTS.md, and an import may repeat a file already collected).
add() {
  local f="$1" a
  [[ -f "$f" ]] || return 0
  a="$(abs "$f")"
  case "$seen" in *"
$a
"*) return 0 ;; esac
  seen="$seen$a
"
  files+=("$a")
}

root_doc=""
if [[ -f "$ROOT/CLAUDE.md" ]]; then
  root_doc="$ROOT/CLAUDE.md"
elif [[ -f "$ROOT/AGENTS.md" ]]; then
  root_doc="$ROOT/AGENTS.md"
fi
if [[ -z "$root_doc" ]]; then
  echo "instruction-budget: no CLAUDE.md/AGENTS.md under $ROOT — nothing always-on."
  exit 0
fi
add "$root_doc"

# Resolve @imports transitively (relative to the importing file).
i=0
while (( i < ${#files[@]} )); do
  f="${files[$i]}"; i=$((i + 1))
  while IFS= read -r inc; do
    [[ -n "$inc" ]] && add "$(dirname "$f")/$inc"
  done < <(grep -E '^@[^[:space:]]+[[:space:]]*$' "$f" | sed 's/^@//' || true)
done

# Rules with `paths:` load on demand; the rest load like CLAUDE.md.
shopt -s nullglob
for r in "$ROOT"/.claude/rules/*.md; do
  head -20 "$r" | grep -qE '^paths:' || add "$r"
done
shopt -u nullglob

status=0
total=0
printf '%-52s %8s\n' "always-on instruction file" "chars"
for f in "${files[@]}"; do
  n=$(wc -c < "$f")
  total=$((total + n))
  printf '%-52s %8d\n' "${f#"$ROOT"/}" "$n"
  if (( n > PER_FILE_LIMIT )); then
    echo "  ^ over ${PER_FILE_LIMIT} chars: the harness keeps only the start" >&2
    status=1
  fi
done
printf '%-52s %8d\n' "TOTAL (limit ${TOTAL_LIMIT})" "$total"

if (( total > TOTAL_LIMIT )); then
  echo "instruction-budget: FAIL — ${total} chars, over the ${TOTAL_LIMIT} limit." >&2
  echo "Move reference playbooks to on-demand: point at them from .claude/rules/*.md" >&2
  echo "with a paths: frontmatter instead of @-importing them." >&2
  status=1
elif (( total > WARN_LIMIT )); then
  echo "instruction-budget: WARN — ${total} chars; headroom under ${TOTAL_LIMIT} is thin." >&2
fi

if grep -qE '^@PLAYBOOK_' "$root_doc"; then
  echo "instruction-budget: WARN — $root_doc @-imports a playbook;" >&2
  echo "import the digest, not the set: path-scoped rules are the supported loader." >&2
fi

exit "$status"
