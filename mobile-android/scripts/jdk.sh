#!/usr/bin/env bash
# jdk.sh — print the JAVA_HOME of a JDK that provides javac.
#
# Gradle runs on JAVA_HOME (or on `java` from PATH when it is unset) and AGP
# compiles with javac from that same installation. A PATH where `java` points
# at a JRE-only default fails the build with "does not provide the required
# capabilities: [JAVA_COMPILER]", so the build pins a JDK that has both
# binaries instead of trusting the default.
#
# Usage: JAVA_HOME="$(bash scripts/jdk.sh)"
#   stdout — the chosen JDK home
#   exit 1 — no full JDK found; the reason goes to stderr
set -euo pipefail

jdk_has_compiler() { [[ -n "$1" && -x "$1/bin/java" && -x "$1/bin/javac" ]]; }

if jdk_has_compiler "${JAVA_HOME:-}"; then
  printf '%s\n' "$JAVA_HOME"
  exit 0
fi

candidates=(
  /usr/lib/jvm/java-21-openjdk-amd64
  /usr/lib/jvm/java-17-openjdk-amd64
  /usr/lib/jvm/*
)
if [[ -x /usr/libexec/java_home ]]; then
  candidates+=("$(/usr/libexec/java_home 2>/dev/null || true)")
fi
for candidate in "${candidates[@]}"; do
  if jdk_has_compiler "$candidate"; then
    printf '%s\n' "$candidate"
    exit 0
  fi
done

echo "jdk: a full JDK (17+) is required; set JAVA_HOME to one with bin/javac." >&2
exit 1
