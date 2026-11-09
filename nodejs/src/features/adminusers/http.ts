// Features adminusers HTTP adapter: fastify handlers over the use-case, at
// reference /admin/users + /auth/upgrade-to-admin parity. Every request is
// admin-gated (401 unauthenticated, 403 by rank); App errors throw — the
// app-wide error handler answers the {detail, code} envelope.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, queryPage, requestIP, sendJSON } from "#platform/httpx.js";
import type { Identity } from "#platform/authx.js";
import { UseCase } from "./usecase.js";
import type { Row, Patch } from "./domain.js";

type AdminUser = components["schemas"]["AdminUser"];
type AdminUserList = components["schemas"]["AdminUserList"];
type AuthUpgradeResponse = components["schemas"]["AuthUpgradeResponse"];

const updateBody = z.object({
  is_active: z.boolean().nullish().default(null),
  is_blocked: z.boolean().nullish().default(null),
  role: z.enum(["ADMIN", "MODERATOR", "USER", "GUEST"]).nullish().default(null),
});
const emailBody = z.object({ email: z.email() });

// adminModel maps a slice Row onto the generated API model; a zero last_login
// reads as dropped (reference pointer-nil parity).
const adminModel = (r: Row): AdminUser => ({
  id: r.id,
  email: r.email,
  first_name: r.firstName,
  last_name: r.lastName,
  role: r.role as AdminUser["role"],
  user_type: r.userType as AdminUser["user_type"],
  is_active: r.isActive,
  is_blocked: r.isBlocked,
  is_verified: r.isVerified,
  language: r.language,
  created_at: r.createdAt.toISOString(),
  last_login: r.lastLogin.getTime() !== 0 ? r.lastLogin.toISOString() : undefined,
});

// actor pulls the identity set by requireAuth (the preHandler guarantees it).
const actor = (req: FastifyRequest): Identity => req.identity as Identity;

// adminusersHttp builds the handler table.
export const adminusersHttp = (uc: UseCase) => ({
  // list renders one page plus the total row count.
  list: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const { limit, offset } = queryPage(req);
    const { rows, total } = await uc.list(limit, offset);
    sendJSON(reply, 200, {
      items: rows.map(adminModel),
      total,
    } satisfies AdminUserList);
  },

  // get renders a single user by id.
  get: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const row = await uc.get((req.params as { id: string }).id);
    sendJSON(reply, 200, adminModel(row));
  },

  // update applies the nullable patch and returns the fresh row.
  update: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(updateBody, req.body ?? {});
    const inp: Patch = {
      isActive: b.is_active ?? null,
      isBlocked: b.is_blocked ?? null,
      role: b.role ?? "",
    };
    const a = actor(req);
    const row = await uc.update(
      a.userId,
      a.role,
      (req.params as { id: string }).id,
      requestIP(req),
      userAgentOf(req),
      inp,
    );
    sendJSON(reply, 200, adminModel(row));
  },

  // upgradeToAdmin is the unauthenticated bootstrap route: the
  // <APP>_ALLOWED_ADMIN_EMAILS whitelist is the only gate, so no requireAuth
  // here (reference upgrade_to_admin parity).
  upgradeToAdmin: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(emailBody, req.body);
    const res = await uc.upgradeToAdmin(b.email);
    sendJSON(reply, 200, {
      email: res.email,
      message: res.message,
      upgraded: res.upgraded,
    } satisfies AuthUpgradeResponse);
  },
});

const userAgentOf = (req: FastifyRequest): string => {
  const v = req.headers["user-agent"];
  return typeof v === "string" ? v : Array.isArray(v) ? (v[0] ?? "") : "";
};
