#!/usr/bin/env bash
# verify-template.sh — smoke-test the iOS template end to end.
#
# The template ships with literal <app>/<APP>/<scope> placeholders, which are
# not valid Xcode target or bundle-id segments and cannot be built as shipped.
# This script stages a throwaway copy in a temp dir, renames it via
# scripts/rename.sh (example values), then runs the template's own portable
# gates on the instantiated copy: the three .claude linters, the instruction
# budget, the leak guard, a shell syntax check, the architecture gate, the
# node contract evals and the live-tier no-op guard. The Apple-toolchain tier
# runs when the machine has it and prints SKIP otherwise. The real working
# tree is never mutated.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="demo"
EXAMPLE_SCOPE="demoorg"

for tool in node tar; do
  command -v "$tool" >/dev/null || {
    echo "verify-template: $tool is required on PATH" >&2
    exit 1
  }
done

TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

STAGED="$TMP/kit"
failures=0

stage() {
  # stage <name> <command...> — run a gate and report one PASS/FAIL line.
  local name="$1"
  shift
  local log
  log="$(mktemp -t verify-template-stage.XXXXXX)"
  if "$@" >"$log" 2>&1; then
    echo "verify-template: PASS $name"
  else
    echo "verify-template: FAIL $name" >&2
    sed 's/^/  | /' "$log" >&2
    failures=$((failures + 1))
  fi
  rm -f "$log"
}

echo "verify-template: staging $STAGED"
cp -a "$ROOT_DIR" "$STAGED"

# The rename must reject a slug that cannot be an Xcode target or bundle-id
# segment BEFORE it touches the tree: a hyphenated slug yields an invalid
# bundle id and a slash aborts the substitution mid-tree.
rename_rejects_invalid_slug() {
  local scratch bad
  for bad in "my-app" "acme/app"; do
    scratch="$(mktemp -d -t verify-template-rename.XXXXXX)"
    cp -a "$STAGED" "$scratch/kit"
    if (cd "$scratch/kit" && bash scripts/rename.sh "$bad" "$EXAMPLE_SCOPE" >/dev/null 2>&1); then
      rm -rf "$scratch"
      return 1
    fi
    rm -rf "$scratch"
  done
  return 0
}
stage "rename rejects an invalid slug" rename_rejects_invalid_slug

echo "verify-template: rename"
(cd "$STAGED" && bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE" >/dev/null)

# --- .claude contract gates ---------------------------------------------------
stage "lint-agents" bash "$STAGED/scripts/lint-agents.sh" "$STAGED"
stage "lint-skills" bash "$STAGED/scripts/lint-skills.sh"
stage "lint-commands" bash "$STAGED/scripts/lint-commands.sh"

# The always-on instruction set (CLAUDE.md + @imports + path-less rules) must
# stay under the harness budget; a playbook belongs behind a path-scoped rule.
stage "instruction-budget" bash "$STAGED/scripts/instruction-budget.sh" "$STAGED"

# Post-rename placeholder sweep. The secret scan is skipped: the staged copy
# is a throwaway of the template, which carries no secrets.
stage "leak-guard" bash "$STAGED/scripts/leak-guard.sh" --placeholders-only

# Every shell script in the kit must parse.
shell_syntax() {
  local script
  while IFS= read -r script; do
    bash -n "$script" || return 1
  done < <(find "$STAGED" -name '*.sh' -type f -not -path '*/DerivedData/*' | sort)
}
stage "shell-syntax" shell_syntax

# run-live.sh must be a no-op without EVALS=1 — it costs API budget and needs
# a live claude CLI, so it is never part of the default gate.
stage "run-live no-op guard" bash "$STAGED/evals/run-live.sh"

# --- architecture -------------------------------------------------------------
stage "check-architecture" bash "$STAGED/scripts/check-architecture.sh"

# The contract evals run on the RENAMED copy: the rename-state sweep only
# activates once evals/package.json carries the substituted name.
stage "evals (node --test)" node --test "$STAGED/evals/contracts.test.mjs"

# --- Apple-toolchain tier -----------------------------------------------------
# A visible SKIP, not silence: on Linux or without Xcode the native proof is
# deferred to macOS/CI, and the stage still reports its outcome.
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "verify-template: SKIP native-gates (macOS + Xcode required)"
else
  stage "native-gates" bash "$STAGED/scripts/native-gates.sh"
fi

if [[ $failures -gt 0 ]]; then
  echo "verify-template: FAILED — ${failures} stage(s) failed." >&2
  exit 1
fi

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
