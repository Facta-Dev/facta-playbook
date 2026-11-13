#!/usr/bin/env bash
# check-architecture.sh — fail when the Gradle module graph is wrong.
#
# The compiler already catches the impossible mistakes (an android.* import
# in a pure JVM module). This script catches the possible ones: a ui -> data
# dependency, a second application module, a kernel that depends on a slice.
# It is a gate, not a lint: it exits non-zero.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
fail() { echo "check-architecture: $*" >&2; status=1; }

# 1. Pure JVM modules never apply an Android plugin.
for module in core/kernel feature/notes/contracts feature/notes/domain evals; do
  if grep -qE 'com\.android\.(application|library)' "$module/build.gradle.kts"; then
    fail "$module applies an Android plugin"
  fi
done

# 2. The kernel depends on nothing internal.
if grep -q 'project(' core/kernel/build.gradle.kts; then
  fail "core/kernel depends on a project"
fi

# 3. The pure JVM modules import no Android API. The AndroidX artifacts are
# legal only where they publish a JVM variant: :feature:notes:contracts uses
# the multiplatform Navigation3 runtime.
for module in core/kernel feature/notes/contracts feature/notes/domain evals; do
  if grep -rqE '^import android\.' "$module/src" 2>/dev/null; then
    fail "$module imports the Android SDK"
  fi
done
for module in core/kernel feature/notes/domain evals; do
  if grep -rqE '^import androidx\.' "$module/src" 2>/dev/null; then
    fail "$module imports an AndroidX artifact"
  fi
done

# 4. ui never reaches data; data never reaches ui.
if grep -q ':feature:notes:data' feature/notes/ui/build.gradle.kts; then
  fail "feature/notes/ui depends on feature/notes/data"
fi
if grep -rq 'feature\.notes\.data' feature/notes/ui/src 2>/dev/null; then
  fail "feature/notes/ui imports feature/notes/data"
fi
if grep -q ':feature:notes:ui' feature/notes/data/build.gradle.kts; then
  fail "feature/notes/data depends on feature/notes/ui"
fi

# 5. Only :app is an application.
for module in core/kernel core/ui-core feature/notes/contracts feature/notes/domain feature/notes/data feature/notes/ui evals; do
  if grep -q 'libs.plugins.android.application' "$module/build.gradle.kts"; then
    fail "$module applies android.application"
  fi
done

# 6. The evals stay pure: no dependency on the app modules.
if grep -q 'project(' evals/build.gradle.kts; then
  fail "evals depends on an app module"
fi

if [[ "$status" -eq 0 ]]; then
  echo "check-architecture: PASSED"
fi
exit "$status"
