# Device Support Matrix

The per-release table. It is a product artifact: when a device class leaves
the supported set, the change is made here, not in a chat.

The tier definitions, the reasoning and the budgets are in
[02 — Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md).

## 1 · The three tiers

| Tier     | Profile                                                                                                                  | The floor of the tier                                                                      |
| -------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| **low**  | ≤ 3 GB RAM, little-core CPU (Cortex-A53/A55 class), entry GPU, eMMC storage, small screen, aggressive OEM battery killer | A 5–6-year-old Android phone, an entry "Go" device, the oldest supported iPhone generation |
| **mid**  | 4–6 GB RAM, mid-tier SoC (Snapdragon 6/7 series, Dimensity 700/800, A-series from ~3–4 years ago), UFS storage           | The bulk of the global market                                                              |
| **high** | ≥ 8 GB, flagship SoC                                                                                                     | Optional extras only                                                                       |

## 2 · Minimum supported versions

| Platform    | Minimum                                              | Recommended            | Architectures                                                    | Notes                                                     |
| ----------- | ---------------------------------------------------- | ---------------------- | ---------------------------------------------------------------- | --------------------------------------------------------- |
| **Android** | The minimum API level of the pinned Expo SDK         | The current target API | `arm64-v8a` primary; `armeabi-v7a` only if the fleet requires it | 16 KB page-size aligned native libraries; R8 shrinking on |
| **iOS**     | The minimum deployment target of the pinned Expo SDK | The current release    | `arm64`                                                          | Privacy manifest; phased release                          |

> **Verify before publishing.** The minimum versions follow the pinned Expo
> SDK and React Native version, and they move with every SDK upgrade. The
> ABI split follows the fleet data, not the marketing target.

## 3 · What each tier may enable

| Capability                      | low               | mid                       | high                             |
| ------------------------------- | ----------------- | ------------------------- | -------------------------------- |
| Frame budget                    | 33 ms (30 fps)    | 24 ms                     | 16 ms (60 fps)                   |
| Render scale                    | 0.7 (floor 0.55)  | 0.85 (floor 0.7)          | 1.0 (floor 0.85)                 |
| Shadows / post-processing       | off               | simple                    | on                               |
| Texture size                    | 512               | 1024                      | 2048                             |
| Particle budget                 | 100               | 400                       | 1000                             |
| Mesh LOD                        | aggressive        | balanced                  | generous                         |
| 3D scene                        | Off by default    | Light scenes              | Full scenes                      |
| AR                              | **No**            | If the device supports it | Yes                              |
| Background location             | **No**            | Foreground only           | Foreground + optional background |
| Voice capture                   | **No**            | Yes                       | Yes                              |
| Offline map tiles / asset packs | Deferred to Wi-Fi | On Wi-Fi                  | Anywhere                         |

The frame budget, the render scale, the shadows, the texture resolution and
the particle budget are the shipped `QualityProfile`
(`packages/core/src/quality-governor.ts`). The floors in brackets are the
degraded values the governor falls back to under thermal, memory or battery
pressure; the render scale never goes below `0.5`.

The rule: **a high tier adds fidelity, never different logic.** A feature
that only works on a high tier does not work.

## 4 · Reference devices

The reference set for the manual passes. The exact models are a product
decision; the **shape** of the set is not.

| Slot        | What it represents                                                   | Used for                                                         |
| ----------- | -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Low Android | ≤ 3 GB RAM, little-core CPU, eMMC, an aggressive OEM battery manager | The release gate; the memory budget; the push-delivery behaviour |
| Low iOS     | The oldest supported iPhone generation                               | The iOS floor; the memory budget                                 |
| Mid Android | 4–6 GB RAM, mid-tier SoC, UFS                                        | The daily-use reference                                          |
| Mid iOS     | A 3–4-year-old iPhone                                                | The daily-use reference                                          |
| High        | A current flagship                                                   | The upper bound; the optional extras                             |

The pool must contain **at least two Low, two Mid and one High per OS**, plus
a cloud device farm for breadth. An emulator is not a substitute: it does not
reproduce the thermal behaviour, the battery manager or the memory pressure
of a real device.

## 5 · The test pool

| Pool          | What runs on it                                                                    | How often                                       |
| ------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------- |
| The Low pool  | The release gate: cold start, memory, crash-free rate, the push-delivery behaviour | Every release candidate; nightly for the trends |
| The Mid pool  | The daily-use reference; the full E2E suite                                        | Nightly; every release candidate                |
| The High pool | The optional extras                                                                | Before a release that enables them              |
| The farm      | OS versions, screen sizes, locales, right-to-left                                  | Nightly; on an OS release                       |
| Emulators     | The E2E smoke suite                                                                | Every pull request                              |

## 6 · The budgets, restated

Measured on the **Low** pool, in a release build.

| Budget                               | Target      | Ceiling     |
| ------------------------------------ | ----------- | ----------- |
| Cold start to interactive            | ≤ 3.5 s     | 5.0 s       |
| Touch response                       | ≤ 100 ms    | 150 ms      |
| JS frame rate in lists and forms     | 60 fps      | ≥ 30 fps    |
| Shell memory resident                | ≤ 180 MB    | 220 MB      |
| 3D scene memory (whole process)      | ≤ 300 MB    | 350 MB      |
| Crash-free sessions                  | ≥ 99.5 %    | —           |
| Memory growth over 30 minutes of use | ≈ 0         | ≤ 10 MB     |
| Download size per ABI                | per variant | per variant |

## 7 · Known limitations per tier

The table a release fills in and a review reads.

| Limitation                                        | low | mid | high | Notes                                                               |
| ------------------------------------------------- | --- | --- | ---- | ------------------------------------------------------------------- |
| ARCore / ARKit availability                       |     |     |      | Many low-end Android phones lack ARCore                             |
| Background location                               |     |     |      | Requires a foreground service and a store justification             |
| Voice latency                                     |     |     |      | The microphone path is the expensive one                            |
| Push delivery under an aggressive battery manager |     |     |      | The OEM may delay or drop the message                               |
| Thermal throttling in direct sun                  |     |     |      | The governor steps down; the frame rate drops                       |
| Memory pressure under a split screen              |     |     |      | The process is killed and the state is restored from the projection |

## 8 · How this table changes

| Trigger                                   | Change                                                        |
| ----------------------------------------- | ------------------------------------------------------------- |
| An Expo SDK upgrade raises the minimum OS | Update §2 and the low-tier reference device                   |
| A tier boundary moves                     | Update §1 and re-measure the budgets in §6                    |
| A device model enters or leaves the fleet | Update the pool in §5 and re-run the Low gate                 |
| A new capability is enabled for a tier    | Update §3 and check the store disclosure for it               |
| A budget is missed                        | Fix the regression or move the budget; never quietly widen it |

## See also

- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
- [Testing & CI](12-testing-and-ci.md)
- [Playbooks](13-playbooks.md)
