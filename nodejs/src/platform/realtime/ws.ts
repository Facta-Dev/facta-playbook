// Realtime WS adapter: the wire shape and per-connection pump (go
// internal/platform/realtime/ws.go parity) over the `ws` package.

import type { WebSocket } from "ws";
import type { Subscriber } from "./hub.js";

// Frame is the wire shape in both directions: {"type", "payload"} (ctx0).
export interface Frame {
  type: string;
  payload?: unknown;
}

// Keepalive interval and the close semantics for failed authentication.
export const pingPeriodMs = 30_000;
export const authFailedCloseCode = 4001;

// closeAuthFailed rejects a connection whose token failed verification; call
// only after the handshake completed (close codes are meaningless before it).
export const closeAuthFailed = (conn: WebSocket): void => {
  conn.close(authFailedCloseCode, "authentication_failed");
};

// serveConn pumps one connection: discards client frames, pings every 30s and
// streams hub events as JSON frames. Cleanup runs on peer disconnect or send
// failure; the caller (module.ts) owns the handshake and close.
export const serveConn = (conn: WebSocket, sub: Subscriber): void => {
  conn.on("message", () => undefined); // drain client frames
  let pong = true; // missing pong => peer is gone, stop pumping
  const ping = setInterval(() => {
    if (!pong) {
      stop();
      conn.terminate();
      return;
    }
    pong = false;
    conn.once("pong", () => {
      pong = true;
    });
    conn.ping();
  }, pingPeriodMs);
  const stop = (): void => {
    clearInterval(ping);
  };
  conn.on("close", () => {
    stop();
    sub.cancel();
  });
  conn.on("error", () => {
    stop();
  });

  // The event pump: hub events stream as JSON frames; when the publisher
  // closes the subscription the connection closes normally (1000).
  const pump = async (): Promise<void> => {
    for await (const ev of sub) {
      try {
        conn.send(JSON.stringify({ type: ev.type, payload: ev.payload ?? undefined }));
      } catch {
        stop();
        return;
      }
    }
    conn.close(1000, "");
  };
  void pump();
};
