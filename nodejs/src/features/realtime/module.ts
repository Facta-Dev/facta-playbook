// Realtime slice: one WS endpoint over the platform hub and authx verifier
// (go internal/features/realtime/module.go parity).

import { WebSocketServer, type WebSocket } from "ws";
import type { FastifyInstance } from "fastify";
import type { Verifier } from "#platform/authx.js";
import type { EventBus } from "#platform/realtime/index.js";
import { closeAuthFailed, serveConn } from "#platform/realtime/ws.js";
import { roomUser } from "#platform/realtime/index.js";

// RealtimeDeps carries everything Register needs; bin/api.ts is the only
// caller. go's Verifier/Publisher ports map onto authx + realtime platform.
export interface RealtimeDeps {
  verifier: Verifier;
  publisher: EventBus;
}

// registerRealtime mounts the WS route on the raw server's upgrade event
// (Fastify routes HTTP only; the upgrade is intercepted before routing).
// Bad tokens still complete the handshake and get close 4001, so clients can
// react programmatically — go's upgrade-then-authenticate parity.
export const registerRealtime = (app: FastifyInstance, d: RealtimeDeps): void => {
  const wss = new WebSocketServer({ noServer: true });
  app.addHook("onClose", async () => {
    wss.clients.forEach((c) => c.close());
    await new Promise<void>((r) => wss.close(() => r()));
  });
  app.server.on("upgrade", (req, socket, head) => {
    const pathname = (req.url ?? "").split("?")[0];
    if (pathname !== "/api/v1/ws") {
      socket.destroy(); // not our upgrade; let nothing else claim it
      return;
    }
    wss.handleUpgrade(req, socket, head, async (conn: WebSocket) => {
      const id = await d.verifier
        .parse(new URL(req.url ?? "/", "http://x").searchParams.get("token") ?? "")
        .catch(() => null);
      if (id === null) {
        closeAuthFailed(conn);
        return;
      }
      serveConn(conn, d.publisher.subscribe(roomUser(id.userId)));
    });
  });
};
