# 09 — Offline & Outbox

## The model

The app is **offline-first**: the local projection is what the user sees, and
the server is reconciled later. Two kinds of data exist:

| Kind | Direction | Owner | Stored where |
| --- | --- | --- | --- |
| **Projection** | Server → app | The server | SQLite ([08](08-storage-and-migrations.md)) |
| **Mutation** | App → server | The app, until the server accepts it | The **outbox** |

A mutation is not a row in a projection table. It is a *pending intent*:
"the user asked for this, the server has not confirmed it yet". It must
survive a crash between "the user clicked" and "the server answered", and
it has a life cycle of its own: queued, in flight, confirmed, rejected.

## The outbox contract

A queued mutation carries at least:

| Field | Why |
| --- | --- |
| A local id | The optimistic row the UI shows needs one |
| The operation | `create`, `update`, `delete` — with the payload |
| The idempotency key | A retry must not create two notes |
| The creation time | To expire an old intent, and to order the queue |
| The attempt count | To back off, and to stop retrying forever |
| The state | `pending`, `in_flight`, `failed`, `rejected` |

The **idempotency key** is the important one: it goes to the server in a
header, and the server returns the same result for the same key — without it,
a retry after a timeout that actually succeeded creates a duplicate.

## The write path

The sequence for a write, with the state the user sees:

| Step | Thread | State |
| --- | --- | --- |
| 1. The user clicks Save | GUI | — |
| 2. The view model validates through the domain | GUI | — |
| 3. The domain rejects an invalid note | GUI | `Error`, message shown |
| 4. The view model appends the pending row to the projection | GUI | `Ready` with the optimistic row |
| 5. The mutation is queued in the outbox | GUI (write) / worker (durable) | — |
| 6. The sync worker drains the queue | Worker | — |
| 7. The server accepts or rejects | Worker | — |
| 8. The result is applied to the projection | Worker writes, GUI redraws | `Ready`, or `Error` on rejection |

Two properties matter: **the user never waits for the network** (step 4 is a
local write), and **a rejected mutation is visible** (the optimistic row is
marked as rejected and the user is told, not silently dropped).

## Conflicts

The server owns the data, so the server's answer wins:

| Case | Rule |
| --- | --- |
| The server changed the same row | The server's version wins; the local pending change is rejected with a conflict |
| The server deleted the row | The local row disappears; the pending change is rejected as not found |
| Two devices | Last write wins on the server, with the client version checked |

The client sends the version it based the change on. The server rejects a
change whose base version is older than the current one, and the app shows
the conflict: silently overwriting another device's work is a data-loss bug.

## What the template ships today

| Piece | State |
| --- | --- |
| The local projection (SQLite) | **Shipped** — `NoteStore` |
| `SdkNoteTransport`, an implementation of the port over the SDK | **Shipped and unit-tested** |
| The SDK client | **Shipped** — `packages/sdk-cpp/` |
| The composition root binding the SDK adapter | **Not wired** — `main.cpp` binds the SQLite adapter |
| The outbox table and the queue | **Not implemented** |
| The sync worker | **Not implemented** |
| The idempotency key and its header | **Not implemented** |
| Conflict detection and resolution | **Not implemented** |
| The realtime envelope and the realtime transport | **Contract only** — `desktop::contracts::RealtimeEnvelope` |

The template ships the **shape**, not the machinery: the port has two
implementations, one local and one remote, and the composition root chooses
between them — the part that is hard to retrofit, and it is done.

> **Known gap — do not copy the shipped write path.** `addNote()` writes
> straight to the projection with no outbox, no idempotency key and no
> server round trip. It is correct for a local-only app and wrong for a
> synced one. The moment a second device exists, this path must change.

## How to build it

The smallest correct sequence, in the order that keeps the app usable at
every step:

1. **Add the outbox table** as a migration ([08](08-storage-and-migrations.md)).
2. **Add the port** for the outbox in the application layer of the slice, and
   the `NoteRepository` methods that write.
3. **Make the port asynchronous** ([07](07-threading-and-the-gui-thread.md)) —
   the outbox write is a disk write.
4. **Add the sync worker** that drains the queue on a timer and on network
   availability.
5. **Add the idempotency key** to the SDK request.
6. **Apply the server's answer** to the projection and settle the outbox row.

## The offline states in the UI

The view model already has the states an offline-first UI needs:

| State | When |
| --- | --- |
| `Loading` | A read is in flight |
| `Ready` | The projection is shown |
| `Empty` | The projection is empty |
| `Offline` | The last operation failed with `ErrorCode::Network` |
| `Forbidden` | The server refused the operation with `PermissionDenied` |
| `NotFound` | The resource is gone |
| `PolicyBlocked` | The managed policy blocked the operation |
| `Error` | Anything else |

`Offline` is a state, not a modal: the user keeps working with the projection.

## Do / don't

| Do | Don't |
| --- | --- |
| Write the intent locally, sync later | Block the UI on the server |
| Send an idempotency key with every mutation | Retry a `POST` blindly |
| Show a rejected mutation to the user | Drop it silently |
| Let the server win a conflict | Overwrite another device's change |
| Keep the projection bounded by the server's data | Grow a local table forever |

## See also

- [Storage & Migrations](08-storage-and-migrations.md) — where the outbox
  table goes.
- [Threading & the GUI Thread](07-threading-and-the-gui-thread.md) — the
  sync worker and the asynchronous port.
- [Data Flows](data-flows.md) — the SDK → HTTP hop.
