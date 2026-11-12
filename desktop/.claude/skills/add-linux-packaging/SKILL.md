---
name: "add-linux-packaging"
description: "Adds Linux packaging to the <app> desktop app: a .desktop entry, AppStream metadata, AppImage, deb, rpm and Flatpak targets with the install rules they depend on."
division: client
---

# `<app>` add-linux-packaging skill

> Reusable workflow for packaging the `<app>` desktop app for Linux. An agent
> invokes this skill; the requester supplies the target formats, the skill
> supplies the metadata, the install rules and the checks.

## When to Activate

Activate this skill when the request is to package the `<app>` app for Linux:
a `.desktop` entry, AppStream metadata, an AppImage, a `.deb`, an `.rpm`, a
Flatpak manifest, or a tarball for a system package.

Do NOT activate this skill when:

- The target is Windows or macOS — those have their own installers, and the
  Qt Widgets tree they package is the same.
- The request is a code change inside the app; packaging does not change
  behaviour.
- The template has no `packaging/` tree yet and the request is only to
  document the matrix: write the matrix, do not invent the artifacts.

## Core Principles

- **One identity, every format.** The application id, the human name, the
  version and the icon come from one source (`DESKTOP_VERSION` in
  `cmake/versioning.cmake` plus the metadata files), so the `.desktop` entry,
  the AppStream record and the package name cannot drift apart.
- **Install rules live in CMake.** `install(TARGETS …)` and the runtime
  dependency policy are declared in the build, not copied by hand into a
  packaging script.
- **The bundle is self-contained.** The Qt runtime and the required platform
  plugins (`platforms/`, `imageformats/`) travel with the artifact, and the
  app starts on a machine with no GPU through the software rendering path.
- **Least privilege.** The Flatpak manifest asks for the narrowest sandbox
  that works: the user data directory, and nothing else.
- **Reproducible.** The same commit and the same tool versions produce the
  same artifact, and the artifact is built in CI, never on a laptop.

## Rules

- The version in the package metadata must equal `DESKTOP_VERSION`; a
  mismatch is a defect, not a cosmetic issue.
- No secret, key or private URL in any packaged file, in the `.desktop`
  entry or in the AppStream metadata.
- The `.desktop` entry declares `Type=Application`, the right `Categories`,
  `StartupWMClass` matching the window class, `Terminal=false` and an
  `Icon` that resolves after installation.
- The AppStream record needs an id, a name, a summary, a description, a
  licence, a release and screenshots; the licence must match the one in the
  repository.
- The Flatpak manifest pins every runtime and every module version; no
  floating `latest`.
- No installer writes outside the directories the format allows (no script
  writing to `/usr/local` outside the package's own file list).
- The artifact is verified: install it in a clean container or VM, launch it
  with `--software-rendering`, and report the real result.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| The desktop entry fields the format requires | freedesktop.org Desktop Entry Specification | 2025 | https://specifications.freedesktop.org/desktop-entry-spec/latest/ |
| AppStream metadata is the standard store metadata for Linux apps | freedesktop.org AppStream | 2025 | https://specifications.freedesktop.org/appstream-spec/latest/ |
| Flatpak manifests pin the runtime and the modules used | Flatpak documentation | 2025 | https://docs.flatpak.org/en/latest/flatpak-command-reference.html |
| The version packaged comes from the CMake cache variable | `cmake/versioning.cmake` | - | `cmake/versioning.cmake` |
| The packaging matrix per operating system | `PLAYBOOK_DESKTOP_QT.md` §14 | - | `PLAYBOOK_DESKTOP_QT.md` |

## Output Format

```
Linux packaging for: <app>
Metadata:
  desktop entry  <path> — id, categories, icon, terminal
  appstream      <path> — id, licence, release, screenshots
Artifacts:
  AppImage  <path> — <size> — verified: <how>
  deb      <path> — <size> — dependencies: <list>
  rpm      <path> — <size> — dependencies: <list>
  Flatpak  <path> — runtime <version>, modules <pinned>
  tarball  <path> — <size>
Install rules: <CMake install() targets>
Verification: <clean install + launch with --software-rendering → result>
Not packaged: <what was left out and why>
```
