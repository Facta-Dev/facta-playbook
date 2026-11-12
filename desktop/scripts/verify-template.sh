#!/usr/bin/env bash
# verify-template.sh — smoke-test the desktop Qt template end to end.
#
# The template ships with literal <app>/<APP>/<scope> placeholders, which are
# not valid CMake target names and cannot be built as shipped. This script
# stages a throwaway copy in a temp dir, renames it via scripts/rename.sh
# (example values), then runs the template's own gates on the instantiated
# copy: the three .claude linters, the instruction budget, a shell syntax
# check, the CMake configure/build, the headless test run and the evals
# harness. The real working tree is never mutated. Exits nonzero on any
# failure and removes the staged copy on the way out.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="demo"
EXAMPLE_SCOPE="demoorg"

failures=0

stage() {
  # stage <name> <command...> — run a gate and report one PASS/FAIL line.
  # A failing stage keeps its output: the reason a gate failed is the whole
  # point of the line.
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

command -v cmake >/dev/null || { echo "verify-template: cmake is required on PATH" >&2; exit 1; }

TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

STAGED="$TMP/kit"
BUILD="$TMP/build"

echo "verify-template: staging $STAGED"
cp -a "$ROOT_DIR" "$STAGED"

echo "verify-template: rename"
(cd "$STAGED" && bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE" >/dev/null)

# --- .claude contract gates ---------------------------------------------------
# lint-agents.sh takes the tree root as its argument; the other two cd into
# their own parent directory and take optional file arguments.
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
  done < <(find "$STAGED" -name '*.sh' -type f -not -path '*/build/*' | sort)
}
stage "shell-syntax" shell_syntax

# --- build and test -----------------------------------------------------------
echo "verify-template: cmake configure"
stage "cmake-configure" cmake -S "$STAGED" -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Debug -DENABLE_TESTS=ON -DEVALS=ON

echo "verify-template: cmake build"
stage "cmake-build" cmake --build "$BUILD" -j "$(nproc 2>/dev/null || echo 4)"

echo "verify-template: ctest (offscreen)"
stage "ctest" env QT_QPA_PLATFORM=offscreen ctest --test-dir "$BUILD" --output-on-failure

# The evals harness is its own tree: it validates the .claude layer and the
# rename state of the tree it was built from.
echo "verify-template: evals"
stage "evals" env QT_QPA_PLATFORM=offscreen ctest --test-dir "$BUILD" \
  --output-on-failure -R desktop_evals_contracts

if [[ $failures -gt 0 ]]; then
  echo "verify-template: FAILED — ${failures} stage(s) failed." >&2
  exit 1
fi

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
