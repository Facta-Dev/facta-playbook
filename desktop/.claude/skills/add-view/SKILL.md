---
name: "add-view"
description: "Adds a Widgets view to an <app> slice: a QObject view model with the standard status set, an item model, the window that renders them, and the offscreen test that proves the states."
division: client
---

# `<app>` add-view skill

> Reusable workflow for adding a screen to an existing `<app>` slice. An agent
> invokes this skill; the requester supplies the screen, the skill supplies
> the MVVM wiring, the Widgets discipline and the test.

## When to Activate

Activate this skill when the request is a new screen or a new widget in an
existing slice: a detail panel, a list with an empty state, a settings form, a
filter bar.

Do NOT activate this skill when:

- The screen needs a new domain rule or a new use case — add the slice first
  (`add-slice`) or the use case first.
- The request is a pure visual tweak of an existing screen.
- The request introduces a Qt Quick surface: it needs an ADR before this
  skill runs.

## Core Principles

- **The view renders state; it does not compute it.** A widget reads the view
  model and sends intents back. No business rule lives in a slot handler.
- **One status set.** The view model exposes the standard statuses —
  `Loading`, `Ready`, `Empty`, `Forbidden`, `NotFound`, `Offline`, `Error`,
  `PolicyBlocked` — and the view has one rendering per status.
- **Reuse the ui-kit.** Check `src/ui-kit/` before writing a widget; a
  duplicated control is a finding.
- **Widgets, CPU rendering.** No Qt Quick, no shader effects, no assumption
  that a GPU exists; the layout must survive Compatibility Mode.
- **Async by default.** The view model asks a use case and receives a
  `QFuture<Result<T>>`; it never blocks.

## Rules

- No GUI type reaches the domain or the application layer: the view model may
  use `QObject` and `Q_PROPERTY`, and nothing below `presentation/` may.
- The view model is `QObject` with `Q_PROPERTY` and emits a signal on every
  status change; the view never polls.
- No blocking call on the GUI thread: no `QFuture::result()`, no
  `waitForReadyRead()`, no `QThread::sleep`.
- Every screen implements the empty state, the error state and the loading
  state — a screen that only renders `Ready` is incomplete.
- Accessibility basics are mandatory: accessible name on every interactive
  widget, keyboard reachability for every action, visible focus.
- The test runs headless with `QT_QPA_PLATFORM=offscreen` and asserts on the
  view model state and on the item model, not on pixels.
- Report the real output of the build and the test run.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The view model exposes observable properties and the view reacts to signals | Qt documentation | 2025 | https://doc.qt.io/qt-6/qobject.html |
| Item models should update with precise range signals instead of resetting | Qt documentation | 2025 | https://doc.qt.io/qt-6/qabstractitemmodel.html |
| Widgets rasterize on the CPU, which is why the template is widgets-first | `PLAYBOOK_DESKTOP_QT.md` §2.3 | - | `PLAYBOOK_DESKTOP_QT.md` |
| Headless GUI tests use the offscreen platform plugin | `cmake/qt.cmake` (`desktop_add_test()`) | - | `cmake/qt.cmake` |
| The shared widgets and theme to reuse | `src/ui-kit/` | - | `src/ui-kit/CMakeLists.txt` |

## Output Format

```
View: <name> — slice: <slice>
Files:
  presentation/  <view model, item model>
  <ui files>     <window and widgets>
States implemented: Loading · Ready · Empty · Forbidden · NotFound · Offline · Error · PolicyBlocked
Reused from ui-kit: <widgets> — or: none needed
Accessibility: <accessible names, focus order>
Gates:
  cmake --build build → <result>
  QT_QPA_PLATFORM=offscreen ctest --test-dir build -R <suite> → <result>
Open: <what the next screen would need>
```
