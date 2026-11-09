// Features authpassword module: the slice's registration seam. bin/api.ts is
// the only caller — slices never see each other.

import type { FastifyInstance } from "fastify";
import { requireAuth, type Verifier } from "#platform/authx.js";
import type { Clock } from "#shared/clockx.js";
import { authpasswordHttp } from "./http.js";
import { UseCase } from "./usecase.js";
import type { EmailEnqueuer, Store } from "./ports.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  emails: EmailEnqueuer;
  verifier: Verifier;
  clock: Clock;
  pepper: string;
  cookieName: string; // forwarded to requireAuth
  resetTtlMs: number; // code lifetime (30m)
  rateLimit: number; // max codes per window
  rateWindowMs: number; // rate-limit window (5m)
  logOutAllOnReset: boolean;
}

// register mounts the slice's routes; change-password sits behind requireAuth.
export const registerAuthpassword = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(
    d.store,
    d.emails,
    d.clock,
    d.pepper,
    d.resetTtlMs,
    d.rateWindowMs,
    d.rateLimit,
    d.logOutAllOnReset,
  );
  const h = authpasswordHttp(uc);
  app.post("/api/v1/auth/forgot-password", h.forgot);
  app.post("/api/v1/auth/validate-code", h.validate);
  app.post("/api/v1/auth/reset-password", h.reset);
  app.post(
    "/api/v1/auth/change-password",
    { preHandler: requireAuth(d.verifier, d.cookieName) },
    h.change,
  );
};
