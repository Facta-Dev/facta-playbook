// Platform authx is the authentication kernel: identity on the request, the
// HS256 JWT verifier/minter (jose), password hashing comes with the identity
// slices, and the RequireAuth/RequireRole preHandlers. Port of the Go
// template's authx package.

import { SignJWT, jwtVerify } from "jose";
import crypto from "node:crypto";
import bcrypt from "bcryptjs";
import type { FastifyReply, FastifyRequest, preHandlerHookHandler } from "fastify";
import {
  E,
  CodeForbidden,
  CodeUnauthorized,
  AuthInvalidToken,
  AuthForbidden,
  AuthUnauthorized,
} from "#shared/apperr.js";
import { atLeast, type Role } from "#shared/rolex.js";

// Identity is the authenticated actor, extracted from verified claims.
export interface Identity {
  userId: string;
  email: string;
  role: Role;
  jti: string;
}

declare module "fastify" {
  interface FastifyRequest {
    identity?: Identity;
  }
}

interface AccessClaims {
  sub?: string;
  email?: string;
  role?: string;
  jti?: string;
}

export interface Verifier {
  parse(raw: string): Promise<Identity>;
}

export interface Minter {
  mint(id: Identity, ttlMs: number): Promise<{ token: string; exp: Date }>;
}

// newHS256Verifier builds the verifier; issuer tags claims for hygiene. Only
// HS256 is accepted, and expiry/issuer are checked by jose.
export function newHS256Verifier(secret: string, issuer: string): Verifier {
  const key = new TextEncoder().encode(secret);
  return {
    async parse(raw: string): Promise<Identity> {
      let payload;
      try {
        ({ payload } = await jwtVerify(raw, key, { issuer, algorithms: ["HS256"] }));
      } catch {
        throw E(CodeUnauthorized, AuthInvalidToken, "invalid or expired token");
      }
      const c = payload as AccessClaims;
      return {
        userId: c.sub ?? "",
        email: c.email ?? "",
        role: (c.role ?? "") as Role,
        jti: c.jti ?? "",
      };
    },
  };
}

// newHS256Minter signs HS256 JWTs with the shared secret; issuer tags tokens
// with "<app>" (rename-substituted). Mint signs {sub, email, role, jti} plus
// std exp/iat/iss; aud stays unset (single-audience API; verifier accepts any
// aud).
export function newHS256Minter(secret: string, issuer: string): Minter {
  const key = new TextEncoder().encode(secret);
  return {
    async mint(id: Identity, ttlMs: number): Promise<{ token: string; exp: Date }> {
      const exp = new Date(Date.now() + ttlMs);
      const token = await new SignJWT({ email: id.email, role: id.role, jti: id.jti })
        .setProtectedHeader({ alg: "HS256" })
        .setSubject(id.userId)
        .setIssuer(issuer)
        .setIssuedAt()
        .setExpirationTime(Math.floor(exp.getTime() / 1000))
        .sign(key);
      return { token, exp };
    },
  };
}

// bearerToken extracts the raw token: `Authorization: Bearer` header first,
// then the configured cookie when cookieName != "". WS/SSE handlers pass
// cookieName="" and read the `token` query param themselves. In cookie mode
// the name is "<app>_access_token" (rename-substituted).
export const bearerToken = (req: FastifyRequest, cookieName: string): string | undefined => {
  const h = req.headers["authorization"];
  if (typeof h === "string" && h.startsWith("Bearer ")) {
    return h.slice("Bearer ".length).trim();
  }
  if (cookieName !== "") {
    const v = req.cookies?.[cookieName];
    if (typeof v === "string" && v !== "") return v;
  }
  return undefined;
};

// requireAuth verifies the token and stores the Identity on the request;
// otherwise it raises 401 (the app error handler answers the envelope).
// Protected-mount only: /healthz and /readyz are mounted outside it.
export const requireAuth =
  (v: Verifier, cookieName: string): preHandlerHookHandler =>
  async (req) => {
    const raw = bearerToken(req, cookieName);
    if (raw === undefined || raw === "") {
      throw E(CodeUnauthorized, AuthUnauthorized, "authentication required");
    }
    req.identity = await v.parse(raw);
  };

// requireRole authorizes by rank (rolex.atLeast) on the identity stored by
// requireAuth; otherwise it raises 403 AUTH_FORBIDDEN.
export const requireRole =
  (min: Role): preHandlerHookHandler =>
  async (req) => {
    const id = req.identity;
    if (id === undefined || !atLeast(id.role, min)) {
      throw E(CodeForbidden, AuthForbidden, "insufficient role");
    }
  };

// setAuthCookie issues the cookie-mode access-token cookie: HttpOnly,
// SameSite=Strict, Path=/ and Secure everywhere except the local dev env
// (see <app>_AUTH_COOKIE in .env.example). Bearer mode is always accepted.
export const setAuthCookie = (
  reply: FastifyReply,
  name: string,
  token: string,
  secure: boolean,
): void => {
  void reply.setCookie(name, token, {
    path: "/",
    httpOnly: true,
    secure,
    sameSite: "strict",
  });
};

// clearAuthCookie expires the cookie-mode access-token cookie on logout.
export const clearAuthCookie = (reply: FastifyReply, name: string, secure: boolean): void => {
  void reply.clearCookie(name, {
    path: "/",
    httpOnly: true,
    secure,
    sameSite: "strict",
  });
};

// ---------------------------------------------------------------------------
// Passwords: bcrypt (cost 10) over a peppered HMAC-SHA256 pre-hash. Port of
// the Go template's authx/password.go.
// ---------------------------------------------------------------------------

// bcryptCost mirrors ctx0 parity (gensalt 10).
const bcryptCost = 10;

// preHash conditions the password before bcrypt. With a pepper: HMAC-SHA256
// keyed by the pepper, base64-encoded — this defeats bcrypt's 72-byte password
// limit and any NUL truncation, and keeps the pepper secret even from a
// stolen users table. Without a pepper the raw password feeds bcrypt directly
// (documented ceiling: bcrypt silently truncates past 72 bytes).
const preHash = (password: string, pepper: string): string => {
  if (pepper === "") {
    return password;
  }
  return crypto.createHmac("sha256", pepper).update(password).digest("base64");
};

// hashPassword builds the stored bcrypt hash (cost 10) for a password.
export const hashPassword = (password: string, pepper: string): Promise<string> =>
  bcrypt.hash(preHash(password, pepper), bcryptCost);

// checkPassword reports whether the password matches the stored hash; a
// mismatch throws for the use-case to map.
export const checkPassword = async (
  hash: string,
  password: string,
  pepper: string,
): Promise<true> => {
  const ok = await bcrypt.compare(preHash(password, pepper), hash);
  if (!ok) {
    throw new Error("password mismatch");
  }
  return true;
};
