//! Loom hub — the in-process event bus + websocket pump for real-time delivery.
//!
//! Go `internal/platform/realtime` (hub.go + ws.go) parity: an [`EventBus`]
//! that fans room events out to per-subscriber buffered channels (publish
//! never blocks — events are DROPPED, not queued, when a subscriber's buffer
//! is full), the `user:<id>` identity-room vocabulary, and the websocket pump
//! that discards client frames, pings, and streams hub events as `{"type",
//! "payload"}` JSON frames.
//!
//! ctx0 parity: Redis pub/sub is the documented evolution path (adapter
//! ready) — this hub is in-process only.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use chrono::{DateTime, Utc};
use <app>_warden::JwtCodec;
use serde_json::{Value, json};

/// ctx0 close semantics for failed authentication: close codes are
/// meaningless pre-handshake, so the connection is upgraded FIRST, then
/// closed `4001 "authentication_failed"` when the `?token=` fails to verify.
pub const AUTH_FAILED_CLOSE_CODE: u16 = 4001;
/// Reason string paired with [`AUTH_FAILED_CLOSE_CODE`]. Public because the
/// api client (`<app>-sdk`) mirrors it when decoding closes.
pub const AUTH_FAILED_CLOSE_REASON: &str = "authentication_failed";

/// Keepalive interval (go `pingPeriod`).
const PING_PERIOD: Duration = Duration::from_secs(30);
/// Liveness bound after a ping (go `pingTimeout`).
const PING_TIMEOUT: Duration = Duration::from_secs(10);

/// One notification routed to a room (go `realtime.Event`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// `"user:<id>"` (see [`room_user`]) or a topic room.
    pub room: String,
    /// Event discriminator, e.g. `"widget.created"`.
    pub r#type: String,
    /// Opaque event data; `Value::Null` omits it from the wire frame.
    pub payload: Value,
    /// When the use-case published the event.
    pub at: DateTime<Utc>,
}

impl Event {
    /// Stamp `at` with the current clock (python dataclass default parity).
    #[must_use]
    pub fn new(room: impl Into<String>, r#type: impl Into<String>, payload: Value) -> Self {
        Self {
            room: room.into(),
            r#type: r#type.into(),
            payload,
            at: Utc::now(),
        }
    }
}

/// One-shot unsubscribe handle returned by [`EventBus::subscribe`].
///
/// Closing the subscription stops future deliveries and lets buffered events
/// still drain (go `close(ch)` parity); the per-room entry is removed so the
/// room is pruned once its last subscriber leaves.
pub type CancelFn = Box<dyn FnOnce() + Send>;

/// Room-event fan-out port (go `realtime.EventBus`). `Hub` satisfies it.
pub trait EventBus: Send + Sync + 'static {
    /// Fan the event out to every subscriber of the event's room, never
    /// blocking: a subscriber whose buffer is full DROPS the event (go
    /// `Publish` default-case parity — a slow consumer must not stall a
    /// use-case).
    fn publish(&self, event: Event);

    /// Register one subscriber's buffered stream for `room`. The returned
    /// [`CancelFn`] closes the stream (see [`CancelFn`]).
    fn subscribe(&self, room: &str) -> (tokio::sync::mpsc::Receiver<Event>, CancelFn);
}

/// Identity room vocabulary shared by WS and SSE: one room per user; events
/// published there reach all of that user's live connections (go `RoomUser`).
#[must_use]
pub fn room_user(user_id: &str) -> String {
    format!("user:{user_id}")
}

#[derive(Debug, Default)]
struct HubInner {
    /// Per-subscriber buffer, applied at `subscribe` time (go `NewHub(<=0) => 16`).
    buf: usize,
    mu: Mutex<HashMap<String, HashMap<u64, tokio::sync::mpsc::Sender<Event>>>>,
    next: AtomicU64,
}

/// In-process [`EventBus`]: one room, many subscribers, buffer-per-subscriber
/// with drop-on-full semantics. Cloneable; every clone shares one hub, so the
/// composition root builds exactly one and passes it everywhere.
#[derive(Debug, Clone, Default)]
pub struct Hub {
    inner: Arc<HubInner>,
}

impl Hub {
    /// Build a hub with per-subscriber buffer (`0` => 16, go `NewHub` parity).
    #[must_use]
    pub fn new(buffer: usize) -> Self {
        Self {
            inner: Arc::new(HubInner {
                buf: if buffer == 0 { 16 } else { buffer },
                ..HubInner::default()
            }),
        }
    }
}

impl EventBus for Hub {
    fn publish(&self, event: Event) {
        let mu = self.inner.mu.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(subs) = mu.get(&event.room) {
            for tx in subs.values() {
                // Full => drop for this subscriber only (go default-case).
                // Closed => a cancelled subscription not yet reaped; skipped.
                let _ = tx.try_send(event.clone());
            }
        }
    }

