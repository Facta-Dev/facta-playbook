# Support Matrix

The per-release table. It is a product artifact: when an operating system
leaves the supported set, the change is made here, not in a chat.

The floor profile, the budgets and the reasoning are in
[01 — Support Matrix & Reference Hardware](01-support-matrix-and-reference-hardware.md).

## 1 · Minimum supported versions

| Platform | Minimum | Recommended | Architectures | End of support |
| --- | --- | --- | --- | --- |
| **Windows** | 11 23H2 | 11 24H2+ | x86-64, arm64 | When Microsoft ends the release |
| **Windows 10** | 22H2 — **ESU only** | — | x86-64 | 14 October 2025 (extended with ESU) |
| **macOS** | 13 Ventura | Current | x86-64, arm64 (universal) | ~3 years after release |
| **Ubuntu** | 22.04 LTS (glibc 2.35) | 24.04 LTS | x86-64, arm64 | 2027 (standard maintenance) |
| **Debian** | 12 Bookworm (glibc 2.36) | 13 Trixie | x86-64, arm64 | ~5 years |
| **Fedora** | 41 | Current | x86-64, arm64 | ~13 months |
| **RHEL / Rocky** | 9 | Current | x86-64 | ~10 years |

**Qt baseline: 6.8** (the first LTS with the Wayland and high-DPI behaviour
the template relies on). The build resolves
`find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network Sql)` and
adds `Test` when the tests are on. Raising the baseline is a change to
`cmake/qt.cmake` and to this table, in the same pull request.

> **Verify before publishing.** The end-of-support dates come from the
> vendors' schedules and move. Check the two tables you do not remember
> before a release announcement.

## 2 · Display servers and rendering

| Environment | Supported | Rendering path |
| --- | --- | --- |
| Windows desktop | Yes | Native (D3D / raster) |
| macOS desktop | Yes | Native |
| X11 on Linux | Yes | Native (GLX / EGL) |
| Wayland on Linux | Yes | Native (EGL) |
| XWayland | Yes | Through the X11 path |
| RDP / VDI | Yes | **Software** (Compatibility Mode) |
| Virtual machine without a 3D device | Yes | **Software** (Compatibility Mode) |
| SSH / headless console | No | — |
| Windows Remote Assistance, legacy | No | — |

The last two are unsupported because there is no window to draw on, not
because the app cannot start.

## 3 · Explicitly unsupported

| Excluded | Reason |
| --- | --- |
| 32-bit Windows and Linux | Qt 6 ships no 32-bit binaries |
| Windows 7, 8, 8.1 | End of vendor security updates; Qt 6 requires Windows 10+ |
| macOS 10.15 Big Sur and older | Out of the Qt 6 support window |
| Alpine Linux and other musl distributions | Qt 6 is not shipped for musl; the SQLite plugin set differs |
| iOS, Android, embedded (QNX, VxWorks) | A different kernel, a different store and a different UI stack are required |
| WebAssembly | Widgets need a native event loop and a desktop window system |
| WebKit-based rendering of the UI | The UI is Widgets; the SDK is not a browser |
| A second process writing the same SQLite file | One writer per file, by design ([08](08-storage-and-migrations.md)) |
| Running the app from a network share | Latency and locking are undefined; install it locally |
| Installing without the bundled runtime | The installers carry the Qt runtime |

## 4 · Reference hardware

| Profile | CPU | RAM | Disk | Display | GPU | Used for |
| --- | --- | --- | --- | --- | --- | --- |
| **Floor** | 2 cores | 4 GB | HDD, 1 GB free | 1366 × 768 | None | The release gate |
| Typical | 4 cores | 8 GB | SSD | 1920 × 1080 | Integrated | Daily use |
| High-DPI | 4 cores | 16 GB | SSD | 2560 × 1440 @ 150 % | Integrated | Scaling checks |
| Remote desktop | 2 vCPU | 4 GB | — | 1280 × 720 | None | Compatibility Mode |

## 5 · Budgets

Measured on the **floor** profile.

| Budget | Target | Ceiling |
| --- | --- | --- |
| Cold start to first window | 1.5 s | 3.0 s |
| Warm start to first window | 0.6 s | 1.2 s |
| UI input latency | 60 ms | 150 ms |
| List scroll, 10 000 rows | 16 ms | 33 ms |
| Network list refresh | 400 ms | 1 200 ms |
| Local list read, 10 000 rows | 30 ms | 120 ms |
| Idle memory | 120 MB | 250 MB |
| Installer size | 90 MB | 150 MB |

## 6 · The support tiers

| Tier | Meaning | Commitment |
| --- | --- | --- |
| Supported | In §1, within the vendor's support window | Security fixes, bug fixes, Qt patches |
| Best effort | Older but not end-of-life; or an unsupported display server | Fixes only when trivial |
| Unsupported | §3, or outside §1 | Not installed, not tested, not diagnosed |

## 7 · How this table changes

| Trigger | Change |
| --- | --- |
| A vendor ends support for a release | Move it to §3 in the next release |
| A Qt LTS moves the baseline | Change §1 and `cmake/qt.cmake` together |
| The floor profile is no longer sold | Re-measure the budgets and update §4 and §5 |
| A new architecture is needed | Add it to §1 and to the packaging matrix ([11](11-packaging-and-updates.md)) |

## See also

- [01 — Support Matrix & Reference Hardware](01-support-matrix-and-reference-hardware.md)
- [The No-GPU Fallback](03-no-gpu-fallback.md)
- [Packaging & Updates](11-packaging-and-updates.md) — what is built per
  platform.
