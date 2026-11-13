# Offline, Outbox & Sync

An offline-first app has two kinds of data, and confusing them is the
classic bug:

| Kind | Direction | Owner | Stored where |
| --- | --- | --- | --- |
| **Projection** | Server → app | The server | The local store (SwiftData) |
| **Mutation** | App → server | The app, until the server accepts it | The **outbox** |

A mutation is not a row in a projection table. It is a *pending intent*: "the
user asked for this, the server has not confirmed it yet". It must survive a
crash between the tap and the answer, and it has a life cycle of its own.

> **Known gap — do not copy a fiction.** The template **ships no persistence
> at all**. `InMemoryNoteRepository` in `Packages/Feature/Notes/Data` returns
> two fixed notes and writes nothing: no SwiftData container, no outbox
> table, no `BGTaskScheduler` job, no idempotency key and no write method on
> the port. This chapter specifies the contract a project must implement and
> the order in which to build it.

## 1. The projection

The local store is a **projection of server data**, never the truth.

| Concern | Choice |
| --- | --- |
| Storage | **SwiftData** — a `ModelContainer` opened once, a `ModelContext` per unit of work |
| Schema | **Code-declared `@Model` classes**, not an `.xcdatamodeld` |
| Migrations | **Versioned and forward-only**, with a test from every released version |
| Destructive fallback | **Forbidden** — it destroys the user's pending mutations |
| Reads | `stale-while-revalidate`: show the projection, refresh in the background |

The rule that keeps the projection honest: **it is bounded by the server's
data**. A projection table that grows without bound is a local store
pretending to be a client.

### SwiftData, concretely

| Piece | What it is |
| --- | --- |
| The `@Model` type | The **local** shape, not the DTO: it carries the sync metadata |
| The `ModelContainer` | One per app, created once in the composition root |
| The `ModelContext` | One per unit of work; the transaction boundary |
| The schema | The `@Model` classes themselves — `scripts/check-architecture.sh` fails the build on an `.xcdatamodeld` |

The adapter in `Packages/Feature/Notes/Data` is where SwiftData lives. The
domain never sees a `ModelContainer`, a `ModelContext` or a `@Model` type:
it sees the port and the domain types.

Two iOS-specific notes:

- **The Core Data model editor is excluded.** `scripts/check-architecture.sh`
  rejects `*.xcdatamodeld` as a visual-builder artefact, and
  `PLAYBOOK_MOBILE_IOS_SWIFT.md` D3 excludes it as a source of truth. SwiftData
  is fine, because its schema is code.
- **SwiftData is a proposal, not a mandate.** The iOS playbook's D10 names
  **GRDB** for an offline cache and an outbox, and keeps SwiftData for simple
  local state; the dependency register lists the choice as open
  ([Dependency Register](dependency-register.md)). The port below is the same
  either way: a project that picks GRDB implements the same adapter against
  the same protocol.

## 2. The outbox contract

A queued mutation carries at least:

| Field | Why |
| --- | --- |
| A local id | The optimistic row the UI shows needs one |
| The operation | `create`, `update`, `delete` — with the payload |
| The **idempotency key** | A retry must not create two notes |
| The creation time | To expire an old intent, and to order the queue |
| The attempt count | To back off, and to stop retrying forever |
| The state | `pending`, `inFlight`, `failed`, `rejected` |
| The entity id it touches | Per-entity ordering: two updates to the same entity must not race |

The **idempotency key** is the important one. It travels with the request —
`Idempotency-Key` — and the server returns the same result for the same
key. Without it, a retry after a timeout that actually succeeded creates a
duplicate: the single most common data bug in an offline-first app.

Per-entity ordering matters as much: the outbox drains the queue, and two
mutations to the same entity must be applied in the order the user made
them, or the second one is computed against a stale base version.

## 3. The write path

The sequence for a write, with the state the user sees:

