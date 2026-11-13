#!/usr/bin/env bash
# verify-template.sh — smoke-test the <app> Java skeleton end to end.
# Stages a throwaway copy, renames it, asserts no literal placeholder survives
# where it must not, runs the .claude gates and (when Maven is installed) the
# Maven gate. The working tree is never mutated.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="exampleapp"
EXAMPLE_SCOPE="factadev"
TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

echo "verify-template: staging $TMP"
(cd "$ROOT_DIR" && tar cf - --exclude='./.git' --exclude='./target' .) | (cd "$TMP" && tar xf -)

echo "verify-template: rename"
(cd "$TMP" && bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE" >/dev/null)

survivors=$(grep -rlE '<app>|<APP>|<scope>' "$TMP" \
  --exclude-dir=scripts --exclude-dir=.claude --exclude-dir=evals \
  --exclude-dir=.git --exclude-dir=target --exclude=pom.xml 2>/dev/null || true)
if [[ -n "$survivors" ]]; then
  echo "verify-template: FAIL — literal placeholders survived the rename:" >&2
  echo "$survivors" >&2
  exit 1
fi

echo "verify-template: .claude linters"
bash "$TMP/scripts/lint-agents.sh" "$TMP" >/dev/null
(cd "$TMP" && bash scripts/lint-skills.sh) >/dev/null
(cd "$TMP" && bash scripts/lint-commands.sh) >/dev/null
(cd "$TMP" && bash scripts/leak-guard.sh) >/dev/null
(cd "$TMP" && bash scripts/instruction-budget.sh) >/dev/null
grep -q 'PreToolUse' "$TMP/.claude/settings.json" \
  || { echo "verify-template: FAIL — settings.json does not wire PreToolUse" >&2; exit 1; }
while IFS= read -r hook; do bash -n "$hook"; done < <(find "$TMP/scripts/hooks" -name '*.sh' 2>/dev/null)

echo "verify-template: evals"
(cd "$TMP" && python3 evals/contracts_test.py)

echo "verify-template: Maven gate"
if command -v mvn >/dev/null 2>&1; then
  (cd "$TMP" && mvn -q verify)
else
  echo "verify-template: SKIP Maven — not installed (CI runs it)"
fi

if [[ -f "$TMP/evals/run-live.sh" ]]; then
  (cd "$TMP" && bash evals/run-live.sh >/dev/null 2>&1) \
    || { echo "verify-template: FAIL — run-live.sh must no-op without EVALS=1" >&2; exit 1; }
fi

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
