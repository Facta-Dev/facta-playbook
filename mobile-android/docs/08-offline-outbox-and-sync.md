# Offline, Outbox & Sync

An offline-first app has two kinds of data, and confusing them is the
classic bug:

| Kind | Direction | Owner | Stored where |
| --- | --- | --- | --- |
| **Projection** | Server → app | The server | The local database (Room) |
| **Mutation** | App → server | The app, until the server accepts it | The **outbox** |

A mutation is not a row in a projection table. It is a *pending intent*: "the
user asked for this, the server has not confirmed it yet". It must survive a
crash between the tap and the answer, and it has a life cycle of its own.

> **Known gap — do not copy a fiction.** The template **ships no
> persistence at all**. `:feature:notes:data` contains an **in-memory**
> adapter: a map in a class. There is no Room database, no outbox table, no
> `WorkManager` job and no idempotency key in the tree. This chapter
> specifies the contract a project must implement and the order in which to
> build it.

## 1. The projection

The local database is a **projection of server data**, never the truth.

| Concern | Choice |
| --- | --- |
| Storage | **Room** (SQLite in WAL mode) with `Flow` queries |
| Schema | Exported (`room.schemaLocation`) so the migration test has a fixture |
| Migrations | **Forward-only**, with a test from every released version |
| Destructive fallback | **Forbidden** — it destroys the user's pending mutations |
| Reads | `stale-while-revalidate`: show the projection, refresh in the background |

The rule that keeps the projection honest: **it is bounded by the server's
data**. A projection table that grows without bound is a local database
pretending to be a client.

### Room, concretely

| Piece | What it is |
| --- | --- |
| The DAO | `Flow`-returning queries; a suspend function for a write |
| The entity | The **local** shape, not the DTO: it carries the sync metadata |
| The database | One per app, created once in `:app` |
| The migration | A `Migration` object, registered in the builder, tested with the previous schema |

The adapter in `:feature:notes:data` is where Room lives. The domain never
sees a `RoomDatabase`, a DAO or an entity: it sees the port and the domain
types.

## 2. The outbox contract

A queued mutation carries at least:

| Field | Why |
| --- | --- |
| A local id | The optimistic row the UI shows needs one |
| The operation | `create`, `update`, `delete` — with the payload |
| The **idempotency key** | A retry must not create two notes |
| The creation time | To expire an old intent, and to order the queue |
| The attempt count | To back off, and to stop retrying forever |
| The state | `pending`, `in_flight`, `failed`, `rejected` |
| The entity id it touches | Per-entity ordering: two updates to the same entity must not race |

The **idempotency key** is the important one. It travels in an
`Idempotency-Key` header, and the server returns the same result for the
same key. Without it, a retry after a timeout that actually succeeded
creates a duplicate — the single most common data bug in an offline-first
app.

Per-entity ordering matters as much: the outbox drains the queue, and two
mutations to the same entity must be applied in the order the user made
them, or the second one is computed against a stale base version.

## 3. The write path

The sequence for a write, with the state the user sees:

| Step | Where it runs | State the user sees |
| --- | --- | --- |
| 1. The user commits the change | Main thread | — |
| 2. The domain validates it | Main thread | — |
| 3. The domain rejects it | Main thread | `Error`, the reason shown |
| 4. The optimistic row is written to the projection | `io` dispatcher | `Ready` with the optimistic row |
| 5. The mutation is queued in the outbox | `io` dispatcher | — |
| 6. The sync worker drains the queue | WorkManager | — |
| 7. The server accepts or rejects | `io` dispatcher | — |
| 8. The result is applied to the projection | `io` dispatcher | `Ready`, or `Error` on rejection |

Two properties matter, and both are testable:

| Property | Test |
| --- | --- |
| **The user never waits for the network** | Step 4 completes with the network down |
| **A rejected mutation is visible** | The optimistic row is marked rejected and the user is told — not silently dropped |

The optimistic row is what makes the app feel fast; the outbox is what makes
it correct. A write path with neither is a local-only app.

## 4. WorkManager

**WorkManager** is the only scheduler that survives Doze and the standby
buckets ([01](01-platform-model-and-android-runtime.md)). A timer, a service
or a coroutine loop will not.

