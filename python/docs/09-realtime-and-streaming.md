# Realtime & Streaming

Two channels out of the box: WebSocket (`/api/v1/ws`) for bidirectional and
SSE (`/api/v1/widgets/events`) for one-way fan-out — both riding a small
in-process hub. Redis pub/sub is the documented evolution, not a dependency
today.

## 1. The hub contract (`platform/realtime`)

```python
class Hub:
    """Rooms are process-local; a second replica needs a real broker."""

    async def subscribe(self, room: str) -> Subscription: ...
    async def unsubscribe(self, subscription: Subscription) -> None: ...
    async def publish(self, room: str, event: dict[str, Any]) -> None: ...
```

`Subscription` is a room name plus a bounded `asyncio.Queue(32)`. Rooms are
plain strings (`user:{id}`) and the hub knows nothing about who is in them.

Design notes: `publish` never blocks the caller — if a subscriber's queue
is full, the **oldest** event is dropped to make room for the new one
(ponytail ceiling: per-subscriber overflow policies come with a real
broker). `unsubscribe` is idempotent and removes the room entirely when its
last listener leaves, so a forgotten unsubscribe leaks a queue, not a room.
Every subscriber pairs its subscription with `unsubscribe` on teardown (the
`finally:` block in the WS loop and in the SSE generator) — forgetting to
call it leaks the subscription into the room's set.

## 2. WebSocket (`/api/v1/ws?token=<jwt>`)

Served by the `realtime` slice (`app/features/realtime/module.py`) over the
platform loop (`app/platform/realtime/ws.py`):

- **Auth**: browsers cannot set `Authorization` on WS handshakes, so the
  token comes from the query string and is parsed directly with the HS256
  verifier. A missing or invalid token still **completes the handshake**
  (`accept()`) and only then closes the socket with code **4001, reason
  `authentication_failed`** — the client observes the close code instead of
  an opaque handshake rejection.
- **Frames**: JSON both directions; the frame shape is `{"type": "...", ...}`.
  The server sends `{"type": "hello", "user_id": ...}` right after accepting,
  answers a client frame of `"ping"` with `{"type": "pong"}`, and pushes
  hub events as they arrive.
- **Keepalive**: an application-level `{"type": "ping"}` every 30 s
  (`_PING_SECONDS`); it is a liveness probe, not a protocol-level ping.
- **Rooms**: the identity room `user:<id>` is the default, and the
  subscription happens only after the token verifies — the verify _is_ the
  subscribe-time check, re-run per connection, never a connect-only posture.
- **Teardown**: the `finally:` block cancels the ping task and unsubscribes,
  so a hung client is a cleanup, not a stuck pump.

## 3. SSE (`/api/v1/widgets/events`)

A normal `GET` behind `require_auth` that never ends: the handler
subscribes to the widgets room and answers a `StreamingResponse`:

```python
subscription = await hub.subscribe(f"user:{identity.user_id}")


async def body() -> AsyncIterator[str]:
    try:
        async for frame in stream(subscription.queue):
            if await request.is_disconnected():
                return
            yield frame
    finally:
        await hub.unsubscribe(subscription)


return StreamingResponse(
    body(),
    media_type="text/event-stream",
    headers={"cache-control": "no-cache", "x-accel-buffering": "no"},
)
```

`platform/realtime/sse.py` frames each event as a single `data:` line with
compact JSON, and emits a `: keep-alive` comment every 15 s so proxies keep
the connection open. Client disconnect ends the stream (the handler polls
`request.is_disconnected()`), and the `finally:` block unbinds the
subscription. The buffer is small — drops over stale clients are fine.

The `widgets` use case publishes lifecycle events (`widget.created` /
`widget.updated` / `widget.deleted`) into the hub **through the publisher
port** — the SSE handler is just another subscriber. The port is optional
(`publisher: Publisher | None`): a hub-less wiring streams nothing and
still returns fast.

## 4. Fan-out across processes: the Redis evolution

The in-process hub is correct for one replica. The moment you run >1
instance (or want WS on pod A to see an event published on pod B), swap the
_adapter_, not the architecture:

- Keep the `Hub` surface (or the `Publisher` port every slice sees) exactly
  as it is.
- Add a `platform/realtime/redisbus` that SUBSCRIBEs to a channel and
  republishes into the local hub; the local hub stays the per-process
  fan-out.
- Decide the delivery contract explicitly (at-least-once vs
  at-most-once) — the outbox already gives you at-least-once for email.

The seam is deliberately one module; that is the whole migration.

## 5. Testability

- Unit: `FakePublisher` (`tests/fakes.py`) captures publishes in a list —
  ten lines, no mock framework. `tests/unit/test_services.py` asserts that a
  widget creation published `("user:{owner_id}", {"type": "widget.created",
  ...})`.
- Acceptance: the fake-wired app listens on an ephemeral port with the
  **real** `Hub` injected (only the stores are faked), so the SSE scenario in
  `specs/features/widgets.feature` opens the stream, creates a widget and
  reads the `widget.created` frame off it. SSE needs a true server, so the
  acceptance tier starts uvicorn in a background thread
  ([Testing & CI](11-testing-and-ci.md)).
- The queue semantics (drop-oldest, idempotent unsubscribe) are the
  concurrency surface to pin when you extend the hub.

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