    fn subscribe(&self, room: &str) -> (tokio::sync::mpsc::Receiver<Event>, CancelFn) {
        let id = self.inner.next.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = tokio::sync::mpsc::channel(self.inner.buf);
        {
            let mut mu = self.inner.mu.lock().unwrap_or_else(PoisonError::into_inner);
            mu.entry(room.to_string()).or_default().insert(id, tx);
        }
        let inner = Arc::clone(&self.inner);
        let room = room.to_string();
        let cancel: CancelFn = Box::new(move || {
            let mut mu = inner.mu.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(subs) = mu.get_mut(&room) {
                subs.remove(&id);
            }
        });
        (rx, cancel)
    }
}

/// The wire shape in both directions: `{"type", "payload"}` (go `Frame`).
/// `payload` is omitted when null (go `omitempty`).
pub fn frame(ev: &Event) -> Value {
    if ev.payload.is_null() {
        json!({"type": ev.r#type})
    } else {
        json!({"type": ev.r#type, "payload": ev.payload})
    }
}

/// Pump one authenticated connection (go `realtime.ServeConn`).
///
/// Discards client frames (a `Pong` clears the pending-liveness check),
/// pings every [`PING_PERIOD`] and streams hub events as JSON frames.
/// Returns (and drops the socket halves, closing the connection) on peer
/// disconnect, a subscription close, an unanswered-ping timeout, or a write
/// failure — go's `ServeConn` return set parity.
pub async fn serve_conn(socket: WebSocket, events: tokio::sync::mpsc::Receiver<Event>) {
    use futures_util::{SinkExt, StreamExt};
    use tokio::time::{MissedTickBehavior, interval, sleep_until};

    let (mut tx, mut rx_half) = socket.split();
    let mut events = events;
    // go `time.NewTicker(pingPeriod)`: first fire at +period, not immediately.
    let mut ping = interval(PING_PERIOD);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ping.tick().await; // consume tokio's immediate first tick
    let mut awaiting_pong = false;
    // Reset per ping; the select's guard keeps the deadline polled only while
    // a ping is outstanding (10s, go `pingTimeout` parity).
    let mut deadline = Box::pin(sleep_until(tokio::time::Instant::now()));

    loop {
        tokio::select! {
            // Discard client frames (go read loop): a `Pong` clears liveness.
            incoming = rx_half.next() => match incoming {
                // Peer disconnect / close handshake ends the pump.
                None | Some(Ok(Message::Close(_)) | Err(_)) => return,
                Some(Ok(Message::Pong(_))) => awaiting_pong = false,
                Some(Ok(_)) => {}
            },
            // Stream hub events as `{"type", "payload"}` frames; a write
            // failure (peer gone) ends the pump.
            Some(event) = events.recv() => {
                if tx
                    .send(Message::Text(frame(&event).to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            _ = ping.tick() => {
                // An outstanding ping that stayed unanswered for a full
                // period means the peer is (worse than) gone: stop pumping.
                if awaiting_pong {
                    return;
                }
                if tx.send(Message::Ping(vec![].into())).await.is_err() {
                    return;
                }
                awaiting_pong = true;
                deadline = Box::pin(sleep_until(
                    tokio::time::Instant::now() + PING_TIMEOUT,
                ));
            }
            () = &mut deadline, if awaiting_pong => {
                // go: `conn.Ping(pctx)` unmet within pingTimeout => return.
                return;
            }
        }
    }
}

/// Serve one `/ws` upgrade lifetime (go `features/realtime` `h.ws` parity).
///
/// Upgrades FIRST, then authenticates: a bad `?token=` (raw query, not
/// percent-decoded — JWTs are URL-safe unreserved) gets the 4001
/// `"authentication_failed"` close so clients can react programmatically. On
/// any verify error — not just expiry — the connection is auth-failed closed.
/// Verifies against the shared [`JwtCodec`] and subscribes to the caller's
/// identity room until the pump ends.
pub fn serve_ws(
    ws: WebSocketUpgrade,
    query: Option<String>,
    codec: Arc<JwtCodec>,
    hub: Arc<dyn EventBus>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        let token = query.as_deref().and_then(query_token).unwrap_or_default();
        match codec.verify(&token) {
            Err(_) => close_auth_failed(socket).await,
            Ok(claims) => {
                let (rx, cancel) = hub.subscribe(&room_user(&claims.sub.to_string()));
                serve_conn(socket, rx).await;
                drop(cancel);
            }
        }
    })
}

/// Reject a connection whose token failed verification (go `CloseAuthFailed`).
/// Call only after the upgrade — close codes are meaningless pre-handshake.
async fn close_auth_failed(socket: WebSocket) {
    let mut socket = socket;
    let _ = socket
        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
            code: AUTH_FAILED_CLOSE_CODE,
            reason: AUTH_FAILED_CLOSE_REASON.into(),
        })))
        .await;
}

/// `?token=` from a raw query string. Only the first `token` key wins; the
/// value is NOT percent-decoded — JWTs are URL-safe unreserved (go
/// `r.URL.Query().Get("token")` reads the same shape for real tokens).
fn query_token(query: &str) -> Option<String> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("token=").map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::{sleep, timeout};

    use super::{Event, EventBus, Hub, room_user};

    fn ev(room: &str, r#type: &str) -> Event {
        Event::new(room_user(room), r#type, serde_json::json!({}))
    }

    #[tokio::test]
    async fn subscribe_publish_cancel_delivers() {
        let hub = Hub::new(4);
        let (mut rx, cancel) = hub.subscribe(&room_user("u-1"));

        hub.publish(Event::new(
            room_user("u-1"),
            "widget.created",
            serde_json::json!({"id": "w-1"}),
        ));
        let got = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("event not delivered");
        let got = got.expect("event is Some");
        assert_eq!(got.r#type, "widget.created");
        assert_eq!(got.payload, serde_json::json!({"id": "w-1"}));
        drop(cancel);
    }

    #[tokio::test]
    async fn room_isolation() {
        let hub = Hub::new(4);
        let (mut rx, _cancel) = hub.subscribe(&room_user("a"));
        hub.publish(ev("b", "nope"));
        assert!(
            timeout(Duration::from_millis(50), rx.recv()).await.is_err(),
            "cross-room leak"
        );
    }

    #[tokio::test]
    async fn drops_when_full() {
        let hub = Hub::new(1);
        let (mut rx, _cancel) = hub.subscribe(&room_user("a"));
        hub.publish(ev("a", "one")); // fills the buffer
        hub.publish(ev("a", "two")); // must drop, not block
        hub.publish(ev("a", "three"));
        // Publish must have returned even though the buffer holds one event,
        // and the buffered (oldest) event still delivered.
        let got = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("queued event lost");
        assert_eq!(got.map(|e| e.r#type), Some("one".to_string()));
        // The buffer now holds at most one more event: a rapid drain proves
        // the other two publishes dropped instead of queueing behind it.
        let mut drained = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        while tokio::time::Instant::now() < deadline {
            match rx.try_recv() {
                Ok(_) => drained += 1,
                Err(
                    tokio::sync::mpsc::error::TryRecvError::Empty
                    | tokio::sync::mpsc::error::TryRecvError::Disconnected,
                ) => break,
            }
        }
        assert!(drained <= 1, "buffer grew beyond capacity: {drained}");
    }

    #[tokio::test]
    async fn cancel_closes() {
        let hub = Hub::new(1);
        let (mut rx, cancel) = hub.subscribe(&room_user("a"));
        cancel();
        assert!(
            rx.recv().await.is_none(),
            "want closed channel after cancel"
        );
    }

    #[tokio::test]
    async fn cancel_drains_buffered_then_closes() {
        // go close(ch) parity: buffered values stay readable after cancel.
        let hub = Hub::new(2);
        let (mut rx, cancel) = hub.subscribe(&room_user("a"));
        hub.publish(ev("a", "queued"));
        cancel();
        let got = rx.recv().await.expect("buffered event lost");
        assert_eq!(got.r#type, "queued");
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn cancelled_subscriber_stops_receiving() {
        let hub = Hub::new(2);
        let (mut rx, cancel) = hub.subscribe(&room_user("a"));
        cancel();
        hub.publish(ev("a", "late"));
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn same_room_broadcast_reaches_both_subscribers() {
        let hub = Hub::new(8);
        let (mut first, _c1) = hub.subscribe(&room_user("u-1"));
        let (mut second, _c2) = hub.subscribe(&room_user("u-1"));
        hub.publish(ev("u-1", "tick"));
        let firstgot = timeout(Duration::from_secs(1), first.recv())
            .await
            .expect("first")
            .expect("event is Some");
        assert_eq!(firstgot.r#type, "tick");
        let secondgot = timeout(Duration::from_secs(1), second.recv())
            .await
            .expect("second")
            .expect("event is Some");
        assert_eq!(secondgot.r#type, "tick");
        sleep(Duration::ZERO).await; // keep the receivers open until both drains
    }

    #[test]
    fn room_user_prefix() {
        assert!(room_user("u-1").starts_with("user:"));
    }

    #[test]
    fn frame_omits_null_payload() {
        use super::frame;
        let with_payload = Event::new("user:a", "t", serde_json::json!({"k": 1}));
        assert_eq!(
            frame(&with_payload),
            serde_json::json!({"type": "t", "payload": {"k": 1}})
        );
        let bare = Event::new("user:a", "t", serde_json::Value::Null);
        assert_eq!(frame(&bare), serde_json::json!({"type": "t"}));
    }
}
