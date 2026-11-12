# 07 — Threading & the GUI Thread

## The rules

1. **The GUI thread paints and dispatches events. Nothing else.** A slot that
   touches the disk, the network or a lock blocks the window, the input
   queue and the repaint.
2. **Never call `QCoreApplication::processEvents()`** to "unstick" a slow
   operation. It re-enters the event loop and turns a latency bug into a
   re-entrancy bug.
3. **One `QNetworkAccessManager` per thread.** It is not thread-safe.
4. **One `QSqlDatabase` connection per thread.** A connection is created and
   used in the same thread, and is closed in that thread.
5. **A worker thread never touches a widget.** It returns a value or emits a
   signal; the GUI thread applies it.
6. **Cross-thread communication is by signal, queued** — never by a raw
   pointer to a `QObject` another thread owns.

## What the template does today

| Thing | Thread | Correct? |
| --- | --- | --- |
| `QApplication` and the windows | GUI (main) | Yes |
| `NotesViewModel`, `NotesListModel` | GUI | Yes — a model must live where the view reads it |
| `HttpTransport` and its `QNetworkAccessManager` | The thread that creates it | Yes — the callback runs on the transport's thread |
| `NotesClient` in the SDK | The thread that creates it | Yes |
| **`NoteStore::open` and the migrations** | **GUI** | **Blocking, at startup** |
| **`ListNotes::execute` → `QSqlQuery`** | **GUI** | **Blocking, on every refresh** |

The last two are the template's known deviation, and they are the most
important thing in this chapter.

## The blocking call, exactly

`NotesViewModel::refresh()` is a `Q_INVOKABLE` slot. It calls the use case,
which calls the port, which runs a `QSqlQuery`:

```cpp
void NotesViewModel::refresh() {
    setState(State::Loading, QStringLiteral("Loading the notes"));
    const kernel::Result<QVector<domain::Note>> listed = listNotes_.execute();
    …
}
```

`SqliteNoteRepository::list()` → `NoteStore::listNotes()` →
`QSqlQuery::exec()`, all on the calling thread, which is the GUI thread.
The same is true for `addNote()`, which calls `upsertNote()` and then
`refresh()` — two blocking queries in one click.

At the scale of the template's sample data this is invisible. At 10 000
rows on a hard disk it is a visible freeze, and it is exactly the case
[01 — Support Matrix](01-support-matrix-and-reference-hardware.md) says the
app must survive.

> **Known gap — do not copy this pattern.** The port is synchronous:
> `kernel::Result<QVector<domain::Note>> list() const`. The fix is in
> [the port's signature](#the-fix), not in the widget.

### Why the template ships it anyway

Because the shape it demonstrates is the shape you need: the use case does
not know whether the data comes from SQLite or from the server, the adapter
translates rows into domain objects, and the view model translates the
result into a state. Making the port asynchronous changes none of those; it
changes one type in one header.

## The fix

Make the port asynchronous, and let the use cases stay synchronous in shape
by returning a future:

```cpp
[[nodiscard]] virtual QFuture<kernel::Result<QVector<domain::Note>>> list() const = 0;
```

| Layer | Change |
| --- | --- |
| Adapter | Run the query on a worker thread, resolve the future from the worker |
| Use case | Return the future unchanged — no `await`, no callback |
| View model | Observe it with `QFutureWatcher`; set the state when it resolves |
| View | Unchanged — it still reacts to `stateChanged` and `notesLoaded` |

Two details make this correct: **the future resolves on the GUI thread**
(with `QPromise`, resolve it from the worker; the continuation is delivered
to the thread that attached it), and **the state machine already
anticipates it** — `State::Loading` is set *before* the work starts.

The SDK already works this way: `NotesClient` returns a `QPromise`. The
port is the only thing that has not caught up.

## The threads of the target design

| Thread | Owns | Rule |
| --- | --- | --- |
| GUI | Widgets, view models, models, the UI's `QNetworkAccessManager` | No blocking call, ever |
| Storage worker | One `QSqlDatabase` connection | One connection, one thread, closed there |
| Sync worker | The SDK client, the outbox | Started by the GUI thread, stopped by it |

A worker is created in the GUI thread and stopped in the GUI thread. The
shutdown order matters: stop the worker, wait for it, then destroy the
database.

## Qt's own rules, in short

| Rule | Why |
| --- | --- |
| A `QObject` lives in the thread that created it | Its children, timers and queued connections follow it |
| `moveToThread` must be called from the object's own thread | Otherwise the affinity does not change |
| `QSqlDatabase` is not a value type | Copying the handle does not copy the connection |
| `QNetworkAccessManager` is not thread-safe | One per thread |

## Testing the rules

The suite runs headless (`QT_QPA_PLATFORM=offscreen`), so a view model test
runs on the GUI thread of the test process. A test that asserts "the
repository was not called on the GUI thread" needs a fake that records
`QThread::currentThread()`, and it is worth writing: it is the test that
keeps the blocking call from coming back. The fake is the seam: it is why
the view model can be tested without a database, and it is where the thread
assertion belongs.

## Do / don't

| Do | Don't |
| --- | --- |
| Move the query to a worker thread | Run `QSqlQuery::exec()` in a slot |
| Use `QFutureWatcher` to observe the result | Poll a flag with a `QTimer` |
| Stop the worker in the GUI thread | Let a worker outlive what it touches |
| Keep one `QNetworkAccessManager` per thread | Share one across threads |
| Assert the thread in the fake | Assume the call is fast enough |

## See also

- [Data Flows](data-flows.md) — the thread of every hop.
- [Hexagonal Layers & MVVM](04-hexagonal-layers-and-mvvm.md) — the port.
- [Storage & Migrations](08-storage-and-migrations.md) — the SQLite layer.
- [Offline & Outbox](09-offline-and-outbox.md) — the sync worker.
