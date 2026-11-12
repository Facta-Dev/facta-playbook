#!/usr/bin/env bash
# run-live.sh — live-tier evals for the desktop .claude skills layer.
#
# Distinct from the offline Qt Test suite in evals/ (fixture-only, no
# network, wired into scripts/verify-template.sh). This script drives the
# Claude Code CLI against a live skill invocation and, optionally, an
# LLM-judge pass. It is deliberately kept OUTSIDE verify-template.sh and the
# default CI lanes — it costs real API budget and needs a live `claude` CLI,
# neither of which belong in the default gate. Trigger it by hand.
#
# Deliberately placeholder-free: this file references a skill slug, never an
# app/product name, and never spells out any of the three template
# substitution tokens. scripts/rename.sh does not carry this file in its
# frozen set, so any occurrence of those tokens here would otherwise be
# silently rewritten on rename — staying placeholder-free avoids that
# entirely and keeps this script byte-identical before and after rename.
#
# Usage:
#   EVALS=1 bash evals/run-live.sh                      # tier 2 smoke only
#   EVALS=1 TIER=3 bash evals/run-live.sh                # tier 2 + tier 3 judge
#   EVALS=1 SKILL_SLUG=ownership-review bash evals/run-live.sh
#
# Every guard below exits 0 on skip, so an accidental default-path call
# (EVALS unset, no claude CLI, no API key, no skill in the tree) is always a
# no-op — never a failure.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if [[ "${EVALS:-0}" != "1" ]]; then
  echo "run-live: set EVALS=1 (needs claude CLI + API budget); skipping."
  exit 0
fi

if ! command -v claude >/dev/null 2>&1; then
  echo "run-live: claude CLI not found on PATH; skipping."
  exit 0
fi

if [[ -z "${ANTHROPIC_API_KEY:-}" ]]; then
  echo "run-live: ANTHROPIC_API_KEY not set; skipping."
  exit 0
fi

TIER="${TIER:-2}"
SKILL_SLUG="${SKILL_SLUG:-ownership-review}"
SKILL_FILE="$ROOT_DIR/.claude/skills/$SKILL_SLUG/SKILL.md"

if [[ ! -f "$SKILL_FILE" ]]; then
  echo "run-live: no skill at .claude/skills/$SKILL_SLUG/SKILL.md; skipping."
  exit 0
fi

SAMPLE_DIFF='--- a/src/features/notes/domain/note.cpp
+++ b/src/features/notes/domain/note.cpp
@@ -1,3 +1,8 @@
 namespace notes {
+  auto load(const std::string& raw) {
+    return Note{raw, false};
+  }
 }'

echo "run-live: tier 2 — smoke-testing the '$SKILL_SLUG' skill end to end"
tier2_out="$(claude -p "Invoke the skill from .claude/skills/$SKILL_SLUG/SKILL.md against this diff and report findings per its Output Format section:
${SAMPLE_DIFF}" 2>&1)" || {
  echo "run-live: tier 2 FAILED — claude -p returned nonzero" >&2
  exit 1
}

if [[ -z "$tier2_out" ]]; then
  echo "run-live: tier 2 FAILED — empty response from claude -p" >&2
  exit 1
fi

if ! grep -qiE 'BLOCK|WARN|APPROVE|Verdict' <<< "$tier2_out"; then
  echo "run-live: tier 2 FAILED — response missing an expected review marker (BLOCK/WARN/APPROVE/Verdict)" >&2
  echo "$tier2_out" >&2
  exit 1
fi

echo "run-live: tier 2 PASS"

if [[ "$TIER" != "3" ]]; then
  echo "run-live: TIER=$TIER — skipping tier 3 judge pass."
  exit 0
fi

echo "run-live: tier 3 — LLM-judge pass over a canned skill output"
CANNED_OUTPUT='### demo review — pre-landing checklist

Gate: make ci (clang-format + build + ctest)

Findings:
- [BLOCK] layer violation @ src/features/notes/domain/note.cpp:2 — the domain
  path parses raw input without an error path | fix: return a typed error
  instead of a defaulted value

Not reviewed (not read):
- (none)

Verdict: BLOCK'

RUBRIC='Score the review output 1-5 on: (1) correctly flags the defaulted
error path as a BLOCK, (2) cites the real file:line from the diff, (3)
supplies an actionable fix, (4) ends with an explicit Verdict line, (5)
contains no fabricated finding not present in the diff. Reply with exactly
one line: "PASS score=<n>" if every criterion is met, otherwise "FAIL
score=<n> reason=<short reason>".'

tier3_out="$(claude -p "You are a strict grader. ${RUBRIC}

Output to grade:
${CANNED_OUTPUT}" 2>&1)" || {
  echo "run-live: tier 3 FAILED — claude -p judge call returned nonzero" >&2
  exit 1
}

if ! grep -qE '^(PASS|FAIL) score=[0-9]' <<< "$tier3_out"; then
  echo "run-live: tier 3 FAILED — judge response did not match 'PASS|FAIL score=<n>'" >&2
  echo "$tier3_out" >&2
  exit 1
fi

echo "run-live: tier 3 judge verdict: $tier3_out"

if grep -qE '^FAIL' <<< "$tier3_out"; then
  echo "run-live: tier 3 FAILED — judge returned FAIL" >&2
  exit 1
fi

echo "run-live: tier 3 PASS"
