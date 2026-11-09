#!/usr/bin/env bash
#
# leak-guard.sh — fail if any literal template placeholder (<app>/<APP>/<scope>)
# or obvious secret token survives in the tree. Template-local; no external
# dependency. Two checks:
#   1. PLACEHOLDER LEAK: any residual <app>/<APP>/<scope> after rename.
#   2. SECRET LEAK: high-signal secret patterns (private keys, long bearer
#      tokens, password assignments) committed in tracked text files.
#
# Usage: bash scripts/leak-guard.sh [--placeholders-only]
#   --placeholders-only  skip the secret-pattern scan (used post-rename in
#                        verify-template.sh where the staged copy has no secrets)
# Exit 0 = clean, exit 1 = leak found.

set -euo pipefail
cd "$(dirname "$0")/.."

mode="all"
[[ "${1:-}" == "--placeholders-only" ]] && mode="placeholders"

leaks=0

# --- 1. placeholder leak ------------------------------------------------------
# Scan tracked text files; skip binary assets, build artifacts, lockfiles, the
# frozen eval fixtures (the invalid leak fixture deliberately keeps <app> to
# exercise the leak detector), and the pure-bash linter/leak-guard/installer
# scripts whose <app>/<APP>/<scope> literals are PATTERN TOKENS, not template
# placeholders to flag.
placeholder_hits="$(grep -rIl -e '<app>' -e '<APP>' -e '<scope>' \
  --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
  --exclude-dir=fixtures --exclude-dir=evals \
  --exclude='*.lock' --exclude='*.png' --exclude='*.jpg' --exclude='*.ico' \
  --exclude='*.woff' --exclude='*.woff2' --exclude='*.ttf' \
  --exclude='lint-skills.sh' --exclude='lint-commands.sh' \
  --exclude='leak-guard.sh' --exclude='install-claude-runtime.sh' \
  . 2>/dev/null || true)"
if [[ -n "$placeholder_hits" ]]; then
  echo "leak-guard: PLACEHOLDER LEAK — residual <app>/<APP>/<scope> found:" >&2
  printf '  %s\n' "$placeholder_hits" >&2
  leaks=$((leaks + 1))
fi

# --- 2. secret leak (skipped in --placeholders-only mode) --------------------
if [[ "$mode" == "all" ]]; then
  # High-signal patterns: private key headers, long hex/base64 bearer tokens,
  # password = "..." assignments. False-positive-prone patterns are deliberately
  # omitted; this is a guard rail, not a full SAST. Matched lines are then
  # filtered to drop obvious test/example fixtures (values containing
  # 'secret', 'example', 'test', 'placeholder', 'redacted', 'change-me',
  # 'your-', 'sample', 'here-at-least', 'default') — real credentials rarely
  # contain those words, but test fixtures ("sk-super-secret-key-here-...")
  # routinely do.
  raw_secret_hits="$(grep -rInE \
    -e '-----BEGIN (RSA |EC |OPENSSH |PRIVATE )?PRIVATE KEY-----' \
    -e '(password|passwd|secret|api_key|apikey|access_token|bearer)[[:space:]]*[:=][[:space:]]*["'"'"'][A-Za-z0-9+/=_-]{16,}' \
    --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
    --exclude='*.lock' --exclude='*.sum' \
    --exclude='.env.example' \
    . 2>/dev/null || true)"
  secret_hits="$(printf '%s\n' "$raw_secret_hits" | grep -viE \
    -e 'secret-key-here' -e 'super-secret' -e 'example' -e 'placeholder' \
    -e 'redacted' -e 'change-me' -e 'your-' -e 'sample' -e 'here-at-least' \
    -e 'test-secret' -e 'default-secret' -e 'cfg\.[a-z_]+_secret = ' \
    2>/dev/null || true)"
  if [[ -n "$secret_hits" ]]; then
    echo "leak-guard: SECRET LEAK — high-signal secret pattern found:" >&2
    printf '  %s\n' "$secret_hits" >&2
    leaks=$((leaks + 1))
  fi
fi

if [[ $leaks -gt 0 ]]; then
  echo "leak-guard: FAILED — ${leaks} leak class(es) found." >&2
  exit 1
fi
echo "leak-guard: 0 leaks"
exit 0
