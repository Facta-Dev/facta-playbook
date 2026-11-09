#!/usr/bin/env bash
# run-live.sh -- live-tier evals for the .claude skills layer.
#
# Distinct from the offline pytest contract suite (`uv run pytest evals`,
# fixture-only, no network, hermetic). This script drives the Claude Code CLI
# against a live skill invocation and, optionally, an LLM-judge pass. Kept
# OUTSIDE verify-template.sh and CI (INV-T16): costs API budget + needs live
# claude.
#
# Placeholder-free: references the skill slug ("review"), never an app name,
# never spells a template substitution token; byte-identical across rename.

# Usage:
#   EVALS=1 bash evals/run-live.sh            # tier 2 smoke only
#   EVALS=1 TIER=3 bash evals/run-live.sh     # tier 2 + tier 3 judge
#
# Every guard exits 0 on skip: an accidental default-path call is a no-op.

set -euo pipefail

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

# src/app/features/widgets/domain.py diff: a ValueError swallowed into a
# silent "0" fallback — the review skill should flag it as a swallowed error
# in a library path. (inert display names only; no placeholders)
SAMPLE_DIFF='--- a/src/app/features/widgets/domain.py
+++ b/src/app/features/widgets/domain.py
@@ -19,3 +19,10 @@ class Widget:
     kind: WidgetKind
     owner_user_id: str
     created_at: datetime
+
+
+def parse_widget_id(raw: str) -> str:
+    """Parse a widget id, falling back to zero when it is not a number."""
+    try:
+        return str(int(raw))
+    except ValueError:
+        return "0"'

echo "run-live: tier 2 -- smoke-testing the review skill end to end"
tier2_out="$(claude -p "Invoke the review skill from .claude/skills/review/SKILL.md against this diff and report findings per its Output Format section:
${SAMPLE_DIFF}" 2>&1)" || {
  echo "run-live: tier 2 FAILED -- claude -p returned nonzero" >&2
  exit 1
}

if [[ -z "$tier2_out" ]]; then
  echo "run-live: tier 2 FAILED -- empty response" >&2
  exit 1
fi

if ! grep -qiE 'BLOCK|WARN|APPROVE|Verdict' <<< "$tier2_out"; then
  echo "run-live: tier 2 FAILED -- response missing a review marker" >&2
  echo "$tier2_out" >&2
  exit 1
fi

echo "run-live: tier 2 PASS"

if [[ "$TIER" != "3" ]]; then
  echo "run-live: TIER=$TIER -- skipping tier 3 judge pass."
  exit 0
fi

echo "run-live: tier 3 -- LLM-judge pass over a canned skill output"

CANNED_OUTPUT='### demo review — pre-landing checklist

Gate: make ci = ruff + mypy + import-linter + pytest

Findings:
- [BLOCK] error-discipline @ src/app/features/widgets/domain.py:28 — the
  ValueError is swallowed and the function returns a silent "0" id, so a
  malformed id reaches the caller as a valid-looking value | fix: let the
  ValueError propagate as an AppError with a stable Code instead of
  returning a fallback

Not reviewed (not read):
- (none)

Verdict: BLOCK'

RUBRIC='Score the review output 1-5 on: (1) correctly flags the swallowed ValueError fallback as a BLOCK, (2) cites the real file:line from the diff, (3) supplies an actionable fix, (4) ends with an explicit Verdict line, (5) contains no fabricated finding not present in the diff. Reply with exactly one line: "PASS score=<n>" if every criterion is met, otherwise "FAIL score=<n> reason=<short reason>".'

tier3_out="$(claude -p "You are a strict grader. ${RUBRIC}

Output to grade:
${CANNED_OUTPUT}" 2>&1)" || {
  echo "run-live: tier 3 FAILED -- judge call returned nonzero" >&2
  exit 1
}

if ! grep -qE '^(PASS|FAIL) score=[0-9]' <<< "$tier3_out"; then
  echo "run-live: tier 3 FAILED -- judge response did not match 'PASS|FAIL score=<n>'" >&2
  echo "$tier3_out" >&2
  exit 1
fi

echo "run-live: tier 3 judge verdict: $tier3_out"


if grep -qE '^FAIL' <<< "$tier3_out"; then
  echo "run-live: tier 3 FAILED -- judge returned FAIL" >&2
  exit 1
fi

echo "run-live: tier 3 PASS"
