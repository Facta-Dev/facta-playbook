// Features authsession postgres adapts the Kysely database to this slice's
// Store port (the sqlc adapter's analog). Login reads keep sqlc zero-value
// parity for columns the login view does not select: epoch timestamps and ""
// names — the flows never surface them.

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import type { UserRow, SessionRow, SessionUser, MultiIPRow, UpdateMeInput } from "./domain.js";
import type { Store } from "./ports.js";
import {
  sessionLoginGetUserByEmail,
  sessionUserByID,
  sessionAfterFailedLogin,
  sessionAfterSuccessfulLogin,
  sessionCreate,
  sessionGetActiveByToken,
  sessionDeactivate,
  sessionRotateTokens,
  sessionDeactivateByTokenForUser,
  sessionListByUser,
  sessionCurrentByUser,
  sessionRevokeForUser,
  sessionRevokeAllForUser,
  sessionCountExpiredActive,
  sessionAnomalyMultiIP,
  sessionUpdateMe,
  type Database,
  type SessionLoginRow,
  type SessionMeRow,
  type SessionSel,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async getUserByEmail(email: string): Promise<UserRow | null> {
    const r = await sessionLoginGetUserByEmail(this.#db, email);
    return r === null ? null : rowToLoginUser(r);
  }

  async getUserByID(id: string): Promise<UserRow | null> {
    const r = await sessionUserByID(this.#db, id);
    return r === null ? null : rowToMeUser(r);
  }

  // updateMe is update-then-read (reference parity): 0 rows = unknown user.
  async updateMe(userId: string, inp: UpdateMeInput): Promise<UserRow | null> {
    const rows = await sessionUpdateMe(this.#db, { userId, inp });
    if (rows === 0) {
      return null;
    }
    const r = await sessionUserByID(this.#db, userId);
    return r === null ? null : rowToMeUser(r);
  }

  async afterFailedLogin(userId: string, lockAt: number): Promise<boolean> {
    return sessionAfterFailedLogin(this.#db, { id: userId, lockAt });
  }

  async afterSuccessfulLogin(userId: string): Promise<void> {
    await sessionAfterSuccessfulLogin(this.#db, userId);
  }

  async createSession(s: SessionRow): Promise<void> {
    await sessionCreate(this.#db, {
      id: s.id,
      userId: s.userId,
      refreshToken: s.refreshToken,
      ipAddress: s.ipAddress,
      userAgent: s.userAgent,
      expiresAt: s.expiresAt,
    });
  }

  async activeSessionByToken(token: string): Promise<SessionUser | null> {
    const r = await sessionGetActiveByToken(this.#db, token);
    if (r === null) {
      return null;
    }
    return {
      id: r.session_id,
      userId: r.user_id,
      refreshToken: r.refresh_token,
      expiresAt: r.expires_at,
      isActive: r.is_active,
      ipAddress: "",
      userAgent: "",
      // rotation lookup selects no view columns; coalesced with now() at wire.
      createdAt: new Date(0),
      lastActivity: new Date(0),
      email: r.email,
      role: r.role,
      userActive: r.user_is_active,
      isBlocked: r.is_blocked,
      isVerified: r.is_verified,
    };
  }

  async deactivateSession(sessionId: string): Promise<boolean> {
    return (await sessionDeactivate(this.#db, sessionId)) > 0;
  }

  async rotateSession(sessionId: string, oldToken: string, s: SessionRow): Promise<boolean> {
    return (
      (await sessionRotateTokens(this.#db, {
        refreshToken: s.refreshToken,
        ipAddress: s.ipAddress,
        userAgent: s.userAgent,
        id: sessionId,
        oldToken,
      })) > 0
    );
  }

  async deactivateByTokenForUser(userId: string, token: string): Promise<void> {
    await sessionDeactivateByTokenForUser(this.#db, { refreshToken: token, userId });
  }

  async listSessionsByUser(userId: string, activeOnly: boolean): Promise<SessionRow[]> {
    const rs = await sessionListByUser(this.#db, { userId, activeOnly });
    return rs.map(rowToSession);
  }

  async currentSession(userId: string): Promise<SessionRow | null> {
    const r = await sessionCurrentByUser(this.#db, userId);
    return r === null ? null : rowToSession(r);
  }

  async revokeSessionForUser(userId: string, sessionId: string): Promise<boolean> {
    return (await sessionRevokeForUser(this.#db, { userId, sessionId })) > 0;
  }

  async revokeAllForUser(userId: string): Promise<number> {
    return sessionRevokeAllForUser(this.#db, userId);
  }

  async countExpiredActive(): Promise<number> {
    return sessionCountExpiredActive(this.#db);
  }

  async multiIPSessions(since: Date): Promise<MultiIPRow[]> {
    return (await sessionAnomalyMultiIP(this.#db, since)).map((r) => ({
      userId: r.user_id,
      ipCount: r.ip_count,
    }));
  }
}

// rowToLoginUser maps the login view: password present, profile/audit absent
// (sqlc zero-value parity — nothing here reaches the wire).
const rowToLoginUser = (r: SessionLoginRow): UserRow => ({
  id: r.id,
  email: r.email,
  firstName: "",
  lastName: "",
  passwordHash: r.password,
  role: r.role,
  userType: "",
  language: r.language,
  phone: null,
  picture: null,
  address: null,
  postal: null,
  subscription: null,
  isActive: r.is_active,
  isBlocked: r.is_blocked,
  isVerified: r.is_verified,
  retryCount: r.retry_count,
  lastLogin: null,
  createdAt: new Date(0),
  updatedAt: new Date(0),
  googleLinked: r.google_id !== null,
  githubLinked: r.github_id !== null,
});

// rowToMeUser maps the me view (profile + audit columns; never selects the
// password hash).
const rowToMeUser = (r: SessionMeRow): UserRow => ({
  id: r.id,
  email: r.email,
  firstName: r.first_name,
  lastName: r.last_name,
  passwordHash: "",
  role: r.role,
  userType: r.user_type,
  language: r.language,
  phone: r.phone_number,
  picture: r.profile_picture,
  address: r.address,
  postal: r.postal_code,
  subscription: r.subscription,
  isActive: r.is_active,
  isBlocked: r.is_blocked,
  isVerified: r.is_verified,
  retryCount: r.retry_count,
  lastLogin: r.last_login,
  createdAt: r.created_at,
  updatedAt: r.updated_at,
  googleLinked: r.google_linked,
  githubLinked: r.github_linked,
});

// rowToSession maps one sessions select-row (ip/ua may be NULL; "" reads as
// dropped at the wire via omitempty parity).
const rowToSession = (r: SessionSel): SessionRow => ({
  id: r.id,
  userId: r.user_id,
  refreshToken: r.refresh_token,
  ipAddress: r.ip_address ?? "",
  userAgent: r.user_agent ?? "",
  expiresAt: r.expires_at,
  isActive: r.is_active,
  createdAt: r.created_at,
  lastActivity: r.last_activity,
});
