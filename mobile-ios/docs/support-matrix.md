# Support Matrix

The per-release table. It is a product artifact: when a device class leaves
the supported set, the change is made here, not in a chat.

The tier definitions and the budgets are in
[02 — Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md).

**Read this first:** the template ships **no tier implementation**. There is
no `DeviceTier` type, no capability detection, no quality governor and no
budget enforcement in the tree. Everything in §2 and §3 below is a
**contract to implement**, not a description of shipped code. The governor
itself is a known gap
([00 — Overview & Principles](00-overview-and-principles.md)).

## 1 · The supported range

| | |
| --- | --- |
| Minimum iOS version | **iOS 18.0** |
| Package platform | iOS 18 / macOS 15 (`platforms: [.iOS(.v18), .macOS(.v15)]`) |
| App deployment target | iOS 18.0 (`project.yml`) |
| Swift language mode | 6.0, `swiftLanguageModes: [.v6]` in every manifest |
| Strict concurrency | `complete` |
| Toolchain | Xcode 16+, XcodeGen |
| Host for the native tier | macOS |
| Architectures | The Apple platform's own — the template ships no native code and no binary dependency |

iOS 18 is the template's floor. It is the platform the package manifests and
`project.yml` declare, and it is the floor the Swift 6 mode and the
Observation-based UDF assume. Lowering it is a manifest change plus a
branch at every call site, so it is a decision with a cost, not a default.

The **macOS 15** platform in the package manifests is not a shipping target:
it is what lets `swift test` run the pure packages on a Mac host without a
simulator.

The template ships **no native code**, so there is no per-architecture
binary to split and no `XCFramework` to align. A dependency that brings one
changes this row and the App Store review posture
([09](09-security-and-keychain.md)).

## 2 · The device tiers

The classes are defined by **silicon generation and RAM**, not by a model
list, so a new device joins a tier without a table edit.

| Tier | Roughly | What it means |
| --- | --- | --- |
| **Low** | iPhone XS/XR (A12), iPhone 11 / SE 2 / SE 3 (A13/A15), 3–4 GB RAM | The floor. The app must be usable, not fast. Older GPU family, thermal throttling sooner, small RAM headroom. |
| **Mid** | iPhone 12/13/14 (A14–A16), 4–6 GB RAM | The bulk of the active base. The reference class. |
| **High** | A17 Pro and newer, 8 GB RAM | The ceiling. Optional extras. |

### What each tier would enable

Nothing in this table is implemented. It is the decision each tier would
carry.

| Capability | Low | Mid | High |
| --- | --- | --- | --- |
| The full feature set | ✔ | ✔ | ✔ |
| Render scale | Reduced (≤ 0.75) | Full | Full |
| Frame cap | 30 fps | 30–60 fps | 60/120 fps |
| Texture quality | The compressed pack | Full | Full |
| Animations | Reduced; `accessibilityReduceMotion` honored | Full | Full |
| Heavy offline prefetch | Never | On Wi-Fi only | On Wi-Fi and cellular |
| A 3D scene | The 2D fallback only | The mid scene tier | The high scene tier |

The rule behind the table: **a capability a Low device cannot run is gated
by the tier, not by a build flag**. A build flag ships dead code to every
device; a tier gate ships it to nobody.

### What is never enabled, on any tier

| Capability | Why |
| --- | --- |
| A required AR feature | It would exclude most of the market from installing |
| A background location flow the user did not start | App Review restricts it, and it is a battery and privacy cost |
| A per-frame rendering loop above the frame cap | The thermal budget, not the hardware class, is the limit |

## 3 · The reference devices

One device per tier, fixed. The performance budgets are measured on the Low
one, and a release that regresses them fails.

| Profile | Class | Reference | Used for |
| --- | --- | --- | --- |
| **Floor** | Low | An A12 device on iOS 18 with 3–4 GB RAM | The release gate |
| **Low** | Low | An A13/A15 device (iPhone 11 or SE 2/3) | The startup and memory budgets |
| **Reference** | Mid | An A14–A16 device (iPhone 12/13/14) | The daily development device |
| **Ceiling** | High | An A17 Pro or newer | The upper bound of the experience |

