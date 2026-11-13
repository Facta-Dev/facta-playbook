#!/usr/bin/env bash
#
# bootstrap.sh — pick the install path for this machine and run it.
#
# Decision tree:
#
#   PRECONDITIONS: git, python3 >= 3.9, claude >= 2.1.x, gh authenticated
#        |
#   OS?
#        |- Linux / macOS / WSL2  -> continue
#        |- native Windows        -> stop: install WSL2 and re-run inside it
#        |
#   PROVIDER?
#        |- Ollama endpoint answers at 127.0.0.1:11434 -> OLLAMA path
#        |    settings env: tier aliases (opus/sonnet/haiku -> Ollama cloud
#        |    models), endpoint + the fixed literal token `ollama`, cheap
#        |    subagent routing. Verify with the probes in section 9 of
#        |    CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md.
#        |- otherwise (Anthropic login or gateway)     -> ANTHROPIC path
#        |    settings env: none (no endpoint pinning); the `model: sonnet`
#        |    default still applies; native `--advisor` available (runbook §7)
#        |
#   SCOPE?
#        |- user kit        install-user-kit.sh [--provider=...]
#        |- kit check       kit_check.py (0 errors required)
#        |- working set     clone-repos.sh --group=<group>
#        |- skill packs     install-skill-packs.sh
#        '- afterwards      per-repo configure, cost tracker, probes
#
# The provider branch only changes which settings env is merged; everything
# downstream is identical shell work (zero model tokens).
#
# Usage:
#   bash scripts/claude-kit/bootstrap.sh [--provider=ollama|anthropic|auto]
#        [--group=selected|core|products|tools|clients|reference|all]
#        [--dir=DIR] [--dry-run]
#     --provider=   force the provider branch (default: auto-detect Ollama)
#     --group=      clone-repos group (default: selected)
#     --dir=        working-set root (default: ~/work)
#     --dry-run     print the plan, run nothing
#
# Exit 0 = all steps ran (or printed) clean; exit 1 = precondition missing or
# a step failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT_DIR="$(cd "$SCRIPT_DIR/../../kit" && pwd)"

PROVIDER="auto"
GROUP="selected"
DIR="$HOME/work"
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider=*) PROVIDER="${1#--provider=}"; shift;;
    --provider)   PROVIDER="${2:-}"; shift 2;;
    --group=*)    GROUP="${1#--group=}"; shift;;
    --group)      GROUP="${2:-}"; shift 2;;
    --dir=*)      DIR="${1#--dir=}"; shift;;
    --dir)        DIR="${2:-}"; shift 2;;
    --dry-run)    DRY_RUN=1; shift;;
    -h|--help)    sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "bootstrap: unknown arg '$1'" >&2; exit 1;;
  esac
done

say() { echo "bootstrap: $*"; }

run() {
  # run <cmd...> — execute one install step, or print it on --dry-run.
  local cmd="$1"; shift
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "+ $cmd $*"
  else
    say "step: $cmd $*"
    bash "$cmd" "$@"   # kit scripts are bash entry points; keeps quoting simple
  fi
}

# --- 1. OS ---------------------------------------------------------------------
OS_NAME="$(uname -s)"
case "$OS_NAME" in
  Linux)
    OS="Linux"
    if grep -qi microsoft /proc/version 2>/dev/null; then
      OS="Linux (WSL2)"
    fi
    ;;
  Darwin)
    OS="macOS"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "bootstrap: native Windows detected — the kit scripts and hooks are first-class on POSIX shells."
    echo "bootstrap: install WSL2, then re-run inside it:  wsl -e bash $SCRIPT_DIR/bootstrap.sh"
    exit 1
    ;;
  *)
    echo "bootstrap: unsupported OS '$OS_NAME' — Linux, macOS or WSL2 required." >&2
    exit 1
    ;;
esac
say "os: $OS"

# --- 2. prereq checks ------------------------------------------------------------
for tool in git python3 claude gh; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "bootstrap: missing prerequisite: $tool" >&2
    case "$tool" in
      claude) echo "bootstrap:   install per https://code.claude.com/docs (npm install -g @anthropic-ai/claude-code)" >&2;;
      gh)     echo "bootstrap:   install per https://cli.github.com, then: gh auth login" >&2;;
      *)      echo "bootstrap:   install the standard package for your OS" >&2;;
    esac
    exit 1
  fi
done
if ! gh auth status -h github.com >/dev/null 2>&1; then
  echo "bootstrap: gh is not authenticated for github.com — run: gh auth login" >&2
  exit 1
fi
say "prereqs: git $(git --version | awk '{print $3}'), python3 $(python3 --version | awk '{print $2}'), claude $(claude --version 2>/dev/null | awk '{print $1}'), gh ok"

# --- 3. provider branch -----------------------------------------------------------
detect_ollama() {
  command -v curl >/dev/null 2>&1 || return 1
  curl -fsS --max-time 2 http://127.0.0.1:11434/api/version >/dev/null 2>&1
}

case "$PROVIDER" in
  auto)
    if detect_ollama; then PROVIDER_FINAL="ollama"; else PROVIDER_FINAL="anthropic"; fi;;
  ollama|anthropic) PROVIDER_FINAL="$PROVIDER";;
  *) echo "bootstrap: unknown provider '$PROVIDER' (ollama|anthropic|auto)" >&2; exit 1;;
esac
case "$PROVIDER_FINAL" in
  ollama)   say "provider: ollama (endpoint answers at 127.0.0.1:11434 — tier env will be merged)";;
  anthropic) say "provider: anthropic (no local Ollama endpoint — base settings only)";;
esac

# --- 4. the install chain ----------------------------------------------------------
KIT_CHECK="$SCRIPT_DIR/kit_check.py"
DRYFLAG=""
if [[ "$DRY_RUN" -eq 1 ]]; then
  DRYFLAG="--dry-run"
fi

run "$SCRIPT_DIR/install-user-kit.sh" --provider="$PROVIDER_FINAL" $DRYFLAG
if [[ "$DRY_RUN" -eq 0 ]]; then
  say "kit check:"
  set +e
  python3 "$KIT_CHECK"
  KC=$?
  set -e
  if [[ "$KC" -ge 2 ]]; then
    echo "bootstrap: kit_check.py exited $KC (configuration errors) — fix them and re-run." >&2
    exit 1
  fi
fi
run "$SCRIPT_DIR/clone-repos.sh" --group="$GROUP" --dir="$DIR" $DRYFLAG
run "$SCRIPT_DIR/install-skill-packs.sh" --dir="$DIR" $DRYFLAG

# --- 5. next steps (not automated: per-repo judgment) ------------------------------
TRACKER="$DIR/claude-code-cli-cost-tracker"
say ""
say "done — path taken: $OS -> $PROVIDER_FINAL -> user kit + kit_check + clone($GROUP) + skill packs"
say "next steps (manual or per-repo):"
if [[ -d "$TRACKER" ]]; then
  echo "  1. cost tracker:  bash $TRACKER/install.sh"
fi
echo "  2. wire repos:    bash $SCRIPT_DIR/configure-repo.sh ~/work/<repo> --kit=<golang|nodejs|python|rust|java|extension|desktop|frontend-mobile|mobile-android|mobile-ios>"
echo "  3. verify:        python3 $SCRIPT_DIR/kit_check.py; claude -p probes per the routing runbook §9"
echo "  4. in-session:    /context and /doctor"
