# 14 — Glossary

The terms used in this set, in one place. The link goes to the chapter that
explains it.

## Architecture

| Term | Meaning |
| --- | --- |
| **Hexagonal architecture** | The domain sits at the centre; everything else depends on it through interfaces (ports). See [04](04-hexagonal-layers-and-mvvm.md) |
| **Port** | An interface the application layer declares for what it needs from the outside. Owned by the consumer, implemented by an adapter |
| **Adapter** | The implementation of a port: the SQLite repository, the SDK transport |
| **Layer** | A ring of the hexagon: domain, application, adapters, presentation, platform, infrastructure |
| **Vertical slice** | One capability cut through every layer, instead of one layer holding every capability. See [05](05-vertical-slicing.md) |
| **Composition root** | The place where the concrete parts are chosen and connected: `src/app/main.cpp` |
| **Domain** | The rules of the business, with no framework, no database and no operating system |
| **Use case** | One thing the application can do, expressed in domain terms |
| **DTO** | A data transfer object: the wire shape of a thing. `desktop::contracts::NoteDto` |
| **Contract** | A shared shape that two sides agree on: the DTOs, the error envelope, the policy schema |
| **MVVM** | Model–View–ViewModel: the state lives in a view model, the widget renders it. See [04](04-hexagonal-layers-and-mvvm.md) |
| **View model** | A `QObject` that holds the state of a screen, exposes it as `Q_PROPERTY` and takes the commands as `Q_INVOKABLE` |
| **Item model** | A `QAbstractItemModel` / `QAbstractListModel` that feeds a view |
| **Kernel** | `src/shared-kernel/`: `Result`, `ErrorCode`, `StrongId`, `Timestamp` — what every layer needs |
| **ui-kit** | `src/ui-kit/`: the shared widgets and the theme. Knows no domain |
| **Infrastructure** | The adapters that touch the world: storage, HTTP, telemetry |
| **Platform layer** | The only code that knows the operating system |

## Errors and results

| Term | Meaning |
| --- | --- |
| **`Result<T>`** | A value or an error, as a `std::variant`. `Result<void>` carries only the outcome |
| **ErrorCode** | The closed enum of failures. Each code has a wire name |
| **Error envelope** | The wire shape of a failure: `{code, detail, requestId, requiredPermission}` |
| **`[[nodiscard]]`** | The caller must use the result; ignoring it is a compile-time smell |
| **Trust boundary** | The place where data that was not produced by the app enters it. See [Data Flows](data-flows.md) |

## Threading

| Term | Meaning |
| --- | --- |
| **GUI thread** | The thread that owns the windows. It paints and dispatches events, and nothing else. See [07](07-threading-and-the-gui-thread.md) |
| **Worker thread** | A thread that does the blocking work and returns the result |
| **`QFuture` / `QPromise`** | Qt's asynchronous value: a future you can wait for, a promise you can resolve |
| **`QFutureWatcher`** | The `QObject` that turns a future into signals |
| **Blocking call** | A call that does not return until the work is done — a disk query, a network wait, a lock |
| **`QThread`** | Qt's thread class. `moveToThread` changes an object's affinity |
| **`processEvents()`** | Re-entering the event loop from a slot. Never a fix for a slow operation |

## Storage

| Term | Meaning |
| --- | --- |
| **Projection** | The local copy of what the server said. A cache, never the truth. See [08](08-storage-and-migrations.md) |
| **Outbox** | The queue of mutations the user made that the server has not accepted yet. See [09](09-offline-and-outbox.md) |
| **Idempotency key** | A key that lets the server recognise a repeated request and answer it the same way |
| **WAL** | Write-Ahead Logging: SQLite's journal mode where readers do not block the writer |
| **`PRAGMA user_version`** | SQLite's own integer, used by the template as the schema version |
| **Migration** | A numbered, forward-only schema change. Never edited after it ships |
| **`QStandardPaths`** | The Qt API that resolves the conventional per-user directories |
| **Keystore** | The OS-backed store for the refresh token. See [07](07-threading-and-the-gui-thread.md) |
| **Single instance** | One running process per user, enforced by a lock file plus a local socket |

## Platform and delivery

| Term | Meaning |
| --- | --- |
| **Compatibility Mode** | The software-rendering mode the app falls back to. See [03](03-no-gpu-fallback.md) |
| **Safe mode** | A minimal start: no sync, no telemetry, no update check |
| **Widgets-first** | The rule that the UI is `QWidget`, and QML is an exception that needs an ADR. See [02](02-widgets-versus-qt-quick.md) |
| **Managed policy** | The configuration an administrator pushes to the app. See [Admin Guide](admin-guide.md) |
| **Policy source** | Where a managed policy comes from: the Windows registry, the macOS managed preferences, the Linux policy file |
| **ADMX** | The Windows administrative template that projects registry keys into the Group Policy editor |
| **Update channel** | The stream of releases an app follows: `stable`, `beta`, `canary`, or an enterprise one |
| **Notarisation** | The macOS process by which Apple checks a signed app before it runs on a current macOS |
| **Authenticode** | The Windows code-signing scheme |
| **Floor profile** | The oldest, slowest machine the app must support. See [01](01-support-matrix-and-reference-hardware.md) |
| **Budget** | A measured limit: start time, input latency, memory, installer size |

## The build

| Term | Meaning |
| --- | --- |
| **Preset** | A named CMake configuration: `dev` (Debug, tests on) and `release` (Release, tests off) |
| **`desktop_add_test`** | The helper that registers a suite, links `Qt6::Test` and runs it headless |
| **`QT_QPA_PLATFORM=offscreen`** | The environment variable that lets a widget test run without a display server |
| **AUTOMOC** | CMake's automatic `moc` invocation for `Q_OBJECT` classes. On in the root `CMakeLists.txt` |
| **Compile definitions** | The `-D` flags baked into a target, such as `DESKTOP_VERSION` |
| **`desktop_assert_no_forbidden_include`** | The configure-time guard that fails the build on a forbidden include |
| **Link rule** | The `target_link_libraries` rule that makes a dependency violation a link error. See [ADR 0002](adr/0002-one-static-library-per-layer.md) |
| **Placeholder** | The literal tokens the template ships with, substituted by `scripts/rename.sh` |
| **Leak guard** | The script that fails the tree on a placeholder that survived the rename, or on a secret-shaped string |
| **Evals** | The harness that validates the `.claude` layer and the rename state of the tree |

## See also

- [Overview & Principles](00-overview-and-principles.md) — the one rule and
  the chapter map.
- [Data Flows](data-flows.md) — the hops, the threads, the boundaries.
- [Admin Guide](admin-guide.md) — the managed policy in practice.
- [Playbook Desktop Qt](../PLAYBOOK_DESKTOP_QT.md) — the source of the
  decisions and the vocabulary.
