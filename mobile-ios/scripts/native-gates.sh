#!/usr/bin/env bash
# native-gates.sh — the Apple-toolchain tier.
#
# The portable gates (lint, contract evals, architecture) run on any OS. This
# tier needs macOS + Xcode: XcodeGen turns project.yml into the .xcodeproj,
# `swift test` runs every package suite, and `xcodebuild` proves the app
# target compiles against the real SDK. Off macOS, or without the tools, it
# prints a visible SKIP and exits 0: the template is still a valid kit, its
# native proof is deferred to a Mac or CI.
#
# Usage: bash scripts/native-gates.sh [test|build|all]
set -euo pipefail
cd "$(dirname "$0")/.."

MODE="${1:-all}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "native-gates: SKIP — macOS + Xcode are required (xcodegen, swift, xcodebuild)."
  exit 0
fi

for tool in xcodegen swift xcodebuild; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "native-gates: SKIP — '$tool' is not on PATH."
    exit 0
  fi
done

PACKAGES=(
  Packages/Core/Kernel
  Packages/Core/UICore
  Packages/Feature/Notes/Contracts
  Packages/Feature/Notes/Domain
  Packages/Feature/Notes/Data
  Packages/Feature/Notes/UI
)

run_tests() {
  local package
  for package in "${PACKAGES[@]}"; do
    echo "native-gates: swift test — $package"
    (cd "$package" && swift test)
  done
}

run_build() {
  echo "native-gates: xcodegen generate"
  xcodegen generate
  shopt -s nullglob
  local projects=(*.xcodeproj)
  if [[ "${#projects[@]}" -ne 1 ]]; then
    echo "native-gates: expected exactly one generated .xcodeproj, found ${#projects[@]}" >&2
    exit 1
  fi
  local name="${projects[0]%.xcodeproj}"
  echo "native-gates: xcodebuild — $name"
  xcodebuild \
    -project "${projects[0]}" \
    -scheme "$name" \
    -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath DerivedData \
    CODE_SIGNING_ALLOWED=NO \
    build
}

case "$MODE" in
  test) run_tests ;;
  build) run_build ;;
  all) run_tests; run_build ;;
  *)
    echo "Usage: bash scripts/native-gates.sh [test|build|all]" >&2
    exit 1
    ;;
esac

echo "native-gates: PASS ($MODE)"
