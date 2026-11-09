// Command api middleware — the chain: request-id, security headers, CORS.
// (The Go template keeps these in cmd/api/middleware.go; same layer, same
// hooks, fastify onRequest form.)

import { randomUUID } from "node:crypto";
import type { onRequestHookHandler } from "fastify";
import type { Config } from "#platform/config.js";

// requestID propagates an incoming X-Request-ID or mints one; it is always
// echoed on the response.
export const requestID: onRequestHookHandler = async (req, reply) => {
  const hdr = req.headers["x-request-id"];
  const rid = typeof hdr === "string" && hdr !== "" ? hdr : randomUUID();
  reply.header("x-request-id", rid);
};

// securityHeaders are baseline response hardening (reference parity).
export const securityHeaders: onRequestHookHandler = async (_req, reply) => {
  reply.header("x-content-type-options", "nosniff");
  reply.header("x-frame-options", "DENY");
  reply.header("referrer-policy", "no-referrer");
  reply.header("cache-control", "no-store");
};

// cors enforces the exact-origin allowlist; "*" (development only) matches
// everything. Preflight OPTIONS answers 204 without touching the routes.
export const cors =
  (cfg: Config): onRequestHookHandler =>
  async (req, reply) => {
    const origin = req.headers["origin"];
    if (typeof origin === "string" && allowed(cfg.corsOrigins, origin)) {
      reply.header("access-control-allow-origin", origin);
      reply.header("vary", "Origin");
      if (req.method === "OPTIONS") {
        reply.header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS");
        reply.header("access-control-allow-headers", "Authorization, Content-Type, X-Request-ID");
        reply.header("access-control-max-age", "600");
        await reply.code(204).send();
        return;
      }
    }
  };

// allowed checks the origin against the exact allowlist ("*" = any).
const allowed = (list: string[], origin: string): boolean =>
  origin !== "" && (list.includes("*") || list.includes(origin));
