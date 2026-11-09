#!/usr/bin/env bash
# run-live.sh — live-tier evals for the .claude skills layer.
#
# Distinct from the offline `cargo test --manifest-path evals/Cargo.toml`
# contract suite (fixture-only, no network, wired into `just ci` /
# scripts/verify-template.sh). This script instead drives the Claude Code
# CLI against a live skill invocation and, optionally, an LLM-judge pass. It
# is intentionally kept OUTSIDE verify-template.sh and the default `ci.yml`
# lanes (INV-T16) — it costs real API budget and needs a live `claude` CLI,
# neither of which belong in the default gate. Trigger it only via
# workflow_dispatch (see .github/workflows/evals-live.yml) or by hand.
#
# Deliberately placeholder-free: this file references the skill slug
# ("review"), never an app/product name, and never spells out any of the
# three template substitution tokens. rename.sh does not exclude
# evals/run-live.sh from its substitution pass the way it excludes
# evals/fixtures|src|tests, so any occurrence of those tokens here would
# otherwise be silently rewritten on rename — staying placeholder-free avoids
# that entirely and keeps this script byte-identical before and after rename.
#
# Usage:
#   EVALS=1 bash evals/run-live.sh            # tier 2 smoke only
#   EVALS=1 TIER=3 bash evals/run-live.sh      # tier 2 + tier 3 judge
#
# Every guard below exits 0 on skip, so an accidental default-path call
# (EVALS unset, no claude CLI, no API key) is always a no-op — never a
# failure.

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
SAMPLE_DIFF='--- a/crates/example-core/src/lib.rs
+++ b/crates/example-core/src/lib.rs
@@ -1,3 +1,6 @@
 pub struct Widget {
     pub id: WidgetId,
 }
+
+pub fn load(raw: &str) -> Widget {
+    Widget { id: WidgetId(raw.parse().unwrap()) }
+}'

echo "run-live: tier 2 — smoke-testing the review skill end to end"
tier2_out="$(claude -p "Invoke the review skill from .claude/skills/review/SKILL.md against this diff and report findings per its Output Format section:
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
CANNED_OUTPUT='### example review — pre-landing checklist

Gate: just ci (fmt-check + clippy -D warnings + check + nextest)

Findings:
- [BLOCK] unwrap-expect @ crates/example-core/src/lib.rs:5 — .unwrap() on a
  fallible parse in a library path with no inline justification | fix:
  propagate a WidgetError via ? instead of unwrap()

Not reviewed (not read):
- (none)

Verdict: BLOCK'

RUBRIC='Score the review output 1-5 on: (1) correctly flags the unjustified
unwrap() as a BLOCK, (2) cites the real file:line from the diff, (3) supplies
an actionable fix, (4) ends with an explicit Verdict line, (5) contains no
fabricated finding not present in the diff. Reply with exactly one line:
"PASS score=<n>" if every criterion is met, otherwise "FAIL score=<n>
reason=<short reason>".'

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