| Step | Where it runs | State the user sees |
| --- | --- | --- |
| 1. The user commits the change | Main actor | — |
| 2. The domain validates it | Main actor | — |
| 3. The domain rejects it | Main actor | `Error`, the reason shown |
| 4. The optimistic row is written to the projection | Off the main actor | `Ready` with the optimistic row |
| 5. The mutation is queued in the outbox | Off the main actor, **same transaction** | — |
| 6. The sync job drains the queue | `BGTaskScheduler` or foreground | — |
| 7. The server accepts or rejects | Off the main actor | — |
| 8. The result is applied to the projection | Off the main actor | `Ready`, or `Error` on rejection |

Steps 4 and 5 are **one `ModelContext` transaction**. A crash between them
loses the intent, and an intent that is lost is a write the user believes
happened and the server never heard about.

Two properties matter, and both are testable:

| Property | Test |
| --- | --- |
| **The user never waits for the network** | Step 4 completes with the network down |
| **A rejected mutation is visible** | The optimistic row is marked rejected and the user is told — not silently dropped |

The optimistic row is what makes the app feel fast; the outbox is what makes
it correct. A write path with neither is a local-only app.

### The port grows a write method

The shipped port is read-only:

```swift
// The port as it ships today: one read operation, no write.
public protocol NoteRepository: Sendable {
    func load() async throws -> [Note]
}
```

The write path adds the operations the use case calls — an enqueue, a
settle, a cancel — as **new methods on the same protocol**, next to the use
case that needs them. The protocol stays `Sendable`, the methods stay
`async throws`, and the domain stays free of SwiftData.

## 4. Background execution

**iOS gives no guaranteed background execution.** There is no counterpart to
Android's `WorkManager` guarantee: a background task is a *request*, and the
system decides when — or whether — to grant it. A design that assumes
otherwise is a design that loses writes.

| Property | The outbox job |
| --- | --- |
| Scheduler | `BGTaskScheduler` — `BGAppRefreshTask` for a periodic drain, `BGProcessingTask` for work that needs network and power |
| Registration | The identifiers are declared in the `Info.plist` (`BGTaskSchedulerPermittedIdentifiers`) and the handler is registered at launch |
| Constraint | `requiresNetworkConnectivity`, `requiresExternalPower` |
| Backoff | Exponential, with a floor |
| Bounded | A retry ceiling — a permanently failing mutation is surfaced, not retried forever |
| Upload | A **background `URLSession`** for a payload too large for a task's time budget |
| Foreground | The app also drains the outbox when it comes to the foreground |

Two consequences that differ from Android and must be designed for:

| Consequence | Why |
| --- | --- |
| The `Info.plist` must become a **real file** | `UIBackgroundModes` and `BGTaskSchedulerPermittedIdentifiers` are dictionaries and arrays, not scalar build settings: they do not fit the generated-`Info.plist` model the template uses today |
| A `BGAppRefreshTask` is **not** a timer | The system schedules it by usage pattern; a task that asks too often is scheduled less often, and a device that never opens the app may never run it |

The outbox drain is therefore **deferrable work, never a delivery
guarantee**. The app reconciles on foreground as well, because a device that
was suspended for six hours has a six-hour-old projection — and because the
outbox may not have drained in that window.

> **Known gap.** The template declares **no background mode**, no
> `BGTaskSchedulerPermittedIdentifiers` key and no task handler. The
> `Info.plist` is generated (`GENERATE_INFOPLIST_FILE: YES` in `project.yml`)
> and the only keys are the display name, the launch screen and the scene
> manifest.

## 5. Conflicts

The server owns the data, so the server's answer wins — but the user must be
told.

| Case | Rule |
| --- | --- |
| The server changed the same row | The server's version wins; the local pending change is **rejected with a conflict** |
| The server deleted the row | The local row disappears; the pending change is rejected as not found |
| Two devices | Last write wins on the server, with the client version checked |

The client sends the **version it based the change on**. The server rejects
a change whose base version is older than the current one — a `409` that the
client maps to a typed outcome. Silently overwriting another device's work
is a data-loss bug, and the conflict must reach the user as a typed state —
not as a failed write with no explanation.

The vocabulary already exists in `Kernel`: `ErrorCode` has a `conflict`
case. What does **not** exist yet is a `UiStatus` for it — `UiStatus` has
eight cases and none of them is `conflict`, so `NotesModel` currently maps
`.conflict` into `.error(correlationID:)`. A project that surfaces conflicts
distinctly either adds a `UiStatus` case — a published-contract change, which
`AGENTS.md` says to announce to the dependents — or carries the pending
state in `NotesViewState` beside the notes.

