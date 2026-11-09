// Features authsignup module: the slice's registration seam. bin/api.ts is
// the only caller — slices never see each other.

import type { FastifyInstance } from "fastify";
import { authsignupHttp } from "./http.js";
import { UseCase } from "./usecase.js";
import type { EmailEnqueuer, Store } from "./ports.js";
import type { Clock } from "#shared/clockx.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  emails: EmailEnqueuer;
  clock: Clock;
  pepper: string;
  verifyTtlMs: number;
}

// register mounts the slice's routes (public: no requireAuth).
export const registerAuthsignup = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.emails, d.clock, d.pepper, d.verifyTtlMs);
  const h = authsignupHttp(uc);
  app.post("/api/v1/auth/signup", h.signup);
  app.post("/api/v1/auth/verify", h.verifyEmail);
  app.post("/api/v1/auth/verify/resend", h.resend);
};
