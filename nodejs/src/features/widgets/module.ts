// Features widgets module: the slice's registration seam. bin/api.ts is the
// only caller — slices never see each other.

import type { FastifyInstance } from "fastify";
import { requireAuth, type Verifier } from "#platform/authx.js";
import { widgetHttp } from "./http.js";
import type { Publisher, Store } from "./ports.js";
import { UseCase } from "./usecase.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  publisher: Publisher | null;
  verifier: Verifier;
  cookieName: string; // forwarded to requireAuth
}

// register mounts the slice's routes; everything sits behind requireAuth.
export const registerWidgets = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.publisher);
  const h = widgetHttp(uc, d.publisher);
  const auth = requireAuth(d.verifier, d.cookieName);

  app.get("/api/v1/widgets", { preHandler: [auth] }, h.list);
  app.post("/api/v1/widgets", { preHandler: [auth] }, h.create);
  app.get("/api/v1/widgets/events", { preHandler: [auth] }, h.events);
  app.get("/api/v1/widgets/:id", { preHandler: [auth] }, h.get);
  app.put("/api/v1/widgets/:id", { preHandler: [auth] }, h.update);
  app.delete("/api/v1/widgets/:id", { preHandler: [auth] }, h.remove);
};
