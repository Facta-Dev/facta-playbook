// Features authsession module: the slice's registration seam. bin/api.ts is
// the only caller — slices never see each other.

import type { FastifyInstance } from "fastify";
import { requireAuth, requireRole, type Verifier } from "#platform/authx.js";
import { Admin } from "#shared/rolex.js";
import type { Clock } from "#shared/clockx.js";
import { authsessionHttp } from "./http.js";
import { UseCase } from "./usecase.js";
import type { Store, TokenMinter } from "./ports.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  minter: TokenMinter;
  verifier: Verifier;
  clock: Clock;
  pepper: string;
  cookieName: string; // "" disables the cookie fallback
  cookieSecure: boolean; // Set-Cookie Secure flag (false only for dev)
  accessTtlMs: number;
  refreshTtlMs: number;
  maxLoginRetry: number;
}

// register mounts the slice's routes; protected routes sit behind requireAuth
// and the admin auth-session endpoints additionally behind requireRole(Admin)
// (reference /admin/auth-session; the admin edge is the rank gate).
export const registerAuthsession = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(
    d.store,
    d.minter,
    d.clock,
    d.pepper,
    d.accessTtlMs,
    d.refreshTtlMs,
    d.maxLoginRetry,
  );
  const h = authsessionHttp(uc, {
    accessTtlMs: d.accessTtlMs,
    cookieName: d.cookieName,
    cookieSecure: d.cookieSecure,
  });
  const auth = requireAuth(d.verifier, d.cookieName);
  app.post("/api/v1/auth/login", h.login);
  app.post("/api/v1/auth/refresh", h.refresh);
  app.post("/api/v1/auth/logout", { preHandler: auth }, h.logout);
  app.get("/api/v1/auth/me", { preHandler: auth }, h.me);
  app.put("/api/v1/auth/me", { preHandler: auth }, h.updateMe);
  // session-management views (reference /user/auth-session)
  app.get("/api/v1/auth/sessions", { preHandler: auth }, h.sessions);
  app.get("/api/v1/auth/sessions/current", { preHandler: auth }, h.currentSession);
  app.post("/api/v1/auth/sessions/revoke-current", { preHandler: auth }, h.revokeCurrent);
  app.post("/api/v1/auth/sessions/revoke-all", { preHandler: auth }, h.revokeAll);
  // admin auth-session (reference /admin/auth-session)
  const admin = { preHandler: [auth, requireRole(Admin)] };
  app.get("/api/v1/admin/auth-session/users/:id/sessions", admin, h.adminSessions);
  app.post("/api/v1/admin/auth-session/users/:id/revoke-all", admin, h.adminRevokeAll);
  app.get("/api/v1/admin/auth-session/anomalies", admin, h.anomalies);
};
