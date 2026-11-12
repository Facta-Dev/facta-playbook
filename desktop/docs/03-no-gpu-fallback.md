# 03 — The No-GPU Fallback

## The problem

A desktop client must start on machines with no usable GPU:

- virtual machines and remote desktops (RDP, VDI),
- Windows with a blacklisted or outdated driver,
- old laptops with a driver that renders black,
- Linux sessions over XWayland with no EGL,
- CI containers, which have no GPU at all.

The failure is not always a crash. Often it is a window that opens and stays
black, or a client that hangs while the scene graph negotiates a context. A
black window is a support ticket, and the fix has to be reachable without
the user editing configuration files.

## The two mechanisms

### 1. Compatibility Mode — forced by flag or by policy

The user (or an administrator) starts the app in software rendering:

```sh
<app>-desktop --software-rendering
```

`src/app/main.cpp` reads the flag **from `argv` before `QApplication` is
constructed**, because the renderer must be chosen before the first window
exists and `QApplication::arguments()` needs an instance:

```cpp
const bool software = hasFlag(argc, argv, "--software-rendering");
if (software) {
    qputenv("QT_QUICK_BACKEND", "software");
    qputenv("QT_OPENGL", "software");
}
```

The environment variables are set **before** `QApplication app(argc, argv)`,
which is what makes them effective — the Qt platform plugin reads them while
it initialises. After construction, the app also disables the UI effects
that cost frames on a CPU rasteriser:

```cpp
QApplication::setEffectEnabled(Qt::UI_AnimateCombo, false);
QApplication::setEffectEnabled(Qt::UI_AnimateMenu, false);
QApplication::setEffectEnabled(Qt::UI_AnimateTooltip, false);
```

### 2. Safe mode — forced by the environment

If the app cannot start at all, the user sets an environment variable and
the app starts with the smallest possible surface: no auto-start of the
sync worker, no telemetry, no network calls, no update check.

Safe mode is what the user reaches for when the app crashes on launch. The
template declares the seam in the contracts (`desktop::contracts::Policy`
has a `compatibility_mode` key) but **the resolver is not implemented** —
see the gap below.

## The order of precedence

When more than one source asks for a mode, the stricter one wins:

| Source | Effect |
| --- | --- |
| `--safe-mode` (flag) | Highest: safe mode |
| `--software-rendering` (flag) | Software rendering |
| Managed policy `compatibility_mode` | Software rendering |
| Managed policy `kill_switch` | The app refuses to start |
| `QT_QUICK_BACKEND` / `QT_OPENGL` in the environment | Advisory, respected as-is |
| Detection at startup | Not implemented in the template |

## What the template ships today

| Piece | State | Where |
| --- | --- | --- |
| `--software-rendering` flag | **Shipped** | `src/app/main.cpp` |
| `QT_QUICK_BACKEND=software`, `QT_OPENGL=software` | **Shipped** | `src/app/main.cpp` |
| UI effects disabled in software mode | **Shipped** | `src/app/main.cpp` |
| Safe mode | **Not implemented** | — |
| Automatic GPU detection and fallback | **Not implemented** | — |
| Crash-loop detection ("the app crashed 3 times, start in safe mode?") | **Not implemented** | — |
| Policy-driven compatibility mode | **Not implemented** — the schema exists in `src/contracts/` | [Admin Guide](admin-guide.md) |

The last three are the honest gaps. Automatic detection is a product
decision with a false-positive cost: an app that silently drops to software
rendering on a machine that could have used the GPU is a performance bug
report. The template therefore ships the **explicit** switch and leaves
detection to the product, where the crash history lives.

## Safe mode, when it is implemented

The contract is fixed by the playbook; the implementation is a gap. Safe
mode must:

1. Skip the sync worker, the update check and every telemetry emission.
2. Show a window that says "safe mode" and offers "retry normally" and
   "open the log folder".
3. Write a marker file in the runtime directory so the next launch knows.
4. Never write to the database except to read it.

The safe-mode flag belongs next to `--software-rendering` in `main.cpp`,
parsed from `argv` for the same reason: it must be known before anything
else starts.

## Testing the fallback

The fallback is testable without a GPU:

```sh
make build
QT_QPA_PLATFORM=offscreen <build-dir>/<app>-desktop --software-rendering
```

The CI suite already runs headless — `cmake/qt.cmake` sets
`QT_QPA_PLATFORM=offscreen` for every test target — so a widget test never
needs a display server. What CI cannot prove is the *visual* result of
software rendering; that needs a manual pass on the floor profile from
[01 — Support Matrix](01-support-matrix-and-reference-hardware.md), and it
belongs in the release checklist, not in the unit suite.

## Do / don't

| Do | Don't |
| --- | --- |
| Choose the renderer before `QApplication` is built | Set `QT_OPENGL` after the app exists |
| Give the user a flag that does one thing | Add an undocumented combo of environment variables |
| Disable animations in software mode | Ship a 60 fps animation that needs a GPU |
| Treat the floor profile as a release criterion | Test only on the developer's machine |

## See also

- [Widgets versus Qt Quick](02-widgets-versus-qt-quick.md) — why the
  fallback is cheap in Widgets.
- [Support Matrix & Reference Hardware](01-support-matrix-and-reference-hardware.md)
- [Admin Guide](admin-guide.md) — the policy keys that will drive the mode.
- [`src/app/main.cpp`](../src/app/main.cpp)
