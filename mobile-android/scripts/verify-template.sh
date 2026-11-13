#!/usr/bin/env bash
# verify-template.sh — smoke-test the Android template end to end.
#
# The template ships with literal <app>/<APP>/<scope> placeholders, which are
# not valid Kotlin package segments and cannot be built as shipped. This
# script stages a throwaway copy in a temp dir, renames it via
# scripts/rename.sh (example values), then runs the template's own gates on
# the instantiated copy: the three .claude linters, the instruction budget,
# the leak guard, a shell syntax check, the architecture gate and the Gradle
# build/test set. The real working tree is never mutated. Exits nonzero on
# any failure and removes the staged copy on the way out.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="demo"
EXAMPLE_SCOPE="demoorg"

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

# Gradle runs on JAVA_HOME (or on `java` from PATH when it is unset); AGP also
# needs javac from that same installation. scripts/jdk.sh prints a JDK that has
# both binaries, so a JRE-only default on PATH cannot fail the build with
# "does not provide the required capabilities: [JAVA_COMPILER]".
if ! JAVA_HOME="$(bash "$ROOT_DIR/scripts/jdk.sh")"; then
  echo "verify-template: a full JDK (17+) is required; set JAVA_HOME to one with bin/javac." >&2
  exit 1
fi
export JAVA_HOME
export PATH="$JAVA_HOME/bin:$PATH"

# The Android SDK is resolved from ANDROID_HOME / ANDROID_SDK_ROOT, with the
# Linux default as a last resort.
if [[ -z "${ANDROID_HOME:-}" && -n "${ANDROID_SDK_ROOT:-}" ]]; then
  export ANDROID_HOME="$ANDROID_SDK_ROOT"
fi
if [[ -z "${ANDROID_HOME:-}" && -d "$HOME/Android/Sdk" ]]; then
  export ANDROID_HOME="$HOME/Android/Sdk"
fi
if [[ -z "${ANDROID_HOME:-}" ]]; then
  echo "verify-template: set ANDROID_HOME to an Android SDK with platform 37." >&2
  exit 1
fi

TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

STAGED="$TMP/kit"

echo "verify-template: staging $STAGED"
cp -a "$ROOT_DIR" "$STAGED"

# The rename must reject a slug that cannot be a Java package segment BEFORE
# it touches the tree: a hyphenated slug yields an unbuildable namespace and
# a slash aborts the substitution mid-tree, leaving a half-renamed copy.
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

# run-live.sh must be a no-op without EVALS=1 — it costs API budget and needs
# a live claude CLI, so it is never part of the default gate.
stage "run-live no-op guard" bash "$STAGED/evals/run-live.sh"

# --- architecture and build ---------------------------------------------------
stage "check-architecture" bash "$STAGED/scripts/check-architecture.sh"

echo "verify-template: gradle test (pure JVM + Android unit tests)"
stage "gradle-test" "$STAGED/gradlew" -p "$STAGED" test testDebugUnitTest

echo "verify-template: gradle evals"
stage "gradle-evals" "$STAGED/gradlew" -p "$STAGED" :evals:test

echo "verify-template: gradle lint"
stage "gradle-lint" "$STAGED/gradlew" -p "$STAGED" :app:lintDebug

echo "verify-template: gradle assembleDebug"
stage "gradle-assemble" "$STAGED/gradlew" -p "$STAGED" :app:assembleDebug

if [[ $failures -gt 0 ]]; then
  echo "verify-template: FAILED — ${failures} stage(s) failed." >&2
  exit 1
fi

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
