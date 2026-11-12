# ADR 0001 — Qt Widgets over Qt Quick

| | |
| --- | --- |
| **Status** | Accepted |
| **Date** | 2026-11-12 |
| **Scope** | The UI toolkit of the desktop template |
| **Supersedes** | — |

## Context

The desktop template needs a UI toolkit. Qt offers two: **Widgets**
(`QWidget`, `QAbstractItemModel`) and **Qt Quick** (QML, `QQuickItem`).
They are not interchangeable: a Widgets screen and a QML screen are different
code, different tests and different skills.

The choice is made once, at template time, and it shapes every screen,
every designer conversation, every test container and every hire.

The constraints that decided it:

1. **No GPU is a supported floor.** The app must run on a virtual machine,
   over RDP and on a machine with a blacklisted driver
   ([01 — Support Matrix](../01-support-matrix-and-reference-hardware.md)).
2. **CI runs headless.** The suite runs in a container with no display
   server, through `QT_QPA_PLATFORM=offscreen`.
3. **The UI is forms and lists.** The app shows forms, a list and a status
   line — not an animation, not a live canvas, not a video.
4. **Time to first screen matters.** A template exists to be started, not to
   be studied.

## Decision

**The UI is Qt Widgets.** QML is not part of the template, and `Qt6::Quick`
is not a dependency of any target.

The build requests exactly the modules the UI needs:

```cmake
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network Sql)
```

`Quick`, `Qml` and `QuickWidgets2` are absent from that list, and no
`target_link_libraries` mentions `Qt6::Quick`. The decision is encoded in
the build, not in a comment.

### The fallback path

The rule for introducing QML is narrow and explicit. A QML screen is
admissible only when **all** of these hold:

1. The screen cannot be expressed with widgets — a live canvas, a 60 fps
   visualisation, a hardware-accelerated timeline.
2. It degrades to a widget screen when the GPU is unavailable, or it is
   inside a surface that already has a static fallback.
3. It is isolated: one view, one `QQuickWidget` host, behind the same view
   model as the rest of the UI. The view model does not know that QML
   exists.
4. `Qt6::Quick` is linked explicitly and **this ADR is updated in the same
   pull request**.

The compatibility mode already sets `QT_QUICK_BACKEND=software` before the
`QApplication` is constructed, so a future QML surface cannot silently
reintroduce a GPU requirement in the software-rendering path.

### The migration path

If a product outgrows Widgets, the migration is incremental and does not
start from zero:

| Step | What it costs |
| --- | --- |
| Adopt QML for one new screen | One `QQuickWidget` host, one view model, no change to the layers |
| Keep the view models | The QML layer binds to the same `QAbstractItemModel` and the same `Q_PROPERTY` |
| Migrate one screen at a time | Each screen is behind its own host widget |
| Reach for the point of no return | Never — the layers do not depend on the toolkit |

The architecture is what makes the migration possible: because the view
model is a `QObject` with properties and an item model, a QML view can
consume it without changing a line of the application layer.

## Alternatives considered

### Qt Quick (QML) as the default

**Rejected.** QML's advantages are real: faster to design rich, animated
interfaces, and a better rendering model for continuous updates. Against
that:

- It adds a second toolchain — a designer, a QML review loop, a build step
  for the QML cache, and a second set of skills on the team.
- It makes the CI story harder: a QML engine in a headless container is more
  fragile than `QT_QPA_PLATFORM=offscreen` with Widgets.
- It puts a scene graph in the default path, which is exactly what the
  no-GPU floor forbids.
- For forms and lists, Widgets is faster to build, not slower.

The decisive argument is the third one: the template's floor profile has no
GPU, and a QML-first design would make that floor a special case instead of
the default.

### Qt Quick for the whole UI, Widgets as a fallback

**Rejected.** It is the worst of both: two toolchains, and the fallback —
the case that matters most — is the one nobody tests.

### A web view as the UI

**Rejected.** It brings a Chromium runtime, its own update policy, a large
attack surface and a second rendering model, to build a form.

### One screen in QML, the rest in Widgets

**Rejected for now**, and permitted later under the fallback path above.
The migration cost of a single QML screen is low; the *organisational* cost
of two toolchains in a small team is not.

## Consequences

**Accepted:**

- Native controls, native keyboard handling and native accessibility on all
  three desktop platforms.
- The test suite runs headless with no scene graph and no GPU.
- The compatibility mode is cheap: no animations, no shaders, no fallback
  renderer to write.
- One toolkit, one set of skills, one upgrade path.

**Accepted, with a known gap:**

- The template ships one light stylesheet applied to the whole application
  (`desktop::ui_kit::styleSheet()`), which overrides the native look. There
  is no dark theme and no high-contrast theme. The fix is a theme per
  appearance mode — it does not require changing the toolkit.
- Widgets-based animation is limited to Qt's built-in effects, and they are
  disabled in compatibility mode.

**Consequences for the code:**

- `src/ui-kit/` holds `QWidget` subclasses; no `.qml` file exists in the
  tree.
- The architecture suite checks that the domain never includes a QtWidgets
  header — and the same reasoning keeps Qt Quick out of the domain.
- A QML screen, if one is ever added, must live behind a view model, never
  in the domain or the application layer.

## See also

- [02 — Widgets versus Qt Quick](../02-widgets-versus-qt-quick.md) — the
  working version of this decision.
- [03 — The No-GPU Fallback](../03-no-gpu-fallback.md) — the floor the
  decision protects.
- [ADR 0002 — One static library per layer](0002-one-static-library-per-layer.md)
- [Playbook Desktop Qt §2](../../PLAYBOOK_DESKTOP_QT.md) — the decision this
  ADR implements.
