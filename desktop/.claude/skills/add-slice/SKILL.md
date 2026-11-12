---
name: "add-slice"
description: "Scaffolds a new vertical slice in the <app> desktop tree — domain, application, adapters, presentation and tests — with the CMake targets, the ports and the composition-root wiring that keep the layer law intact."
division: core
---

# `<app>` add-slice skill

> Reusable workflow for adding one vertical slice to `<app>`. An agent invokes
> this skill; the requester supplies the feature, the skill supplies the
> scaffolding, the layer discipline and the gate ladder.

## When to Activate

Activate this skill when the request is a new feature that owns its own
domain — a new kind of entity, screen or workflow that will grow: a new
"tasks" slice, a new "settings" slice, a new billing slice.

Do NOT activate this skill when:

- The change adds a field, a rule or a screen to an **existing** slice — that
  is a change inside `src/features/<slice>/`, not a new slice.
- The change is a bug fix (fix it in place, with a red test first).
- The request is a cross-cutting refactor of the composition root.

## Core Principles

- **Vertical, not horizontal.** A slice owns one feature end to end: domain
  rules, use cases, adapters and presentation. Nothing about the feature is
  scattered across `src/infrastructure/` or `src/app/`.
- **Four layers, four CMake targets.** `domain`, `application`, `adapters` and
  `presentation`, each with its own target, so a wrong dependency fails at
  link time.
- **Ports are consumer-defined.** The interface a use case needs lives in
  `application/`, next to the use case; the adapter that satisfies it lives in
  `adapters/` and may depend on the SDK, the store or the filesystem.
- **The composition root wires, it does not decide.** `src/app/main.cpp`
  constructs the adapters, hands them to the use cases and opens the window.
- **Mirror the `notes` slice.** The template already ships a worked example;
  copy its shape rather than inventing a new one.

## Rules

- No fabricated paths: every file created must be listed with its real path,
  and every existing path referenced must exist in the tree.
- The domain and the application carry **no** GUI, network or SQL types; a
  `QWidget`, `QNetworkAccessManager` or `QSqlQuery` in those layers is a
  defect, not a style choice.
- Each layer calls `desktop_assert_no_forbidden_include()` with its forbidden
  set, exactly as `src/features/notes/CMakeLists.txt` does.
- The slice ships with at least one test per layer: a domain test, an
  application test against the fakes in `tests/_shared/fakes/`, a presentation
  test on the view model, and an architecture test if the slice introduces a
  new include rule.
- No new third-party dependency without an ADR; the Qt modules already
  enabled are enough for a slice.
- No step is "done" until `cmake --preset dev && cmake --build build` and
  `QT_QPA_PLATFORM=offscreen ctest --test-dir build` are green; report the
  real output.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Per-target `target_link_libraries` makes a dependency violation fail at link time | CMake documentation | 2025 | https://cmake.org/cmake/help/latest/command/target_link_libraries.html |
| A consumer-owned interface is the hexagonal/ports-and-adapters rule the layer law encodes | `PLAYBOOK_DESKTOP_QT.md` §3 | - | `PLAYBOOK_DESKTOP_QT.md` |
| Qt Test suites are registered per target with the offscreen environment applied | `cmake/qt.cmake` | - | `cmake/qt.cmake` |
| The worked slice to mirror | `src/features/notes/` | - | `src/features/notes/CMakeLists.txt` |

## Output Format

```
Slice: <name> — path: src/features/<name>/
Created:
  domain/        <files> — <what they hold>
  application/   <files> — <use cases, ports>
  adapters/      <files> — <port implementations>
  presentation/  <files> — <view model, item model>
  tests/         <files> — <cases>
CMake targets: <target> → <links>
Wiring: <what src/app/main.cpp now constructs>
Gates:
  cmake --preset dev → <result>
  cmake --build build → <result>
  QT_QPA_PLATFORM=offscreen ctest --test-dir build → <result>
Open: <anything left for the next slice>
```
