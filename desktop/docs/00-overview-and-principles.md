# 00 — Overview & Principles

## What this template is

A starting point for a **native Qt 6 desktop client**: C++20, Qt Widgets,
CMake + Ninja, a hexagonal core, one static library per layer and a
composition root that wires everything. It ships a worked slice (`notes`) end
to end — domain, use cases, port, SQLite adapter, SDK adapter, view model,
window — so that the shape of a feature is visible before a line of product
code is written.

It is a **template**, not a product: `scripts/rename.sh` turns it into a
project, and the seams that a product needs but a template cannot decide for
you (the real keystore, the outbox, the updater) are documented as gaps
rather than faked.

## The one rule

**Dependencies point inward. The GUI thread never blocks.**

```
presentation ─▶ application ─▶ domain
                    │            ▲
                    ▼            │
                  port ──────────┘
                    ▲
                    │ implemented by
              adapters (SQLite, SDK)
```

Two directions only: **inward** (the presentation layer may know the
application layer, never the reverse) and **outward through the composition
root** (only `src/app/main.cpp` may construct a concrete adapter). The
application layer names a port; the adapter implements it; the root binds
them. Nothing else may.

The rule is not a review guideline. It is enforced three times over
([04 — Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md)):

| Guard | When it fires | What it sees |
| --- | --- | --- |
| `desktop_assert_no_forbidden_include` | CMake configure | A forbidden `#include` in a layer's sources |
| `target_link_libraries` rules | Link time | A symbol the layer was not allowed to see |
| `tests/_platform/architecture` | Test time | A source file that breaks the rules again |

## Principles

1. **Widgets first.** The UI is `QWidget` and `QAbstractItemModel`. Qt Quick
   is an escalation, not a default ([02](02-widgets-versus-qt-quick.md)).
2. **The GUI thread is a scarce resource.** No disk, no network wait, no
   sleep, no `processEvents()` on the thread that owns the windows
   ([07](07-threading-and-the-gui-thread.md)).
3. **Errors are values.** `Result<T>` travels across every layer boundary;
   no exceptions cross a port, no error codes are guessed by the caller
   ([04](04-hexagonal-layers-and-mvvm.md)).
4. **The domain owns the rules.** Length bounds, markup rejection, trimming
   live in `domain::Note`; the UI never validates a business rule twice.
5. **The port is owned by the consumer.** The application layer declares the
   interface it needs; the adapter implements it. The interface lives with
   the use case, not with the database ([05](05-vertical-slicing.md)).
6. **Storage is a projection, not the truth.** SQLite caches what the server
   said; a mutation the server has not accepted yet lives in an outbox
   ([08](08-storage-and-migrations.md), [09](09-offline-and-outbox.md)).
7. **Migrations are forward-only and versioned.** `PRAGMA user_version`, one
   migration per change, no down path ([08](08-storage-and-migrations.md)).
8. **The platform layer is the only OS-aware code.** Paths, single instance,
   keystore, notifications — nothing else touches the operating system.
9. **Nothing sensitive is logged or persisted in the clear.** The logger
   redacts bearer tokens and secret-shaped keys before they reach a file
   ([10](10-telemetry.md)).
10. **A template states its gaps.** Every unfinished seam is named, with the
    file that must change. A gap you have to discover by reading code is a
    defect of the template.

## The chapter map

| Chapter | Read it when |
| --- | --- |
| [01 — Support Matrix & Reference Hardware](01-support-matrix-and-reference-hardware.md) | You need to know what the app runs on, and what "supported" costs |
| [02 — Widgets versus Qt Quick](02-widgets-versus-qt-quick.md) | Someone proposes a QML screen |
| [03 — The No-GPU Fallback](03-no-gpu-fallback.md) | The app starts black on a VM or a remote desktop |
| [04 — Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md) | You add a layer or a view model |
| [05 — Vertical Slicing](05-vertical-slicing.md) | You add a feature |
| [06 — Repository Layout & Project Skeleton](06-repository-layout-and-project-skeleton.md) | You start the project from the template |
| [07 — Threading & the GUI Thread](07-threading-and-the-gui-thread.md) | You write anything that touches disk or network |
| [08 — Storage & Migrations](08-storage-and-migrations.md) | You change the schema |
| [09 — Offline & Outbox](09-offline-and-outbox.md) | You add a write path |
| [10 — Telemetry](10-telemetry.md) | You add a log line or a metric |
| [11 — Packaging & Updates](11-packaging-and-updates.md) | You ship a build |
| [12 — Testing & CI](12-testing-and-ci.md) | You add a test or wire a pipeline |
| [13 — Playbooks](13-playbooks.md) | You need the recipe for a common change |
| [14 — Glossary](14-glossary.md) | A word in this set is unfamiliar |

Reference material: [Support Matrix](support-matrix.md),
[Data Flows](data-flows.md), [Admin Guide](admin-guide.md),
[ADR 0001](adr/0001-qt-widgets-over-qt-quick.md),
[ADR 0002](adr/0002-one-static-library-per-layer.md).

## Do / don't

| Do | Don't |
| --- | --- |
| Put the rule in the domain, in one place | Validate the same rule in the widget and the use case |
| Return `Result<T>` from a port | Throw an exception across a port boundary |
| Construct adapters in `main.cpp` | Construct a `QSqlDatabase` inside a use case |
| Ask for a `QFuture` when the work can block | Call a blocking API from a slot |
| Name a gap in the code and the docs | Leave a `TODO` in a shipped template |

## Known gaps of the template

These are real, verified against the source, and are **not** defects to fix
blindly — each needs a product decision.

| Gap | Where it lives | Chapter |
| --- | --- | --- |
| The repository port is **synchronous**; the shipped slice queries SQLite on the GUI thread | `src/features/notes/application/include/desktop/notes/application/note_repository.h` | [07](07-threading-and-the-gui-thread.md) |
| The Windows and macOS **keystore backends are `Unsupported` stubs**; only the Linux 0600-file backend works | `src/platform/src/{linux,windows,macos}/keystore_*.cpp` | [07](07-threading-and-the-gui-thread.md) |
| **No outbox**: writes go straight to the repository, there is no pending-mutation queue | `src/features/notes/` | [09](09-offline-and-outbox.md) |
| **No policy reader**: the policy schema exists in `src/contracts/`, nothing reads it from the OS | `src/contracts/` | [Admin Guide](admin-guide.md) |
| **No packaging**: no installer, no signing, no updater, no manifest | — | [11](11-packaging-and-updates.md) |
| The **SDK owns its own `QNetworkAccessManager`** and does not use `HttpTransport`, so retries live in two places | `packages/sdk-cpp/src/notes_client.cpp`, `src/infrastructure/src/http/http_transport.cpp` | [10](10-telemetry.md) |
| **No remote telemetry sink**, no crash reporting, no metrics pipeline | `src/infrastructure/src/telemetry/` | [10](10-telemetry.md) |
| The **ui-kit ships one light stylesheet**, which overrides the native look; no dark or high-contrast theme | `src/ui-kit/src/theme.cpp` | [02](02-widgets-versus-qt-quick.md) |

## See also

- [Playbook Desktop Qt](../PLAYBOOK_DESKTOP_QT.md) — the decisions this
  template implements (D1–D15).
- [ADR 0001 — Qt Widgets over Qt Quick](adr/0001-qt-widgets-over-qt-quick.md)
- [ADR 0002 — One static library per layer](adr/0002-one-static-library-per-layer.md)