| Property | The outbox job |
| --- | --- |
| Trigger | `ExistingWorkPolicy.KEEP` on a unique name — **one** drain at a time |
| Constraints | `NetworkType.CONNECTED` |
| Backoff | Exponential, with a floor |
| Bounded | `setMaximumRetries` — a permanently failing mutation is surfaced, not retried forever |
| Periodic | A second, periodic job reconciles the projection |
| Foreground | Only for a long upload, and only with a notification |

The worker is **deferrable work**, not a guarantee of immediate delivery.
The app reconciles on foreground as well, because a device that was in Doze
for six hours has a six-hour-old projection.

## 5. Conflicts

The server owns the data, so the server's answer wins — but the user must be
told.

| Case | Rule |
| --- | --- |
| The server changed the same row | The server's version wins; the local pending change is **rejected with a conflict** |
| The server deleted the row | The local row disappears; the pending change is rejected as not found |
| Two devices | Last write wins on the server, with the client version checked |

The client sends the **version it based the change on**. The server rejects
a change whose base version is older than the current one. Silently
overwriting another device's work is a data-loss bug, and the conflict must
reach the user as a typed state — not as a failed write with no
explanation.

## 6. The offline states in the UI

The `UiStatus` set
([03](03-layers-modules-and-udf.md)) already carries what an offline-first
UI needs:

| State | When |
| --- | --- |
| `Loading` | A read is in flight |
| `Ready` | The projection is shown |
| `Empty` | The projection is empty |
| `Offline` | The last operation failed because the device has no usable connection |
| `Forbidden` | The server refused the operation |
| `NotFound` | The resource is gone |
| `Error` | Anything else, with a `correlationId` |
| `PolicyBlocked` | A managed policy blocked the operation |

`Offline` is a **state, not a modal**: the user keeps working with the
projection. A modal that blocks the screen until the network returns turns
an offline-first app into an offline app.

## 7. The adoption path

The smallest correct sequence, in the order that keeps the app usable at
every step:

1. **Add Room to `:feature:notes:data`.** One entity, one DAO, one database
   in the module. Export the schema.
2. **Add the migration test.** A test with the previous schema as the
   fixture. A destructive fallback is forbidden.
3. **Add the outbox table** as a second entity in the same database.
4. **Add the port** for the outbox in the slice's `domain`, and the
   repository methods that write.
5. **Extend the repository implementation** to write the projection and
   enqueue the mutation — **in one transaction**, so a crash between the two
   cannot lose the intent.
6. **Add the WorkManager job** that drains the queue, with the constraints
   and the backoff of §4.
7. **Add the idempotency key** to the SDK request.
8. **Apply the server's answer** to the projection and settle the outbox
   row.
9. **Add the conflict state** to the screen: a rejected mutation is visible.

## 8. What the template ships today

| Piece | State |
| --- | --- |
| The repository port | **Shipped** — `NoteRepository` in `:feature:notes:domain` |
| The in-memory adapter | **Shipped** — `:feature:notes:data` |
| The optimistic write path | **Not implemented** — there is no write path at all |
| The Room projection | **Not implemented** |
| The outbox table and the queue | **Not implemented** |
| The `WorkManager` job | **Not implemented** |
| The idempotency key and its header | **Not implemented** |
| Conflict detection and resolution | **Not implemented** |
| The realtime envelope | **Not implemented** — out of scope for this template |

The template ships the **port**, and that is the part that is expensive to
retrofit: the port is the seam the adapter plugs into, and the domain does
not change when the in-memory adapter is replaced by Room.

## Do / don't

| Do | Don't |
| --- | --- |
| Write the intent locally, sync later | Block the UI on the server |
| Send an idempotency key with every mutation | Retry a `POST` blindly |
| Show a rejected mutation to the user | Drop it silently |
| Let the server win a conflict | Overwrite another device's change |
| Use WorkManager for the drain | Poll with a timer in a service |
| Keep the projection bounded by the server's data | Grow a local table forever |
| Test the migration from the previous schema | Rely on the destructive fallback |

## See also

- [Layers, Modules & UDF](03-layers-modules-and-udf.md) — where the port
  lives.
- [Platform Model & Android Runtime](01-platform-model-and-android-runtime.md)
  — Doze, standby buckets and `onTrimMemory`.
- [Security & Keystore](09-security-and-keystore.md) — encrypting the
  sensitive fields of the projection.
- [Playbook Native Android Kotlin §§4.5, 14](../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md)
  — the state table and the offline rules.
