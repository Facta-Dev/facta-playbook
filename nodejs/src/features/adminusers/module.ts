// Features adminusers module: the slice's registration seam. bin/api.ts is
// the only caller — slices never see each other.

import type { FastifyInstance } from "fastify";
import { requireAuth, requireRole, type Verifier } from "#platform/authx.js";
import { Admin } from "#shared/rolex.js";
import { adminusersHttp } from "./http.js";
import { UseCase } from "./usecase.js";
import type { Store } from "./ports.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  verifier: Verifier;
  cookieName: string; // forwarded to requireAuth
  whitelist: string[]; // <APP>_ALLOWED_ADMIN_EMAILS for the bootstrap upgrade
}

// register mounts the slice's routes: requireAuth verifies the bearer/cookie
// token and stores the Identity; requireRole(Admin) then gates by rank — an
// admin request is 401 when unauthenticated and 403 when not admin.
export const registerAdminusers = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.whitelist);
  const h = adminusersHttp(uc);
  const admin = { preHandler: [requireAuth(d.verifier, d.cookieName), requireRole(Admin)] };
  app.get("/api/v1/admin/users", admin, h.list);
  app.get("/api/v1/admin/users/:id", admin, h.get);
  app.put("/api/v1/admin/users/:id", admin, h.update);
  // Bootstrap whitelist upgrade: unauthenticated by design — the
  // <APP>_ALLOWED_ADMIN_EMAILS whitelist is the only gate (reference
  // upgrade_to_admin parity).
  app.post("/api/v1/auth/upgrade-to-admin", h.upgradeToAdmin);
};
