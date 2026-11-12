# 11 — Packaging & Updates

## What the template ships

**Nothing.** There is no `packaging/` directory, no installer definition, no
signing step, no updater and no update manifest. The template builds a
binary:

```sh
make build PRESET=release     # → build-release/
```

Everything in this chapter is a gap until a product decides on it.

## The artifacts

One artifact per operating system, each self-contained: the Qt runtime is
bundled, so the app does not require the user to install Qt.

| Platform | Artifact | Runtime bundled | Notes |
| --- | --- | --- | --- |
| Windows x64 | `.exe` installer (MSI or NSIS) | `windeployqt` | Authenticode signed, per-machine or per-user |
| Windows arm64 | `.exe` installer | `windeployqt` | Only if the product needs it |
| macOS universal | `.dmg` or `.pkg` | `macdeployqt` | Notarised and stapled, Developer ID signed |
| Linux x64 | `.AppImage`, `.deb`, `.rpm` | `linuxdeployqt` | Also a tarball for the enterprise channel |
| Linux arm64 | `.AppImage`, `.deb` | `linuxdeployqt` | — |

## The build pipeline

The pipeline that the template does not have yet:

1. **Build** — `cmake --preset release` and `cmake --build build-release`.
2. **Deploy the Qt runtime** — `windeployqt`, `macdeployqt` or
   `linuxdeployqt`, per platform.
3. **Assemble** — the installer or the image.
4. **Sign** — Authenticode, Developer ID + notarisation, or GPG for Linux.
5. **Publish** — the artifact to the update channel and, separately, to the
   enterprise channel.
6. **Record** — the version, the checksums, the signature, the SBOM.

Each step is a separate job with a separate credential: a signing key never
touches a build job. The version of the artifact and the version the app
reports at runtime are the same string: `DESKTOP_VERSION` is compiled in
(`cmake/versioning.cmake`) and is also set as
`QApplication::applicationVersion()` in `main.cpp`.

## Signing and notarisation

| Platform | Mechanism | Requirement |
| --- | --- | --- |
| Windows | Authenticode (a code-signing certificate) | The installer and the binary are signed; the certificate is in the Windows Trusted Root store |
| macOS | Developer ID Application + notarisation | Notarisation by Apple is required for Gatekeeper to allow the app on a current macOS |
| Linux | GPG or a package signature | Package signatures; AppImages are usually distributed with a checksum |

## The update channel

Updates are a **decision the client does not make alone**. The model:

| Piece | Rule |
| --- | --- |
| The manifest | A signed JSON document listing the versions, their URLs, their sizes and their checksums |
| The signature | The manifest is signed; the client verifies the signature before trusting a URL |
| The channel | `stable`, `beta`, or an enterprise channel; the policy key `update_channel` selects it |
| The decision | The managed policy `updates_enabled` can forbid every update |
| The download | Over HTTPS, to the URL in the **verified** manifest |
| The verification | The downloaded file's checksum matches the manifest |
| The install | The installer runs with the user's rights, then the app restarts |

Two rules make this safe:

- **The URL comes from the verified manifest, never from the server's
  response body alone.** A manifest that is not signed is a suggestion.
- **The policy wins.** `updates_enabled = false` means no check, no
  download, no update — not an update that is announced and skipped.

## The safe update sequence

Each step must be able to leave the machine in a working state:

1. The client checks the manifest (a small request, on a timer, and only
   when the policy allows it).
2. The client compares the version with its own and decides.
3. The user is asked, unless the policy says the update is silent.
4. The artifact is downloaded to a temporary file and its checksum verified.
5. The installer runs and replaces the application.
6. The app restarts.

The rollback story is the same for every platform: the previous version is kept
for one release cycle, and the update can be undone.

## The enterprise channel

An enterprise that does not want the public channel needs its own manifest URL
(published with the same signature format), its own policy (`update_channel`
names it, `updates_enabled` may be `false`) and a pinned version, so the
fleet does not drift. See the [Admin Guide](admin-guide.md).

## Reproducibility

| Concern | Rule |
| --- | --- |
| The build | The same source and the same toolchain produce the same binary; the Qt version is pinned |
| The dependencies | The Qt runtime is bundled at a fixed version; the SBOM lists everything |
| The secrets | No secret is in the artifact, in the build log or in the environment of a build job |
| The provenance | The build is attributable: the commit, the builder and the timestamp |
| The version | One version string, in the artifact, in the manifest and in the app |

The template's `scripts/leak-guard.sh` is a first step: it fails the tree on a
secret-shaped string, which stops an accidental key from entering the source.

## What is missing, in order of how much it blocks a release

| Missing | Blocks |
| --- | --- |
| An installer per platform | Any distribution |
| Signing and notarisation | Any distribution on Windows and macOS |
| A signed update manifest | Automatic updates |
| An update client | Automatic updates |
| A rollback path | Automatic updates |
| A crash reporter | Supportability, not distribution |

Each is product work with an infrastructure owner; the template gives the version
string, the bundling tools and the policy keys.

## Do / don't

| Do | Don't |
| --- | --- |
| Bundle the Qt runtime | Require the user to install Qt |
| Verify the checksum and the signature | Trust a URL from an unsigned response |
| Let the managed policy forbid updates | Announce an update you will not install |
| Sign every artifact | Ship an unsigned binary "for internal use" |
| Keep the previous version for a rollback | Update in place with no way back |

## See also

- [Support Matrix & Reference Hardware](01-support-matrix-and-reference-hardware.md)
  — what has to be built and tested.
- [Admin Guide](admin-guide.md) — the update policy keys.
- [Testing & CI](12-testing-and-ci.md) — where the packaging job belongs.
