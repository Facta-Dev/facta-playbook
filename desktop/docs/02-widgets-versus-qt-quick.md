# 02 — Widgets versus Qt Quick

## The decision

**The UI is Qt Widgets.** Not "Widgets for now" — Widgets is the default and
QML is the exception that has to be argued for in an ADR.

The full argument is in
[ADR 0001](adr/0001-qt-widgets-over-qt-quick.md); this chapter is the working
summary and the criteria you apply when someone proposes a QML screen.

## The criteria

| Criterion | Widgets | Qt Quick |
| --- | --- | --- |
| Time to first interactive screen | Days | Weeks — a designer, a build pipeline for QML, a review loop |
| Designer tooling | Native OS controls, native accessibility | Separate toolchain, its own accessibility story |
| Testability in CI | `QT_QPA_PLATFORM=offscreen`, widgets and models are inspectable | Needs a QML engine in the test, harder to introspect |
| Long-term maintenance | One toolkit, one set of skills | Two toolkits, two sets of skills, two upgrade paths |
| GPU requirement | None — Widgets paint with the raster engine | Usually needs a working GL/Vulkan/RHI stack |
| Animation and custom visuals | Possible, expensive | Native strength |
| Memory per screen | Moderate | Lower, but with a scene graph and a shader pipeline |

The deciding factors are **time to first screen**, **testability in CI** and
**the GPU floor**. A desktop client that must run on a virtual machine with no
GPU cannot take on a hard dependency on a scene graph as its default path.

## What the template actually ships

Widgets, end to end:

| Element | Class |
| --- | --- |
| Window | `QMainWindow` (`desktop::app::NotesWindow`) |
| Input | `QLineEdit` wrapped in `desktop::ui_kit::LabeledLineEdit` |
| List | `QListView` + `QAbstractListModel` (`desktop::notes::presentation::NotesListModel`) |
| Panel | `desktop::ui_kit::ListPanel` |
| Theme | one application stylesheet (`desktop::ui_kit::styleSheet()`) |

`cmake/qt.cmake` requests `Core Gui Widgets Network Sql` — **Qt Quick is not
a dependency** (`Quick`, `Qml`, `QuickWidgets2` are absent from the
`find_package` call, and `Qt6::Quick` appears in no `target_link_libraries`).
That is the decision, encoded in the build.

`main.cpp` sets `QT_QUICK_BACKEND=software` in Compatibility Mode. With no
Qt Quick linked the variable is inert today; it is there so that adding a
`QQuickWidget` later cannot silently reintroduce a GPU requirement in the
software-rendering path.

## The narrow door to Qt Quick

A QML screen is admissible only when **all** of these hold:

1. The screen cannot be expressed with widgets — a live canvas, a 60 fps
   visualisation, a hardware-accelerated timeline.
2. It degrades to a widget screen when the GPU is unavailable, or it is
   inside a surface that already has a static fallback.
3. It is isolated: one view, one `QQuickWidget` host, behind the same view
   model as the rest of the UI. The view model does not know that QML
   exists.
4. The build links `Qt6::Quick` explicitly and the ADR is updated in the
   same change.

Everything else stays Widgets. A new form, a new list, a new settings page
is a widget job.

## Why not "one UI framework, chosen later"

The cost of "later" is the entire UI: migrating a Widgets tree to QML means
rewriting every screen, every model binding and every test. Deciding at
template time costs one paragraph; deciding at month six costs the product.

## Consequences of the decision

**Accepted:**

- Native controls, native keyboard handling, native accessibility on all
  three desktop platforms.
- The CI suite runs headless with `QT_QPA_PLATFORM=offscreen`; no scene
  graph, no GPU in the test container.
- Animations are Qt's built-in effects, and they are disabled in
  Compatibility Mode ([03](03-no-gpu-fallback.md)).

**Accepted, with a known gap:**

- The shipped stylesheet is a single light theme applied to the whole
  application. It overrides the native look, so the app does not look native
  on macOS, and there is no dark or high-contrast theme yet. See
  `src/ui-kit/src/theme.cpp`; the fix is to ship a theme per appearance mode
  and to stop overriding the native style where it is not needed.

**Rejected:**

- QML as the default UI.
- Web content embedded in the shell: it brings a Chromium runtime, its own
  update policy and its own attack surface, and it is not needed by a client
  whose UI is forms and lists.

## Do / don't

| Do | Don't |
| --- | --- |
| Reach for `QWidget` + `QAbstractItemModel` first | Add a `QQuickWidget` "just for one screen" |
| Keep the view model free of any Qt Quick type | Let the QML layer become a second source of truth |
| Keep the list a model, not a widget per row | Build a `QWidget` per row for 10 000 rows |
| Disable the built-in effects in Compatibility Mode | Ship an animation that only works on a GPU |

## See also

- [ADR 0001 — Qt Widgets over Qt Quick](adr/0001-qt-widgets-over-qt-quick.md)
- [The No-GPU Fallback](03-no-gpu-fallback.md)
- [Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md) — why the view
  model, and not the widget, holds the state.
- [`cmake/qt.cmake`](../cmake/qt.cmake) — the components the build requests.
