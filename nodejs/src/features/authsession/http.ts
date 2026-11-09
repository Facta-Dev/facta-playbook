// Features authsession HTTP adapter: fastify handlers over the use-case, at
// reference /user/auth-session + /admin/auth-session parity. The shared
// generated model (src/gen/api) types the wire shapes; zod re-states the
// openapi contract for decode-time validation. App errors throw — the
// app-wide error handler answers the {detail, code} envelope.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, requestIP, sendJSON } from "#platform/httpx.js";
import { setAuthCookie, clearAuthCookie, type Identity } from "#platform/authx.js";
import { E, CodeUnprocessable, Validation } from "#shared/apperr.js";
import { UseCase, type Result, type UpdateMeInput } from "./usecase.js";
import type { UserRow, SessionRow, Anomaly } from "./domain.js";

type MeResponse = components["schemas"]["MeResponse"];
type AuthTokens = components["schemas"]["AuthTokens"];
type MessageResponse = components["schemas"]["MessageResponse"];

const loginBody = z.object({ email: z.email(), password: z.string().min(1) });
const refreshBody = z.object({ refresh_token: z.string().min(1) });
const updateMeBody = z.object({
  first_name: z.string().max(100).optional(),
  last_name: z.string().max(100).optional(),
  phone_number: z.string().max(40).optional(),
  profile_picture: z.string().max(500).optional(),
  language: z.string().max(10).optional(),
  address: z.string().max(255).optional(),
  postal_code: z.string().max(20).optional(),
});

// meResponse maps the slice's user row onto the widened generated model
// (undefined reads as dropped, omitempty parity).
const meResponse = (u: UserRow): MeResponse => ({
  id: u.id,
  email: u.email,
  first_name: u.firstName,
  last_name: u.lastName,
  role: u.role as MeResponse["role"],
  user_type: u.userType as MeResponse["user_type"],
  is_active: u.isActive,
  is_blocked: u.isBlocked,
  is_verified: u.isVerified,
  retry_count: u.retryCount,
  language: u.language,
  created_at: u.createdAt.toISOString(),
  phone_number: u.phone ?? undefined,
  profile_picture: u.picture ?? undefined,
  subscription: u.subscription ?? undefined,
  last_login: u.lastLogin ? u.lastLogin.toISOString() : null,
  github_connected: u.githubLinked,
});

// sessionItemOf is the wire shape of one session row (reference
// SessionItemResponse): the refresh token itself is never exposed.
const sessionItemOf = (s: SessionRow) => ({
  id: s.id,
  user_id: s.userId,
  ip_address: s.ipAddress === "" ? undefined : s.ipAddress,
  user_agent: s.userAgent === "" ? undefined : s.userAgent,
  created_at: s.createdAt.toISOString(),
  last_activity: s.lastActivity.toISOString(),
  expires_at: s.expiresAt.toISOString(),
  is_active: s.isActive,
});

// anomalyItemOf is the wire shape of one admin anomaly row.
const anomalyItemOf = (a: Anomaly) => ({
  anomaly_type: a.type,
  count: a.count,
  details: a.details,
});

// actorID pulls the identity set by requireAuth (the preHandler guarantees it).
const actorID = (req: FastifyRequest): Identity => req.identity as Identity;

// userAgent mirrors go http.Request.UserAgent(): "" when the header is absent.
const userAgent = (req: FastifyRequest): string => {
  const v = req.headers["user-agent"];
  return typeof v === "string" ? v : Array.isArray(v) ? (v[0] ?? "") : "";
};

// parseBool mirrors strconv.ParseBool's accepted spellings.
const parseBool = (v: string): boolean | null => {
  switch (v.toLowerCase()) {
    case "1":
    case "t":
    case "true":
      return true;
    case "0":
    case "f":
    case "false":
      return false;
    default:
      return null;
  }
};

// queryBool parses a boolean query param with a default.
const queryBool = (req: FastifyRequest, key: string, def: boolean): boolean => {
  const raw = (req.query as Record<string, unknown>)?.[key];
  if (raw === undefined || raw === "") {
    return def;
  }
  const b = parseBool(String(raw));
  if (b === null) {
    throw E(CodeUnprocessable, Validation, `${key} must be a boolean`);
  }
  return b;
};

