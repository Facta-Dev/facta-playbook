#!/usr/bin/env bash
# verify-template.sh — smoke-test the <app> Chrome extension skeleton end to end.
#
# The skeleton ships with literal <app>/<APP>/<scope> placeholders, so the
# gates cannot run as shipped. This script stages a throwaway copy of the
# template in a temp dir, renames it via scripts/rename.sh (example values),
# then runs the template's own gates on the instantiated copy: the placeholder
# leak guard, the three .claude linters, the instruction budget, a shell
# syntax sweep, the pnpm gates (install, typecheck, lint, test, build) and the
# native contract evals. The real working tree is never mutated. Exits
# nonzero if any stage fails. Idempotent.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="exampletool"
EXAMPLE_SCOPE="factadev"

for tool in node pnpm tar; do
  command -v "$tool" >/dev/null || {
    echo "verify-template: $tool is required on PATH" >&2
    exit 1
  }
done

TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

failures=0

# stage runs one named stage inside the staged copy and prints exactly one
# PASS/FAIL line for it. A failing stage does not abort the run: the point is
# to report every broken gate in one pass.
stage() {
  local name="$1"
  shift
  if (cd "$TMP" && "$@"); then
    echo "PASS  $name"
  else
    echo "FAIL  $name" >&2
    failures=$((failures + 1))
  fi
}

# shell_syntax runs bash -n over every shell script of the staged copy.
shell_syntax() {
  find . -type f -name '*.sh' -not -path './node_modules/*' -print0 |
    xargs -0 -r -n1 bash -n
}

echo "verify-template: staging $TMP"
(cd "$ROOT_DIR" && tar cf - \
  --exclude='./.git' \
  --exclude='./node_modules' \
  --exclude='./dist' \
  --exclude='./coverage' \
  .) | (cd "$TMP" && tar xf -)

stage "rename" bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE"

stage "leak-guard (placeholders)" bash scripts/leak-guard.sh --placeholders-only

stage "lint-agents" bash scripts/lint-agents.sh
stage "lint-skills" bash scripts/lint-skills.sh
stage "lint-commands" bash scripts/lint-commands.sh

stage "instruction-budget" bash scripts/instruction-budget.sh

stage "bash -n (shell scripts)" shell_syntax

stage "pnpm install" pnpm install
stage "typecheck" pnpm run typecheck
stage "lint" pnpm run lint
stage "test" pnpm run test
stage "build" pnpm run build

# The contract evals run on the RENAMED copy: the rename-state sweep only
# activates once evals/package.json carries the substituted name.
stage "evals (node --test)" node --test evals/contracts.test.mjs

# run-live.sh must be a no-op without EVALS=1 — it costs API budget and needs
# a live claude CLI, neither of which belongs in the default gate.
stage "run-live no-op guard" bash -c "bash evals/run-live.sh"

if [[ $failures -gt 0 ]]; then
  echo "verify-template: FAIL — $failures stage(s) failed." >&2
  exit 1
fi

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
