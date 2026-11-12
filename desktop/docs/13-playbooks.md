# 13 — Playbooks

Recipes for the changes you will make most. Each one is ordered so that the
tree compiles and the suite is green at every step.

## 1 · Add a slice

The full recipe is in [05 — Vertical Slicing](05-vertical-slicing.md); the
short form:

1. `mkdir -p src/features/<name>/{domain,application,adapters,presentation}/include`
2. Write the domain entity and its invariants; write the failing test first.
3. Declare the port in `application/` — in domain types.
4. Write the use cases against the port.
5. Write the adapter; translate between the domain and the storage.
6. Write the view model (`QObject`, `Q_PROPERTY`, `Q_INVOKABLE`).
7. Add the CMake targets with the link rules of
   [ADR 0002](adr/0002-one-static-library-per-layer.md).
8. Wire the slice in `src/app/main.cpp` — and only there.
9. Register the suite in `tests/CMakeLists.txt` with `desktop_add_test`.

## 2 · Add a field to an entity

| Place | Change |
| --- | --- |
| `contracts/` | The DTO gains the field |
| `domain/` | The entity gains the field and its invariant |
| `infrastructure/` | The column, and **a new migration** |
| `adapters/` | The mapping between the row and the DTO |

The rules: **never edit a shipped migration** ([08](08-storage-and-migrations.md));
the **invariant lives in the domain**, so no column carries a `CHECK` duplicating
a business rule; and a DTO from the server is **re-validated** by `Note::fromDto`.

## 3 · Add a use case

Put the header in `src/features/<slice>/application/include/…`, take the port by
reference in the constructor, return `kernel::Result<T>` (never throw), and
write the test against the **fake** repository.

## 4 · Add an SDK endpoint

`packages/sdk-cpp/` owns the wire. The path is built by the client, never by the
caller; the `Authorization` and `X-Client` headers are set by the client;
**the error envelope wins over the status code**; the status → `ErrorCode`
mapping is one function to extend; and the client returns a `QPromise`.

## 5 · Add a policy key

The policy surface is in `src/contracts/include/desktop/contracts/policy.h`
and has eleven keys: `TenantUrl`, `SsoMode`, `UpdatesEnabled`,
`UpdateChannel`, `RealtimeMode`, `TelemetryLevel`, `CompatibilityMode`,
`KillSwitch`, `FeatureFlags`, `DataRetentionDays`, `AllowUserSettings`.

To add one: add the field to the `Policy` struct with its default; add the key to
`knownPolicyKeys()` and to the parser in `src/contracts/src/policy.cpp`; add
it to `toJson()`; add it to the three sources in the
[Admin Guide](admin-guide.md); add a test in `tests/contracts/`.

An **unknown key, an unknown enum value or a value of the wrong type** makes
`policyFromJson` fail and keeps the defaults. The app never runs with a
half-parsed policy.

## 6 · Add a widget to the ui-kit

`src/ui-kit/` holds the widgets shared by two screens; a widget that belongs to one
screen lives with that screen. The widget takes its data as a model or as
plain values and never calls a use case — the ui-kit does not link the domain
or the application layer. Add the source to `src/ui-kit/CMakeLists.txt` and a
headless test under `tests/`.

## 7 · Wire an unwired component

Several components exist and are tested but are not connected to the composition
root: `HttpTransport`, `SingleInstanceGuard`, `createKeystore`, `NotesClient`
and `SdkNoteTransport`.

To wire one: construct it in `src/app/main.cpp` — the only place allowed to;
inject it into the slice adapter, which implements the port; bind the adapter
to the use case and the use case to the view model; delete the old wiring.

The rule that makes this work: **the port is in the application layer, the
implementation is in the adapter, and only the root may choose between
them.**

## 8 · Make the port asynchronous

The port is synchronous today, and the shipped slice queries SQLite on the GUI
thread ([07](07-threading-and-the-gui-thread.md)). To fix it: change the port to
return `QFuture<kernel::Result<T>>`; move the work to a worker thread inside
the adapter and resolve the future there; keep the use case synchronous in
shape, returning the future; observe it in the view model with
`QFutureWatcher` and set the state when it resolves. The view does not change.

## 9 · Replace a platform backend

The platform layer has one interface per OS capability:

| Interface | Linux | Windows | macOS |
| --- | --- | --- | --- |
| `Keystore` | 0600 file | **stub** | **stub** |
| `AppPaths` | `QStandardPaths` | `QStandardPaths` | `QStandardPaths` |
| `SingleInstanceGuard` | `QLockFile` + `QLocalServer` | same | same |

To replace a backend, implement the interface in `src/platform/src/<os>/` and
return it from `createKeystore()`. The rest of the app never learns which
backend it got — that is what the interface is for. The Windows and macOS
keystore backends are `Unsupported` stubs and must be replaced before release.

## 10 · Ship a release

1. `make ci PRESET=release` — the gate on the release tree.
2. `bash scripts/verify-template.sh` — the template stays instantiable.
3. Bump `DESKTOP_VERSION`; it must be `MAJOR.MINOR.PATCH`.
4. Build the artifact per OS and sign it
   ([11 — Packaging & Updates](11-packaging-and-updates.md)).
5. Run the manual pass on the **floor** profile
   ([01](01-support-matrix-and-reference-hardware.md)): a VM with no GPU, the
   oldest supported OS, and a 1366 × 768 screen.
6. Publish the manifest and the artifact.

## The pre-review checklist

- [ ] The domain owns the rule; the widget does not re-implement it.
- [ ] The port speaks domain types and returns `Result`.
- [ ] The GUI thread does not block ([07](07-threading-and-the-gui-thread.md)).
- [ ] No secret, token or note body reaches a log
      ([10](10-telemetry.md)).
- [ ] A schema change has a **new** migration ([08](08-storage-and-migrations.md)).
- [ ] A new file is covered by the architecture suite.
- [ ] The new behaviour has a test against the fake.
- [ ] `make ci` is green, and `make lint` did not skip.
- [ ] The docs that describe the changed behaviour are updated.

## See also

- [Vertical Slicing](05-vertical-slicing.md)
- [Repository Layout & Project Skeleton](06-repository-layout-and-project-skeleton.md)
- [Playbook Desktop Qt §20](../PLAYBOOK_DESKTOP_QT.md) — the checklists of
  the playbook itself.