// authsessionHttp builds the handler table: uc plus the cookie/token options.
export const authsessionHttp = (
  uc: UseCase,
  opts: { accessTtlMs: number; cookieName: string; cookieSecure: boolean },
) => {
  // tokenPair converts a use-case Result into the generated wire shape.
  const tokenPair = (res: Result): AuthTokens => ({
    access_token: res.accessToken,
    refresh_token: res.refreshToken,
    token_type: "bearer",
    expires_in: Math.round(opts.accessTtlMs / 1000),
  });
  // setCookie issues the access-token cookie when cookie mode is on.
  const setCookie = (reply: FastifyReply, token: string): void => {
    if (opts.cookieName !== "") {
      setAuthCookie(reply, opts.cookieName, token, opts.cookieSecure);
    }
  };
  return {
    // login: state gates + retry lock in the use-case; 200 + token pair.
    login: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const b = decode(loginBody, req.body);
      const res = await uc.login({
        email: b.email.toLowerCase().trim(),
        password: b.password,
        ipAddress: requestIP(req),
        userAgent: userAgent(req),
      });
      setCookie(reply, res.accessToken);
      sendJSON(reply, 200, tokenPair(res));
    },

    // refresh: rotation in place; a replayed token answers 401.
    refresh: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const b = decode(refreshBody, req.body);
      const res = await uc.refresh(b.refresh_token, requestIP(req), userAgent(req));
      setCookie(reply, res.accessToken);
      sendJSON(reply, 200, tokenPair(res));
    },

    // logout: bearer-protected; revokes the given refresh token (idempotent 200).
    logout: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const b = decode(refreshBody, req.body);
      await uc.logout(actorID(req).userId, b.refresh_token);
      if (opts.cookieName !== "") {
        clearAuthCookie(reply, opts.cookieName, opts.cookieSecure);
      }
      sendJSON(reply, 200, { message: "logged out" } as MessageResponse);
    },

    // me: profile of the authenticated identity.
    me: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const user = await uc.me(actorID(req).userId);
      sendJSON(reply, 200, meResponse(user));
    },

    // updateMe: partial profile update (reference PUT /account/me parity); the
    // absent fields are left unchanged per the use-case contract.
    updateMe: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const b = decode(updateMeBody, req.body);
      const inp: UpdateMeInput = {
        firstName: b.first_name,
        lastName: b.last_name,
        phone: b.phone_number,
        picture: b.profile_picture,
        language: b.language,
        address: b.address,
        postal: b.postal_code,
      };
      const user = await uc.updateMe(actorID(req).userId, inp);
      sendJSON(reply, 200, meResponse(user));
    },

    // sessions lists the caller's sessions, newest activity first.
    sessions: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const activeOnly = queryBool(req, "active_only", true);
      const rows = await uc.sessions(actorID(req).userId, activeOnly);
      sendJSON(reply, 200, rows.map(sessionItemOf));
    },

    // currentSession: the caller's most recent active session; no active
    // session renders null (reference parity: the session_id hint param is
    // unused there too).
    currentSession: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const row = await uc.currentSession(actorID(req).userId);
      if (row === null) {
        sendJSON(reply, 200, null);
        return;
      }
      sendJSON(reply, 200, sessionItemOf(row));
    },

    // revokeCurrent revokes the caller's current session: the access token's
    // jti is the session id (see mintFor), so no body is needed.
    revokeCurrent: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const revoked = await uc.revokeCurrent(actorID(req).userId, actorID(req).jti);
      sendJSON(reply, 200, { revoked });
    },

    // revokeAll deactivates every active session of the caller.
    revokeAll: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      sendJSON(reply, 200, { revoked_count: await uc.revokeAll(actorID(req).userId) });
    },

    // adminSessions lists one target user's sessions.
    adminSessions: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const activeOnly = queryBool(req, "active_only", true);
      const target = (req.params as { id: string }).id;
      const rows = await uc.sessions(target, activeOnly);
      sendJSON(reply, 200, rows.map(sessionItemOf));
    },

    // adminRevokeAll revokes every active session of one target user.
    adminRevokeAll: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const target = (req.params as { id: string }).id;
      sendJSON(reply, 200, { revoked_count: await uc.revokeAll(target) });
    },

    // anomalies renders the admin session report (?window_minutes=, default 60).
    anomalies: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const rows = await uc.anomalies(anomalyWindowMs(req));
      sendJSON(reply, 200, rows.map(anomalyItemOf));
    },
  };
};

// anomalyWindowMs parses ?window_minutes (minutes), defaulting to 60; out of
// range → 422 naming the accepted span.
const anomalyWindowMs = (req: FastifyRequest): number => {
  const raw = (req.query as Record<string, unknown>)?.["window_minutes"];
  const windowMs = 60 * 60_000;
  if (raw === undefined || raw === "") {
    return windowMs;
  }
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0 || n * 60_000 > UseCase.maxAnomalyWindowMs) {
    throw E(CodeUnprocessable, Validation, "window_minutes must be 1..43200");
  }
  return n * 60_000;
};