## 6. The offline states in the UI

The `UiStatus` set already carries what an offline-first UI needs:

| State | When |
| --- | --- |
| `loading` | A read is in flight |
| `ready` | The projection is shown |
| `empty` | The projection is empty |
| `offline` | The last operation failed because the device has no usable connection |
| `forbidden` | The server refused the operation |
| `notFound` | The resource is gone |
| `error(correlationID:)` | Anything else, with a correlation id |
| `policyBlocked` | A managed policy blocked the operation |

`offline` is a **state, not a modal**: the user keeps working with the
projection. A modal that blocks the screen until the network returns turns
an offline-first app into an offline app.

## 7. The adoption path

The smallest correct sequence, in the order that keeps the app usable at
every step:

1. **Add SwiftData to `Packages/Feature/Notes/Data`.** One `@Model` type,
   one `ModelContainer` built in `AppEnvironment.live`, one `ModelContext`
   per unit of work. No `.xcdatamodeld`.
2. **Add the migration test.** A test with the previous schema as the
   fixture. A destructive fallback is forbidden.
3. **Add the outbox table** as a second `@Model` type in the same container.
4. **Add the port methods** for the outbox in the slice's `Domain`, next to
   the use case that needs them.
5. **Extend the repository implementation** to write the projection and
   enqueue the mutation — **in one transaction**, so a crash between the two
   cannot lose the intent.
6. **Add the `BGTaskScheduler` job** that drains the queue, with the
   constraints and the backoff of §4, plus the `Info.plist` keys it needs.
7. **Add the idempotency key** to the SDK request.
8. **Apply the server's answer** to the projection and settle the outbox
   row.
9. **Add the conflict state** to the screen: a rejected mutation is visible.

## 8. What the template ships today

| Piece | State |
| --- | --- |
| The repository port | **Shipped** — `NoteRepository` in `Packages/Feature/Notes/Domain` (read-only) |
| The in-memory adapter | **Shipped** — `InMemoryNoteRepository` in `Packages/Feature/Notes/Data` |
| The optimistic write path | **Not implemented** — there is no write path at all |
| The SwiftData projection | **Not implemented** — no `@Model`, no container |
| The outbox table and the queue | **Not implemented** |
| The `BGTaskScheduler` job | **Not implemented** |
| The idempotency key and its header | **Not implemented** |
| Conflict detection and resolution | **Not implemented** — `ErrorCode.conflict` exists, the resolution does not |
| The realtime envelope | **Not implemented** — out of scope for this template |

The template ships the **port**, and that is the part that is expensive to
retrofit: the port is the seam the adapter plugs into, and the domain does
not change when the in-memory adapter is replaced by a SwiftData one.

## Do / don't

| Do | Don't |
| --- | --- |
| Write the intent locally, sync later | Block the UI on the server |
| Send an idempotency key with every mutation | Retry a `POST` blindly |
| Show a rejected mutation to the user | Drop it silently |
| Let the server win a conflict | Overwrite another device's change |
| Use `BGTaskScheduler` for the drain | Poll with a timer on a background queue |
| Reconcile on foreground as well | Treat a background task as a guarantee |
| Keep the projection bounded by the server's data | Grow a local table forever |
| Test the migration from the previous schema | Rely on the destructive fallback |

## See also

- [Packages, Layers & UDF](03-packages-layers-and-udf.md) — where the port
  lives.
- [Platform Model & iOS Runtime](01-platform-model-and-ios-runtime.md) —
  the suspension model and the memory pressure that ends a background task.
- [Security & Keychain](09-security-and-keychain.md) — encrypting the
  sensitive fields of the projection.
- [Auth & Platform Features](07-auth-and-platform-features.md) — the
  session that the sync call authenticates with.
- [Playbook Native iOS Swift §§4.5, 14](../PLAYBOOK_MOBILE_IOS_SWIFT.md)
  — the offline rules and the background behaviour.
