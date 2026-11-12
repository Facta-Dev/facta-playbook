# 01 — Support Matrix & Reference Hardware

## The rule of the intersection

A desktop client is supported on an operating system when **all three** of
these hold:

1. the OS is still receiving vendor security updates,
2. the Qt version the app links against still supports it,
3. the rendering path (GPU, or the software fallback) works on it.

The supported set is the **intersection**, never the union. An OS that Qt
dropped is not supported by us, however old our toolchain is.

## Minimum versions

The per-release table with end-of-life dates lives in
[Support Matrix](support-matrix.md). The short form:

| Platform | Minimum | Recommended | Notes |
| --- | --- | --- | --- |
| Windows | Windows 11 23H2 | Latest 24H2+ | Windows 10 22H2 only under ESU; 64-bit only |
| macOS | macOS 13 Ventura | Latest | Universal binary, arm64 and x86_64 |
| Linux (glibc) | Ubuntu 22.04 LTS (glibc 2.35) | Ubuntu 24.04 LTS | Build on the oldest supported glibc |
| Linux (rolling) | Fedora 41+ | Current | glibc newer than the floor is fine |
| Wayland | XWayland not required | Native Wayland | Widgets run on the XWayland bridge today |
| X11 | Any X11 server | — | Fallback path for remote sessions |

Qt baseline: **Qt 6.8 or newer**. The template resolves
`find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network Sql)` and
adds `Test` when `ENABLE_TESTS` is on. Qt 6.8 is the floor because it is the
first LTS with the Wayland and high-DPI behaviour the template relies on.
Raise the floor in `cmake/qt.cmake` and re-run the matrix when you move to a
newer LTS.

## Reference hardware

The budgets come from the playbook and are the numbers to test against, not
the numbers a developer machine happens to have.

| Profile | CPU | RAM | Disk | Display | GPU |
| --- | --- | --- | --- | --- | --- |
| **Floor** | 2 cores | 4 GB | HDD, 1 GB free | 1366 × 768 | None — software rendering |
| **Typical** | 4 cores | 8 GB | SSD | 1920 × 1080 | Integrated |
| **High-DPI** | 4 cores | 16 GB | SSD | 2560 × 1440 @ 150 % | Integrated |
| **Remote desktop** | 2 vCPU | 4 GB | — | 1280 × 720 | None — software rendering |

Two floors decide the product, and both are in the playbook:

- **No GPU.** The app must be usable on a virtual machine, over RDP or on a
  machine with a blacklisted driver. That is the whole reason
  [Compatibility Mode](03-no-gpu-fallback.md) exists.
- **A hard disk.** The startup budget must be met on rotational media, which
  is why the startup path touches as little as possible.

## The budgets

Measured on the **floor** profile, not on a workstation.

| Budget | Target | Ceiling | Where it is spent |
| --- | --- | --- | --- |
| Cold start to first window | 1.5 s | 3.0 s | Process start, style sheet, first paint |
| Warm start to first window | 0.6 s | 1.2 s | Platform init, `QStandardPaths` lookups |
| UI input latency | 60 ms | 150 ms | Slot entry to repaint |
| List scroll (10 000 rows) | 16 ms | 33 ms | `QAbstractItemModel` paint |
| Network list refresh | 400 ms | 1 200 ms | Server round trip plus parse |
| Local list read (10 000 rows) | 30 ms | 120 ms | SQLite query and map |
| Idle memory | 120 MB | 250 MB | Widgets, model, SQLite page cache |
| Installer size | 90 MB | 150 MB | Payload plus Qt runtime |

## What the hardware floor implies for the code

- **The first paint must not wait for the network.** The window opens with the
  cached projection ([08](08-storage-and-migrations.md)); the refresh happens
  after it is visible.
- **The list must be a model, not a widget per row.** `QAbstractListModel`
  with `QListView` is the shipped pattern; 10 000 rows are a data question,
  not a widget question.
- **The database must be opened once.** `NoteStore` opens the connection and
  runs the migrations at startup, on the GUI thread — see
  [07](07-threading-and-the-gui-thread.md) for why that is the one blocking
  call the template still makes, and what it must become.
- **The style sheet must be cheap.** One `QApplication` stylesheet, applied
  once, no per-widget runtime `setStyleSheet` in a loop.

## Support tiers

| Tier | Meaning | Commitment |
| --- | --- | --- |
| **Supported** | In the [Support Matrix](support-matrix.md), latest two OS releases of each platform | Security fixes, bug fixes, new Qt patches |
| **Best effort** | Older but not end-of-life, or an unsupported display server | No new features; fixes only if trivial |
| **Unsupported** | End of life, or outside the matrix | Not installed, not tested, not diagnosed |

An OS that leaves the supported set is not patched by the team; it is
*declared* unsupported. The declaration lives in the support matrix, which
is why that document is a product artifact and not a note to self.

## What is explicitly not supported

The template documents the exclusions in
[Support Matrix §3](support-matrix.md) — the short form:

| Excluded | Why |
| --- | --- |
| 32-bit Windows and Linux | Qt 6 dropped 32-bit support; no official Qt binaries |
| Windows 7 / 8 / 8.1, macOS 10.15 and older | End of vendor security updates |
| Linux distributions without a rolling glibc model (musl, Alpine) | Qt 6 is not shipped for musl; the SQL driver set differs |
| Mobile and embedded (iOS, Android, QNX) | The template is a desktop client; a separate kernel and a different store are required |
| Web assembly | Widgets need a native event loop and a desktop window system |
| Multi-process rendering of the same SQLite file | One writer per file; the design forbids a second process |
| Offline installation on a machine without the runtime prerequisites | The installers bundle the Qt runtime |

## See also

- [Support Matrix](support-matrix.md) — the per-release table.
- [The No-GPU Fallback](03-no-gpu-fallback.md) — the floor profile in practice.
- [Playbook Desktop Qt §2](../PLAYBOOK_DESKTOP_QT.md) — the source of the
  floor profile and the budgets.
