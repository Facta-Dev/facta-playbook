# Realtime & Streaming

Two channels out of the box: WebSocket (`/api/v1/ws`) for bidirectional and
SSE (`/api/v1/widgets/events`) for one-way fan-out — both riding a small
in-process hub behind one interface. Redis pub/sub is the documented evolution,
not a dependency today.

## 1. The hub contract (`platform/realtime`)

```ts
interface RealtimeEvent {
  room: string; // e.g. `user:${id}` or "widgets"
  type: string; // e.g. "widget.created"
  payload: unknown;
  at: Date;
}

interface Subscriber extends AsyncIterableIterator<RealtimeEvent> {
  cancel(): void; // idempotent; unregisters the subscription
}

interface EventBus {
  publish(ev: RealtimeEvent): void; // non-blocking: drops, never blocks the caller
  subscribe(room: string): Subscriber;
}

const hub = newHub(16); // per-subscriber buffer; single process — Redis adapter is the upgrade path
```

Design notes: `publish` is non-blocking by contract — a slow subscriber loses
events, never the publishing request path (ponytail ceiling: per-subscriber
overflow policies come with the Redis adapter). `subscribe` returns an async
iterator with an idempotent `cancel()`; forgetting to call it leaks the
subscription into the room's `Set`, so every subscriber pairs the iteration
with `sub.cancel()` on teardown (`res.on("close", ...)`, `conn.on("close",
...)`). Rooms are plain strings (`roomUser(id)` builds the identity room) and
the hub knows nothing about who is in them.

## 2. WebSocket (`/api/v1/ws?token=<jwt>`)

Served by the `realtime` slice over `ws` (`WebSocketServer({ noServer: true
})`), intercepting the raw server's `upgrade` event — fastify routes HTTP
only, so the upgrade is claimed before routing:

- **Auth**: browsers cannot set `Authorization` on WS handshakes, so the token
  comes from the query string. Parse with the authx verifier; on failure the
  handshake **completes and the socket closes with code 4001, reason
  `authentication_failed`** — close codes are meaningless before the
  handshake, and clients get a programmatically-observable rejection.
- **Frames**: JSON `{"type": "...", "payload": {...}}` both directions;
  client frames are drained (this hub is publish-only).
- **Keepalive**: the pump pings every 30s; a missed pong terminates. Every
  close/error path stops the interval and cancels the subscription — a hung
  client is a cleanup, not a stuck pump.
- **Rooms**: the identity room `user:<id>` is the default; rooms are plain
  strings and the hub knows nothing about who is in them.

Identity claims (`sub`, `role`) gate the subscription: the connection is
subscribed to `roomUser(verified.userId)` only after the token verifies — the
verify _is_ the subscribe-time check, re-run per connection, never a
connect-only posture.

## 3. SSE (`/api/v1/widgets/events`)

A normal `GET` behind `requireAuth` that never ends: the handler calls
`reply.hijack()` (bypassing fastify's serialization), writes the headers and
the `retry:` hint, then streams hub events per line:

```ts
reply.hijack();
reply.raw.writeHead(200, {
  "content-type": "text/event-stream",
  "cache-control": "no-cache",
  connection: "keep-alive",
});
reply.raw.write("retry: 2000\n\n");
const sub = pub.subscribe(roomUser(identityOf(req)));
reply.raw.on("close", () => sub.cancel());
for await (const ev of sub) {
  reply.raw.write(`event: ${ev.type}\ndata: ${JSON.stringify(ev.payload)}\n\n`);
}
```

The `widgets` use case publishes lifecycle events (`created` / `updated` /
`deleted`) into the hub **through the publisher port** — the SSE handler is
just another subscriber (`publisher: EventBus | null`; a hub-less wiring
streams nothing and returns fast). Client disconnect ends the stream via the
response `close` event; the buffer is small (drops over stale clients are
fine).

## 4. Fan-out across processes: the Redis evolution

The in-process hub is correct for one replica. The moment you run >1 instance
(or want WS on pod A to see an event published on pod B), swap the _adapter_,
not the architecture:

- Keep the `EventBus` interface exactly as is.
- Add `platform/realtime/redisbus` (SUB over a channel, republish into the
  local hub; the local hub stays the per-process fan-out).
- Move the outbox worker's publish decision to include it if reliability
  matters more than latency (at-least-once vs at-most-once).

The interface is deliberately one file; that is the whole migration.

## 5. Testability

- Unit: a fake `EventBus` captures publishes or hands back an
  immediately-buffered subscriber — ten lines, no mock framework (see
  `hub.test.ts` for the real hub's drop/cancel semantics).
- Acceptance: the fake-wired app listens on an ephemeral port and the SSE
  steps read via `fetch` — SSE needs a true server, `app.inject` cannot
  stream. The step opens the stream first and waits for the `retry: 2000`
  handshake (the handler subscribes before its first byte), then creates the
  widget and asserts the `widget.created` frame — deterministic, no sleeps.
- Integration lane: a real hub against real sockets, same as today — the
  event loop is the runtime, not a test concern.

## Do / Don't

- **Do** publish from use cases, never from adapters' internals — events are
  domain facts.
- **Do** keep payloads small and versioned (typed constants for `type`).
- **Don't** hold DB transactions open across a stream write.
- **Don't** let the hub import slices, or slices import the hub
  _implementation_ — the port is the only seam, and slices never import each
  other.

## See also

- [Request Lifecycle](07-request-lifecycle.md) — the WS/SSE aside
- [Vertical Slicing](02-vertical-slicing.md) — who owns which channel
- [Testing & CI](11-testing-and-ci.md)
