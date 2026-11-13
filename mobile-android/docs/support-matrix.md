# Support Matrix

The per-release table. It is a product artifact: when a device class leaves
the supported set, the change is made here, not in a chat.

The tier definitions and the budgets are in
[02 — Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md).

## 1 · The supported range

| | |
| --- | --- |
| Minimum Android version | **API 26** (Android 8.0 Oreo) |
| `compileSdk` / `targetSdk` | 37 (Android 17) |
| Architectures | `arm64-v8a` only — the template ships no native code |
| Gradle / AGP / Kotlin | 9.6.0 / 9.5.1 / 2.4.21 |
| JVM target | 17 |

`minSdk` 26 is the template's floor. It gives the notification channels
(`NotificationChannel`, API 26+) and the modern `PendingIntent` flags
without a version check at every call site. Lowering it is possible; each
step down adds a branch, so it is a decision with a cost.

The template ships **no native code**, so it produces no `.so` and no
per-ABI split: one APK serves every 64-bit device. `armeabi-v7a` is not
enabled because 32-bit userland is still common on the low end, but it
doubles the artifact count and the native library size. Enable it when the
**fleet data** says so, not by default.

## 2 · The device tiers

| Tier | Roughly | What it means |
| --- | --- | --- |
| **Low** | ≤ 3 GB RAM, little-core CPUs, entry GPUs, eMMC, Go editions | The floor. The app must be usable, not fast. |
| **Mid** | 4–6 GB RAM, mainstream SoC | The reference device. |
| **High** | ≥ 8 GB RAM, capable GPU | The ceiling. |

### What each tier may enable

| Capability | Low | Mid | High |
| --- | --- | --- | --- |
| The full feature set | ✔ | ✔ | ✔ |
| High-quality images | Downloaded at a reduced size | Full size | Full size, prefetched |
| Animations | Reduced; `ANIMATOR_DURATION_SCALE` honoured | Full | Full |
| Rich push notifications | Text only; no images | `MessagingStyle` | Images and `MessagingStyle` |
| Background sync | On a network constraint, deferred | Same | Same |
| Heavy offline prefetch | Never | On Wi-Fi only | On Wi-Fi and metered |
| A 3D scene | The 2D fallback tier only | The mid scene tier | The high scene tier |
| The Live-mic indicator | Always, on every tier | — | — |

The rule behind the table: **a capability that a Low device cannot run is
gated by the tier, not by a build flag**. A build flag ships dead code to
every device; a tier gate ships it to nobody.

### What is never enabled, on any tier

| Capability | Why |
| --- | --- |
| A WebView-hosted 3D or game surface | An order-of-magnitude slowdown on the Low path |
| A required AR feature | It would exclude most of the market from installing |
| A background location flow that the user did not start | Play restricts it, and it is a battery and privacy cost |

## 3 · The reference devices

One device per tier, fixed. The performance budgets are measured on the Low
one, and a release that regresses them fails.

| Profile | Class | Reference | Used for |
| --- | --- | --- | --- |
| **Floor** | Low | An Android Go-class device with ≤ 3 GB RAM and eMMC storage | The release gate |
| **Low** | Low | A mainstream entry device, ≤ 3 GB RAM | The startup and memory budgets |
| **Reference** | Mid | A 4–6 GB device from the current generation | The daily development device |
| **Ceiling** | High | A current flagship | The upper bound of the experience |

The rules for the reference devices:

| Rule | Why |
| --- | --- |
| The Floor device is **never** upgraded mid-cycle | Otherwise the budget is measured against a different machine every release |
| The budget numbers come from the **release build with the baseline profile** | A debug build is slower and lies about the numbers |
| The device is measured **repeatedly** | One run is noise |
| A regression over the last release **fails the build** | A budget that only says "under 2.5 s" drifts upward |

## 4 · The device pool

A real-device pool, because an emulator is not a low-end device.

| Group | Size | What it covers |
| --- | --- | --- |
| **Low** | ≥ 2 devices | The startup, memory and jank budgets; the Go edition |
| **Mid** | ≥ 2 devices | The reference experience |
| **High** | ≥ 1 device | The ceiling |
| **OEM** | ≥ 1 device from an aggressive battery manager | The push delivery and the background work |
| **Cloud farm** | Breadth across manufacturers | The compatibility matrix, on demand |

| Rule | Why |
| --- | --- |
| The pool is **real hardware** | The emulator's CPU, storage and GPU are not the Low tier |
| At least one **Android Go-edition** device | The memory ceiling is the tightest |
| At least one **aggressive OEM** device | AOSP behaviour says nothing about a vendor's battery manager ([10](10-push-notifications.md)) |
| The pool is **versioned** | "We tested on 4.4.2" is not a result |
| The pool runs the **E2E suite** on every release | The journeys must work on the floor device |

## 5 · The budgets

Measured on the **Floor** profile, release build, baseline profile applied.

| Budget | Target | Ceiling |
| --- | --- | --- |
| Cold start to first display (TTID) | ≤ 2.0 s | ≤ 2.5 s |
| Time to fully drawn (TTFD) | ≤ 2.8 s | ≤ 3.5 s |
| Touch response | ≤ 80 ms | ≤ 100 ms |
| Slow frames in the main flows | < 3 % | < 5 % |
| Frozen frames in a scroll benchmark | 0 | 0 |
| Shell memory (PSS) | ≤ 90 MB | ≤ 120 MB |
| Memory growth over 30 minutes | ≈ 0 | ≈ 0 |
| Network | Works on slow 3G | — |
| Per-device download (AAB) | ≤ 25 MB | ≤ 35 MB |

The full table, including the 3D ceilings, is in
[02 — Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md).

## 6 · The support tiers

| Tier | Meaning | Commitment |
| --- | --- | --- |
| **Supported** | In §1, within the vendor's support window, and in the pool | Security fixes, bug fixes, dependency bumps |
| **Best effort** | Older but not end-of-life, or a device not in the pool | Fixes only when trivial |
| **Unsupported** | Outside §1, or a device no longer in the pool | Not tested, not diagnosed |

## 7 · How this table changes

| Trigger | Change |
| --- | --- |
| Play raises the target-API floor | Update `compileSdk`/`targetSdk` in §1 and every module, in the same change |
| The platform deprecates a minimum API | Raise `minSdk` in §1 and remove the version branches |
| The Floor device leaves the fleet | Re-measure the budgets on the new floor and update §3 and §5 |
| A new architecture is needed | Add it to §1 and to the ABI configuration |
| A dependency raises its own `minSdk` | Raise `minSdk` in the same change, or pin the dependency to the previous version |

## See also

- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
  — the tier contract and the governor.
- [Security & Keystore](09-security-and-keystore.md) — the Play policy
  checks.
- [Testing & CI](12-testing-and-ci.md) — where the pool runs the suite.
