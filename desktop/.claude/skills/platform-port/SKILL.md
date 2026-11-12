---
name: "platform-port"
description: "Ports an OS-specific capability into <app>'s src/platform: a new keystore backend, app paths, single instance or notification, with the portable interface, the per-OS implementation and the boundary test."
division: security
---

# `<app>` platform-port skill

> Reusable workflow for adding an operating-system capability to `<app>`
> without leaking it into the portable layers. An agent invokes this skill;
> the requester supplies the capability, the skill supplies the interface, the
> per-OS implementation and the boundary check.

## When to Activate

Activate this skill when the request needs a decision only the operating
system can make: the credential store, the application data directory, the
single-instance lock, the system tray, the notification centre, the file
dialog, the deep-link handler, or an OS-version behaviour.

Do NOT activate this skill when:

- The behaviour is portable and Qt already provides it (use the Qt API
  directly, e.g. `QStandardPaths`).
- The request is to change what the app does rather than where the OS is
  consulted.
- The capability is already behind an interface in `src/platform/` — extend
  that interface instead of adding a second one.

## Core Principles

- **One interface, N implementations.** The portable code calls a stable
  interface in `src/platform/`; each OS provides its own implementation of
  it. The application never learns which OS it runs on.
- **`#ifdef Q_OS_*` only in `src/platform/`.** No OS header, no platform
  branch, no platform-specific type outside that directory — the architecture
  test under `tests/_platform/architecture/` enforces it.
- **Fail closed.** When a capability is unavailable (no keystore, no
  notification daemon), the implementation reports the failure to the caller
  instead of silently degrading to something unsafe.
- **Secrets stay in the OS.** The interface handles a credential; it never
  returns it in plain text to a layer that would log it, and never writes it
  to `QSettings` or to a file.
- **One new behaviour at a time.** A port is small: interface, one
  implementation, one test, then the next OS.

## Rules

- No fabricated platform API: verify the call against the installed Qt
  version and the target OS documentation before using it; if it cannot be
  verified here, say so in the report.
- No OS header outside `src/platform/`, and no `Q_OS_*` branch in a feature,
  in the ui-kit or in the composition root.
- The new interface is documented with its failure modes and its thread
  affinity, and it never blocks the GUI thread.
- A credential is never logged, never stored in `QSettings`, never included
  in an error message, and never written to a crash dump.
- Every implementation has a test: the interface against the fakes, the
  implementation behind the platform abstraction with whatever the OS allows
  in CI.
- The architecture suite is run after the change and reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The platform directory is the only place allowed to branch on the operating system | `PLAYBOOK_DESKTOP_QT.md` §3 | - | `PLAYBOOK_DESKTOP_QT.md` |
| The layer law is asserted by an executable test over the source tree | `tests/_platform/architecture/` | - | `tests/_platform/architecture/` |
| `QStandardPaths` is the portable location API | Qt documentation | 2025 | https://doc.qt.io/qt-6/qstandardpaths.html |
| Secrets belong in the OS credential store, never in the repository | `PLAYBOOK_SECURITY.md` | - | `PLAYBOOK_SECURITY.md` |
| The platform CMake target and its per-OS sources | `src/platform/CMakeLists.txt` | - | `src/platform/CMakeLists.txt` |

## Output Format

```
Capability: <what the OS must provide>
Interface: <path> — <method signatures> — failure modes: <list>
Implementations:
  <os>  <path> — <mechanism> — verified: <how, or "not verifiable here">
Wiring: <what the composition root or the slice now constructs>
Boundary check: <Q_OS_* / OS headers found outside src/platform → none>
Secret handling: <where the credential lives, and where it never goes>
Gates:
  cmake --build build → <result>
  QT_QPA_PLATFORM=offscreen ctest --test-dir build → <result>
Open: <the next OS, if any>
```
