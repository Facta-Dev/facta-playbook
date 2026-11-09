# Realtime & Streaming

Two channels out of the box: WebSocket (`/api/v1/ws`) for bidirectional and
SSE (`/api/v1/widgets/events`) for one-way fan-out — both riding a small
in-process hub behind one interface. Redis pub/sub is the documented evolution,
not a dependency today.

## 1. The hub contract (`internal/platform/realtime`)

```go
type Event struct {
    Room    string          // e.g. "user:<uuid>" or "widgets"
    Type    string          // e.g. "widget.created"
    Payload json.RawMessage
    At      time.Time
}

type EventBus interface {
    Publish(e Event)              // non-blocking: drops, never blocks the caller
    Subscribe(room string) (<-chan Event, func()) // cancel func unsubscribes
}

func NewHub(buf int) *Hub // single process; Redis adapter is the upgrade path
```

Design notes: `Publish` is non-blocking by contract — a slow subscriber loses
events, never the publishing request path (ponytail ceiling: per-subscriber
overflow policies come with the Redis adapter). `Subscribe` returns a cancel
function; forgetting to call it leaks, so every subscriber pairs
`defer cancel()` with `ctx.Done()` in its select.

## 2. WebSocket (`/api/v1/ws?token=<jwt>`)

Served by the `realtime` slice over `coder/websocket`:

- **Auth**: browsers cannot set `Authorization` on WS handshakes, so the token
  comes from the query string. Parse directly with the authx verifier; on
  failure **close with code 4001 and reason `authentication_failed`** before
  any upgrade completes.
- **Frames**: JSON `{"type": "...", "payload": {...}}` both directions.
- **Keepalive**: server pings; missed pong → close. Readers always run with a
  read deadline and the request context — a hung client is a canceled
  context, not a stuck goroutine.
- **Rooms**: the identity room `user:<id>` is the default; rooms are plain
  strings and the hub knows nothing about who is in them.

Identity claims (`sub`, `role`) gate room subscriptions; the slice re-checks
on every subscribe, never on connect alone.

## 3. SSE (`/api/v1/widgets/events`)

A normal `GET` behind `RequireAuth` that never ends: header
`text/event-stream`, then hub events serialized per line with
`http.Flusher.Flush()` per write:

```go
w.Header().Set("Content-Type", "text/event-stream")
for {
    select {
    case ev := <-ch:  fmt.Fprintf(w, "event: %s\ndata: %s\n\n", ev.Type, ev.Payload)
        f.Flush()
    case <-r.Context().Done(): cancel(); return
    }
}
```

The `widgets` slice's use cases publish lifecycle events (`created` /
`updated` / `deleted`) into the hub **through the event-bus port** — the SSE
handler is just another subscriber. Client disconnect ends the stream via the
request context; the buffer is small (drops over stale clients are fine).

## 4. Fan-out across processes: the Redis evolution

The in-process hub is correct for one replica. The moment you run >1 instance
(or want WS on pod A to see an event published on pod B), swap the *adapter*,
not the architecture:

- Keep the `EventBus` port exactly as is.
- Add `platform/realtime/redisbus` (SUB over a channel, republish into the
  local hub; the local hub stays the per-process fan-out).
- Move the outbox worker's publish decision to include it if reliability
  matters more than latency (at-least-once vs at-most-once).

The interface is deliberately one file; that is the whole migration.

## 5. Testability

- Unit: a fake `EventBus` is a channel capture — publish asserts the event;
  subscribe returns an immediately-buffered channel.
- Acceptance: the fake-wired mux's SSE endpoint is read via an
  `http.Client` against `httptest` and the test publishes through the fake
  first, keeping the test deterministic (no sleeps).
- Integration/-tags: a real hub with real goroutines under `-race`.

## Do / Don't

- **Do** publish from use cases, never from adapters' internals — events are
  domain facts.
- **Do** keep payloads small and versioned (typed constants for `Type`).
- **Don't** hold DB transactions open across a stream write.
- **Don't** let the hub import slices, or slices import the hub *implementation*
  — the port is the only seam, and slices never import each other.

## See also

- [Request Lifecycle](07-request-lifecycle.md) — the WS/SSE aside
- [Vertical Slicing](02-vertical-slicing.md) — who owns which channel
- [Testing & CI](11-testing-and-ci.md)