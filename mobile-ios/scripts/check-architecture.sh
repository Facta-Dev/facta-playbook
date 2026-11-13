#!/usr/bin/env bash
# check-architecture.sh — fail when the Swift package graph is wrong.
#
# SwiftPM enforces what a manifest declares, but it does not notice a layer
# direction that should not exist, a UI framework imported into a pure
# package, or a generated project committed into the tree. This script does.
# It is a gate, not a lint: it exits non-zero.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
fail() { echo "check-architecture: $*" >&2; status=1; }

PURE_PACKAGES=(
  Packages/Core/Kernel
  Packages/Core/UICore
  Packages/Feature/Notes/Contracts
  Packages/Feature/Notes/Domain
)

# 1. The kernel depends on nothing internal.
if grep -q '\.package(path:' Packages/Core/Kernel/Package.swift; then
  fail "Core/Kernel depends on another package"
fi

# 2. The contracts package depends on nothing internal.
if grep -q '\.package(path:' Packages/Feature/Notes/Contracts/Package.swift; then
  fail "Notes/Contracts depends on another package"
fi

# 3. The pure packages import no UI or platform framework.
for module in "${PURE_PACKAGES[@]}"; do
  if grep -rqE '^import (SwiftUI|UIKit|AppKit|WatchKit)$' "$module/Sources" 2>/dev/null; then
    fail "$module imports a UI framework"
  fi
done

# 4. The UI never reaches data; data never reaches UI.
if grep -q 'NotesData' Packages/Feature/Notes/UI/Package.swift; then
  fail "Notes/UI depends on Notes/Data"
fi
if grep -rq 'import NotesData' Packages/Feature/Notes/UI/Sources 2>/dev/null; then
  fail "Notes/UI imports NotesData"
fi
if grep -q 'NotesUI' Packages/Feature/Notes/Data/Package.swift; then
  fail "Notes/Data depends on Notes/UI"
fi

# 5. The domain depends on no adapter and no presentation.
if grep -qE 'Notes(Data|UI)' Packages/Feature/Notes/Domain/Package.swift; then
  fail "Notes/Domain depends on an adapter or the UI"
fi

# 6. Only the app target is an application.
applications=$(grep -c 'type: application' project.yml || true)
if [[ "$applications" -ne 1 ]]; then
  fail "project.yml declares $applications application targets (expected 1)"
fi

# 7. No visual-builder artifacts: views are written as Swift code.
if find . -type f \
  \( -name '*.storyboard' -o -name '*.xib' -o -name '*.nib' \
     -o -name '*.xcdatamodeld' -o -name '*.rcproject' \) \
  | grep -q .; then
  fail "a visual-builder file (storyboard/xib/nib/xcdatamodeld/rcproject) is in the tree"
fi

# 8. The Xcode project is generated from project.yml, never committed.
if ! grep -qE '^\*\.xcodeproj/?$' .gitignore; then
  fail ".gitignore does not ignore *.xcodeproj"
fi
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if [[ -n "$(git ls-files -- '*.xcodeproj' '*.xcodeproj/*' 2>/dev/null)" ]]; then
    fail "a .xcodeproj is tracked in git — project.yml is the source"
  fi
fi

# 9. Every package compiles in Swift 6 language mode.
for manifest in Packages/Core/Kernel/Package.swift \
                Packages/Core/UICore/Package.swift \
                Packages/Feature/Notes/Contracts/Package.swift \
                Packages/Feature/Notes/Domain/Package.swift \
                Packages/Feature/Notes/Data/Package.swift \
                Packages/Feature/Notes/UI/Package.swift; do
  if ! grep -q 'swiftLanguageModes: \[.v6\]' "$manifest"; then
    fail "$manifest does not declare swiftLanguageModes: [.v6]"
  fi
done

# 10. The generated project pins Swift 6 and complete concurrency checking.
if ! grep -q 'SWIFT_VERSION: "6.0"' project.yml; then
  fail "project.yml does not pin SWIFT_VERSION 6.0"
fi
if ! grep -q 'SWIFT_STRICT_CONCURRENCY: complete' project.yml; then
  fail "project.yml does not enable complete strict concurrency"
fi

if [[ "$status" -eq 0 ]]; then
  echo "check-architecture: PASSED"
fi
exit "$status"