The rules for the reference devices:

| Rule | Why |
| --- | --- |
| The Floor device is **never** upgraded mid-cycle | Otherwise the budget is measured against a different machine every release |
| The budget numbers come from the **release build** | A debug build is slower and lies about the numbers |
| The device is measured **repeatedly** | One run is noise |
| A regression over the last release **fails the build** | A budget that only says "under 2.5 s" drifts upward |

**This template ships no device pool.** The rows above are the pool to
build, not a pool that exists.

## 4 · The simulator and device pool

A real-device pool, because a simulator is not a low-end device.

| Group | Size | What it covers |
| --- | --- | --- |
| **Low** | ≥ 2 devices (one A12 on iOS 18, one A13) | The startup, memory and hang budgets |
| **Mid** | ≥ 2 devices | The reference experience |
| **High** | ≥ 1 device | The ceiling |
| **Cloud farm** | Breadth across models and OS versions | The compatibility matrix, on demand |

| Rule | Why |
| --- | --- |
| The pool is **real hardware** | The simulator's CPU, storage and GPU are not the Low tier |
| At least one **A12 on iOS 18** device | It is the floor of the supported range |
| The pool is **versioned** | "We tested on 18.2" is not a result |
| The pool runs the **E2E suite** on every release | The journeys must work on the floor device |

### The simulator requirement

| Requirement | Why |
| --- | --- |
| An **iOS 18 simulator runtime** must be installed | The first native build needs it; `xcodebuild` targets `generic/platform=iOS Simulator` |
| The first native build needs **network access** | SwiftPM resolves the packages |
| A simulator is **not** a budget measurement | Only real hardware produces the numbers in §5 |
| Simulator runs cover the **UI / integration** tier | The tier the host cannot run ([12](12-testing-and-ci.md) §10) |

## 5 · The budgets

Measured on the **Floor** profile, release build. These are the starting
points from the platform playbook; **the template enforces none of them** —
there is no instrumentation and no gate behind these numbers yet.

| Budget | Target |
| --- | --- |
| Cold launch to first frame (TTID) | ≤ 1.5 s (Low), ≤ 1.0 s (Mid) |
| Time to usable content (cached) | ≤ 2.5 s (Low) |
| Touch response | ≤ 100 ms |
| Main-thread hangs in the main flows | No hang over 250 ms |
| Scroll | Smooth 60 fps lists, no sustained dropped frames |
| Shell memory (no 3D) | ≤ ~150 MB footprint |
| Memory growth over 30 minutes | ≈ 0 |
| Crash-free sessions | ≥ 99.5 % (measured on the Low pool separately) |
| Download size | Market ≤ ~35 MB |

The full table, including the 3D ceilings, is in
[02 — Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md).

## 6 · The support tiers

| Tier | Meaning | Commitment |
| --- | --- | --- |
| **Supported** | In §1, within Apple's support window, and in the pool | Security fixes, bug fixes, dependency bumps |
| **Best effort** | Older but not end-of-life, or a device not in the pool | Fixes only when trivial |
| **Unsupported** | Outside §1, or a device no longer in the pool | Not tested, not diagnosed |

## 7 · How this table changes

| Trigger | Change |
| --- | --- |
| Apple raises the SDK floor | Update the deployment target in `project.yml` and `platforms:` in every `Package.swift`, in the same change |
| The platform deprecates a minimum iOS version | Raise the floor in §1 and in the manifests, and remove the version branches |
| The Floor device leaves the fleet | Re-measure the budgets on the new floor and update §3 and §5 |
| A dependency raises its own minimum | Raise the floor in the same change, or pin the dependency to the previous version |
| A new device generation appears | Assign it to a tier in §2 by silicon and RAM; do not add it to a model list |

## See also

- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
  — the tier contract and the governor.
- [Security & Keychain](09-security-and-keychain.md) — the App Store policy
  checks.
- [Testing & CI](12-testing-and-ci.md) — where the pool runs the suite.
- [Project Layout & Build Logic](05-project-layout-and-build-logic.md) — the
  deployment target and the toolchain.
